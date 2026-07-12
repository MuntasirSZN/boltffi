use super::*;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HandleMethod {
    name: Identifier,
    parameters: Vec<Parameter<ValueType>>,
    returns: ReturnType,
    body: Vec<Statement>,
    wire_runtime: bool,
    direct_vector_runtime: bool,
}

struct HandleParameter {
    public: Parameter<ValueType>,
    acquire: Vec<Statement>,
    prepare: Vec<Statement>,
    arguments: Vec<Expression>,
    cleanup: Vec<Statement>,
    wire_runtime: bool,
    direct_vector_runtime: bool,
}

struct HandleParameterRender<'context> {
    source: Name,
    name: Identifier,
    version: JavaVersion,
    context: &'context RenderContext<'context, Native>,
}

enum HandleReturnConversion {
    Void,
    Direct,
    DirectRecord(TypeName),
    DirectEnum(TypeName),
    Encoded(boltffi_binding::ReadPlan),
    ScalarOption(Primitive),
    DirectVector(DirectVector),
    ClassHandle(ClassHandle),
    CallbackHandle(CallbackHandle),
}

struct HandleReturn {
    public: ReturnType,
    conversion: HandleReturnConversion,
    wire_runtime: bool,
    direct_vector_runtime: bool,
}

struct HandleReturnRender<'context> {
    version: JavaVersion,
    context: &'context RenderContext<'context, Native>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CallbackHandle {
    ty: TypeName,
    bridge: TypeName,
    carrier: Primitive,
    presence: HandlePresence,
}

impl HandleMethod {
    pub fn from_declaration(
        source: &ImportedMethodDecl<Native, VTableSlot>,
        method: &JniCallbackHandleMethod,
        version: JavaVersion,
        context: &RenderContext<Native>,
    ) -> Result<Self> {
        if source.target().as_str() != method.slot().as_str() {
            return Err(JavaHost::broken_bridge_contract(
                "callback handle method matches the callback declaration",
            ));
        }
        if !matches!(source.callable().execution(), ExecutionDecl::Synchronous(_)) {
            return Err(JavaHost::unsupported("asynchronous callback handle method"));
        }
        if !matches!(source.callable().error().channel(), ErrorChannel::None) {
            return Err(JavaHost::unsupported("fallible callback handle method"));
        }
        let parameters = source
            .callable()
            .params()
            .iter()
            .map(|parameter| HandleParameter::from_declaration(parameter, version, context))
            .collect::<Result<Vec<_>>>()?;
        let returned = source
            .callable()
            .returns()
            .plan()
            .render_with(&mut HandleReturnRender { version, context })?;
        let native = NativeMethod::from_callback_handle_method(method, version)?;
        let call = native.call(
            &TypeIdentifier::known("Native", version),
            std::iter::once(Expression::this().member(Identifier::known("handle"))).chain(
                parameters
                    .iter()
                    .flat_map(|parameter| parameter.arguments.iter().cloned()),
            ),
        )?;
        let protected = parameters
            .iter()
            .flat_map(|parameter| parameter.prepare.iter().cloned())
            .chain(returned.statements(call, version, context)?)
            .collect::<Vec<_>>();
        let cleanup = parameters
            .iter()
            .flat_map(|parameter| parameter.cleanup.iter().cloned())
            .collect::<Vec<_>>();
        let protected = match cleanup.is_empty() {
            true => protected,
            false => vec![Statement::try_finally(protected, cleanup)],
        };
        Ok(Self {
            name: Name::new(source.name()).function(version)?,
            parameters: parameters
                .iter()
                .map(|parameter| parameter.public.clone())
                .collect(),
            returns: returned.public.clone(),
            body: parameters
                .iter()
                .flat_map(|parameter| parameter.acquire.iter().cloned())
                .chain(protected)
                .collect(),
            wire_runtime: returned.wire_runtime
                || parameters.iter().any(|parameter| parameter.wire_runtime),
            direct_vector_runtime: returned.direct_vector_runtime
                || parameters
                    .iter()
                    .any(|parameter| parameter.direct_vector_runtime),
        })
    }

    pub fn name(&self) -> &Identifier {
        &self.name
    }

    pub fn parameters(&self) -> &[Parameter<ValueType>] {
        &self.parameters
    }

    pub fn returns(&self) -> &ReturnType {
        &self.returns
    }

    pub fn body(&self) -> &[Statement] {
        &self.body
    }

    pub fn requires_wire_runtime(&self) -> bool {
        self.wire_runtime
    }

    pub fn requires_direct_vector_runtime(&self) -> bool {
        self.direct_vector_runtime
    }
}

impl HandleParameter {
    fn from_declaration(
        parameter: &ParamDecl<Native, OutOfRust>,
        version: JavaVersion,
        context: &RenderContext<Native>,
    ) -> Result<Self> {
        let source = Name::new(parameter.name());
        let name = source.parameter(version)?;
        let plan = parameter
            .payload()
            .as_value()
            .ok_or(JavaHost::unsupported("callback handle closure parameter"))?;
        plan.render_with(&mut HandleParameterRender {
            source,
            name,
            version,
            context,
        })
    }

    fn direct(public: Parameter<ValueType>, argument: Expression) -> Self {
        Self {
            public,
            acquire: Vec::new(),
            prepare: Vec::new(),
            arguments: vec![argument],
            cleanup: Vec::new(),
            wire_runtime: false,
            direct_vector_runtime: false,
        }
    }

    fn encoded(
        public: Parameter<ValueType>,
        write: crate::target::java::codec::EncodedWrite,
    ) -> Self {
        let (acquire, prepare, arguments, cleanup) = write.into_parts();
        Self {
            public,
            acquire,
            prepare,
            arguments,
            cleanup,
            wire_runtime: true,
            direct_vector_runtime: false,
        }
    }
}

impl<'plan> ParamPlanRender<'plan, Native, OutOfRust> for HandleParameterRender<'_> {
    type Output = Result<HandleParameter>;

    fn direct(
        &mut self,
        ty: &'plan DirectValueType,
        _: <OutOfRust as Direction>::Receive,
    ) -> Self::Output {
        let value = Expression::identifier(self.name.clone());
        match ty {
            DirectValueType::Primitive(primitive) => Ok(HandleParameter::direct(
                Parameter::new(
                    self.name.clone(),
                    ValueType::Primitive(Primitive::try_from(*primitive)?),
                ),
                value,
            )),
            DirectValueType::Record(record) => {
                let record = Record::type_name_for(*record, self.context, self.version)?;
                Ok(HandleParameter::direct(
                    Parameter::new(
                        self.name.clone(),
                        ValueType::Reference(TypeName::named(record)),
                    ),
                    value.call(Identifier::known("toDirectBuffer"), ArgumentList::default()),
                ))
            }
            DirectValueType::Enum(enumeration) => {
                let enumeration =
                    Enumeration::type_name_for(*enumeration, self.context, self.version)?;
                Ok(HandleParameter::direct(
                    Parameter::new(
                        self.name.clone(),
                        ValueType::Reference(TypeName::named(enumeration)),
                    ),
                    value.call(Identifier::known("nativeValue"), ArgumentList::default()),
                ))
            }
            _ => Err(JavaHost::unsupported("callback handle direct parameter")),
        }
    }

    fn encoded(
        &mut self,
        ty: &'plan TypeRef,
        codec: &'plan <OutOfRust as Direction>::Codec,
        _: native::BufferShape,
        _: <OutOfRust as Direction>::Receive,
    ) -> Self::Output {
        let plan = codec.write_self_value();
        let write = WireBuffer::new(&self.source, self.version)?.write(
            &plan,
            Expression::identifier(self.name.clone()),
            self.context,
        )?;
        Ok(HandleParameter::encoded(
            Parameter::new(
                self.name.clone(),
                ValueType::Reference(JavaType::type_ref(ty, self.version, self.context)?),
            ),
            write,
        ))
    }

    fn handle(
        &mut self,
        target: &'plan HandleTarget,
        carrier: native::HandleCarrier,
        presence: HandlePresence,
        _: <OutOfRust as Direction>::Receive,
    ) -> Self::Output {
        let value = Expression::identifier(self.name.clone());
        match target {
            HandleTarget::Class(class) => {
                let handle =
                    ClassHandle::new(*class, carrier, presence, self.version, self.context, None)?;
                Ok(HandleParameter::direct(
                    Parameter::new(self.name.clone(), ValueType::Reference(handle.ty().clone())),
                    handle.native_argument(value)?,
                ))
            }
            HandleTarget::Callback(callback) => {
                let handle = CallbackHandle::new(
                    *callback,
                    carrier,
                    presence,
                    self.version,
                    self.context,
                    None,
                )?;
                Ok(HandleParameter::direct(
                    Parameter::new(self.name.clone(), ValueType::Reference(handle.ty().clone())),
                    handle.native_argument(value)?,
                ))
            }
            _ => Err(JavaHost::unsupported("callback handle parameter")),
        }
    }

    fn scalar_option(&mut self, primitive: BindingPrimitive) -> Self::Output {
        let primitive = Primitive::try_from(primitive)?;
        let value = Expression::identifier(self.name.clone());
        let writer = self.source.generated("writer", self.version)?;
        let payload = self.source.generated("value", self.version)?;
        let write = WireBuffer::new(&self.source, self.version)?.write_statements(
            Expression::integer(1).add(
                value
                    .clone()
                    .call(Identifier::known("isPresent"), ArgumentList::default())
                    .conditional(
                        Expression::integer(primitive.wire_size()),
                        Expression::integer(0),
                    ),
            ),
            vec![Statement::expression(
                Expression::identifier(writer.clone()).call(
                    Identifier::known("writeOptional"),
                    [
                        value,
                        Expression::lambda_statement(
                            [payload.clone()],
                            Statement::expression(Expression::identifier(writer).call(
                                Identifier::parse_for(
                                    format!("write{}", primitive.wire_method_suffix()),
                                    self.version,
                                )?,
                                [Expression::identifier(payload)].into_iter().collect(),
                            )),
                        ),
                    ]
                    .into_iter()
                    .collect(),
                ),
            )],
        )?;
        Ok(HandleParameter::encoded(
            Parameter::new(
                self.name.clone(),
                ValueType::Reference(JavaType::optional_primitive(primitive, self.version)),
            ),
            write,
        ))
    }

    fn direct_vector(
        &mut self,
        element: &'plan boltffi_binding::DirectVectorElementType,
    ) -> Self::Output {
        let vector = DirectVector::from_element(element, self.version, self.context)?;
        Ok(HandleParameter {
            public: Parameter::new(self.name.clone(), ValueType::Reference(vector.ty().clone())),
            acquire: Vec::new(),
            prepare: Vec::new(),
            arguments: vec![vector.native_argument(Expression::identifier(self.name.clone()))],
            cleanup: Vec::new(),
            wire_runtime: false,
            direct_vector_runtime: true,
        })
    }
}

impl<'plan> ReturnPlanRender<'plan, Native, IntoRust> for HandleReturnRender<'_> {
    type Output = Result<HandleReturn>;

    fn void(&mut self) -> Self::Output {
        Ok(HandleReturn {
            public: ReturnType::Void,
            conversion: HandleReturnConversion::Void,
            wire_runtime: false,
            direct_vector_runtime: false,
        })
    }

    fn direct(&mut self, slot: ReturnValueSlot, ty: &'plan DirectValueType) -> Self::Output {
        if slot != ReturnValueSlot::ReturnSlot {
            return Err(JavaHost::unsupported("callback handle out-pointer return"));
        }
        match ty {
            DirectValueType::Primitive(primitive) => Ok(HandleReturn {
                public: ReturnType::Value(ValueType::Primitive(Primitive::try_from(*primitive)?)),
                conversion: HandleReturnConversion::Direct,
                wire_runtime: false,
                direct_vector_runtime: false,
            }),
            DirectValueType::Record(record) => {
                let record = Record::type_name_for(*record, self.context, self.version)?;
                Ok(HandleReturn {
                    public: ReturnType::Value(ValueType::Reference(TypeName::named(
                        record.clone(),
                    ))),
                    conversion: HandleReturnConversion::DirectRecord(TypeName::named(record)),
                    wire_runtime: false,
                    direct_vector_runtime: false,
                })
            }
            DirectValueType::Enum(enumeration) => {
                let enumeration =
                    Enumeration::type_name_for(*enumeration, self.context, self.version)?;
                Ok(HandleReturn {
                    public: ReturnType::Value(ValueType::Reference(TypeName::named(
                        enumeration.clone(),
                    ))),
                    conversion: HandleReturnConversion::DirectEnum(TypeName::named(enumeration)),
                    wire_runtime: false,
                    direct_vector_runtime: false,
                })
            }
            _ => Err(JavaHost::unsupported("callback handle direct return")),
        }
    }

    fn encoded(
        &mut self,
        slot: ReturnValueSlot,
        ty: &'plan TypeRef,
        codec: &'plan <IntoRust as Direction>::Codec,
        _: native::BufferShape,
    ) -> Self::Output {
        if slot != ReturnValueSlot::ReturnSlot {
            return Err(JavaHost::unsupported(
                "callback handle encoded out-pointer return",
            ));
        }
        Ok(HandleReturn {
            public: ReturnType::Value(ValueType::Reference(JavaType::type_ref(
                ty,
                self.version,
                self.context,
            )?)),
            conversion: HandleReturnConversion::Encoded(codec.read_plan()),
            wire_runtime: true,
            direct_vector_runtime: false,
        })
    }

    fn handle(
        &mut self,
        slot: ReturnValueSlot,
        target: &'plan HandleTarget,
        carrier: native::HandleCarrier,
        presence: HandlePresence,
    ) -> Self::Output {
        if slot != ReturnValueSlot::ReturnSlot {
            return Err(JavaHost::unsupported(
                "callback handle value out-pointer return",
            ));
        }
        match target {
            HandleTarget::Class(class) => {
                let handle =
                    ClassHandle::new(*class, carrier, presence, self.version, self.context, None)?;
                Ok(HandleReturn {
                    public: ReturnType::Value(ValueType::Reference(handle.ty().clone())),
                    conversion: HandleReturnConversion::ClassHandle(handle),
                    wire_runtime: false,
                    direct_vector_runtime: false,
                })
            }
            HandleTarget::Callback(callback) => {
                let handle = CallbackHandle::new(
                    *callback,
                    carrier,
                    presence,
                    self.version,
                    self.context,
                    None,
                )?;
                Ok(HandleReturn {
                    public: ReturnType::Value(ValueType::Reference(handle.ty().clone())),
                    conversion: HandleReturnConversion::CallbackHandle(handle),
                    wire_runtime: false,
                    direct_vector_runtime: false,
                })
            }
            _ => Err(JavaHost::unsupported("callback handle value return")),
        }
    }

    fn scalar_option(&mut self, primitive: BindingPrimitive) -> Self::Output {
        let primitive = Primitive::try_from(primitive)?;
        Ok(HandleReturn {
            public: ReturnType::Value(ValueType::Reference(JavaType::optional_primitive(
                primitive,
                self.version,
            ))),
            conversion: HandleReturnConversion::ScalarOption(primitive),
            wire_runtime: true,
            direct_vector_runtime: false,
        })
    }

    fn direct_vector(
        &mut self,
        element: &'plan boltffi_binding::DirectVectorElementType,
    ) -> Self::Output {
        let vector = DirectVector::from_element(element, self.version, self.context)?;
        Ok(HandleReturn {
            public: ReturnType::Value(ValueType::Reference(vector.ty().clone())),
            conversion: HandleReturnConversion::DirectVector(vector),
            wire_runtime: false,
            direct_vector_runtime: true,
        })
    }

    fn closure(
        &mut self,
        _: &'plan boltffi_binding::ClosureReturn<Native, IntoRust>,
    ) -> Self::Output {
        Err(JavaHost::unsupported("callback handle closure return"))
    }
}

impl HandleReturn {
    fn statements(
        &self,
        call: Expression,
        version: JavaVersion,
        context: &RenderContext<Native>,
    ) -> Result<Vec<Statement>> {
        match &self.conversion {
            HandleReturnConversion::Void => Ok(vec![Statement::expression(call)]),
            HandleReturnConversion::Direct => Ok(vec![Statement::return_value(call)]),
            HandleReturnConversion::DirectRecord(record) => {
                Ok(vec![Statement::return_value(Expression::static_call(
                    record.clone(),
                    Identifier::known("fromByteArray"),
                    [call].into_iter().collect(),
                ))])
            }
            HandleReturnConversion::DirectEnum(enumeration) => {
                Ok(vec![Statement::return_value(Expression::static_call(
                    enumeration.clone(),
                    Identifier::known("fromValue"),
                    [call].into_iter().collect(),
                ))])
            }
            HandleReturnConversion::Encoded(codec) => {
                let bytes = Identifier::known("__boltffi_result");
                let reader = Identifier::known("__boltffi_reader");
                let decoded = codec
                    .render_with(&mut Reader::new(reader.clone(), version, context))?
                    .into_expression();
                Ok(vec![
                    Statement::value(
                        TypeName::array(TypeName::primitive(Primitive::Byte)),
                        bytes.clone(),
                        call,
                    ),
                    Statement::value(
                        TypeName::named(TypeIdentifier::known("WireReader", version)),
                        reader,
                        Expression::construct(
                            TypeName::named(TypeIdentifier::known("WireReader", version)),
                            [Expression::identifier(bytes)].into_iter().collect(),
                        ),
                    ),
                    Statement::return_value(decoded),
                ])
            }
            HandleReturnConversion::ScalarOption(primitive) => {
                let bytes = Identifier::known("__boltffi_result");
                let reader = Identifier::known("__boltffi_reader");
                let reader_value = Expression::identifier(reader.clone());
                Ok(vec![
                    Statement::value(
                        TypeName::array(TypeName::primitive(Primitive::Byte)),
                        bytes.clone(),
                        call,
                    ),
                    Statement::value(
                        TypeName::named(TypeIdentifier::known("WireReader", version)),
                        reader,
                        Expression::construct(
                            TypeName::named(TypeIdentifier::known("WireReader", version)),
                            [Expression::identifier(bytes)].into_iter().collect(),
                        ),
                    ),
                    Statement::return_value(
                        reader_value.clone().call(
                            Identifier::known("readOptional"),
                            [Expression::lambda(
                                [],
                                reader_value.call(
                                    Identifier::parse_for(
                                        format!("read{}", primitive.wire_method_suffix()),
                                        version,
                                    )?,
                                    ArgumentList::default(),
                                ),
                            )]
                            .into_iter()
                            .collect(),
                        ),
                    ),
                ])
            }
            HandleReturnConversion::DirectVector(vector) => {
                let bytes = Identifier::known("__boltffi_result");
                Ok(vec![
                    Statement::value(
                        TypeName::array(TypeName::primitive(Primitive::Byte)),
                        bytes.clone(),
                        call,
                    ),
                    Statement::return_value(
                        vector.returned_expression(Expression::identifier(bytes)),
                    ),
                ])
            }
            HandleReturnConversion::ClassHandle(handle) => handle.value_statements(call),
            HandleReturnConversion::CallbackHandle(handle) => handle.value_statements(call),
        }
    }
}

impl CallbackHandle {
    pub fn new(
        id: CallbackId,
        carrier: native::HandleCarrier,
        presence: HandlePresence,
        version: JavaVersion,
        context: &RenderContext<Native>,
        package: Option<&JavaPackage>,
    ) -> Result<Self> {
        let name = Callback::type_name_for(id, context, version)?;
        let ty = match package {
            Some(package) => package.type_name(name.clone()),
            None => TypeName::named(name.clone()),
        };
        let bridge_name = TypeIdentifier::parse(format!("{name}Bridge"), version)?;
        let bridge = match package {
            Some(package) => package.type_name(bridge_name),
            None => TypeName::named(bridge_name),
        };
        Ok(Self {
            ty,
            bridge,
            carrier: Primitive::from_handle_carrier(carrier)?,
            presence,
        })
    }

    pub fn ty(&self) -> &TypeName {
        &self.ty
    }

    pub const fn carrier(&self) -> Primitive {
        self.carrier
    }

    pub fn native_argument(&self, value: Expression) -> Result<Expression> {
        let create = Identifier::known("create");
        let required = Expression::static_call(
            self.bridge.clone(),
            create.clone(),
            [value.clone()].into_iter().collect(),
        );
        match self.presence {
            HandlePresence::Required => Ok(required),
            HandlePresence::Nullable => Ok(value.clone().equal(Expression::null()).conditional(
                Expression::long(0),
                Expression::static_call(self.bridge.clone(), create, [value].into_iter().collect()),
            )),
            _ => Err(JavaHost::unsupported("callback handle presence")),
        }
    }

    pub fn value_statements(&self, value: Expression) -> Result<Vec<Statement>> {
        let wrap = |value| {
            Expression::static_call(
                self.bridge.clone(),
                Identifier::known("wrap"),
                [value].into_iter().collect::<ArgumentList>(),
            )
        };
        match self.presence {
            HandlePresence::Required => Ok(vec![Statement::return_value(wrap(value))]),
            HandlePresence::Nullable => {
                let handle = Identifier::known("__boltffi_handle");
                Ok(vec![
                    Statement::value(TypeName::primitive(self.carrier), handle.clone(), value),
                    Statement::return_value(
                        Expression::identifier(handle.clone())
                            .equal(Expression::long(0))
                            .conditional(Expression::null(), wrap(Expression::identifier(handle))),
                    ),
                ])
            }
            _ => Err(JavaHost::unsupported("callback handle presence")),
        }
    }

    pub fn value_expression(&self, value: Expression) -> Result<Expression> {
        let wrapped = Expression::static_call(
            self.bridge.clone(),
            Identifier::known("wrap"),
            [value.clone()].into_iter().collect(),
        );
        match self.presence {
            HandlePresence::Required => Ok(wrapped),
            HandlePresence::Nullable => Ok(value
                .equal(Expression::long(0))
                .conditional(Expression::null(), wrapped)),
            _ => Err(JavaHost::unsupported("callback handle presence")),
        }
    }
}
