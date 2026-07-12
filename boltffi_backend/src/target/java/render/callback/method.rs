use super::*;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Method {
    name: Identifier,
    jvm_name: Identifier,
    public_parameters: Vec<Parameter<ValueType>>,
    jvm_parameters: Vec<Parameter<ValueType>>,
    public_return: ReturnType,
    jvm_return: ReturnType,
    setup: Vec<Statement>,
    body: Vec<Statement>,
    doc: Option<Javadoc>,
    wire_runtime: bool,
    direct_vector_runtime: bool,
}

struct InvocationParameter {
    public: Parameter<ValueType>,
    jvm: Parameter<ValueType>,
    setup: Vec<Statement>,
    argument: Expression,
    wire_runtime: bool,
    direct_vector_runtime: bool,
}

struct InvocationParameterRender<'context> {
    source: Name,
    name: Identifier,
    version: JavaVersion,
    context: &'context RenderContext<'context, Native>,
}

enum InvocationReturnConversion {
    Void,
    Direct,
    DirectRecord,
    DirectEnum,
    DirectVector(DirectVector),
    Encoded {
        source: Name,
        ty: TypeName,
        codec: WritePlan,
    },
    ScalarOption {
        source: Name,
        primitive: Primitive,
    },
    ClassHandle(ClassHandle),
    CallbackHandle(CallbackHandle),
}

struct InvocationReturn {
    public: ReturnType,
    jvm: ReturnType,
    conversion: InvocationReturnConversion,
    wire_runtime: bool,
    direct_vector_runtime: bool,
}

struct InvocationReturnRender<'context> {
    source: Name,
    version: JavaVersion,
    context: &'context RenderContext<'context, Native>,
    fallible: bool,
}

struct FallibleReturn {
    source: Name,
    error_ty: TypeRef,
    error_codec: WritePlan,
    success_out: Option<SuccessOutArgument>,
}

#[derive(Clone, Copy)]
enum SuccessOutOrder {
    First,
    Last,
}

impl Method {
    pub fn from_declaration(
        source: &ImportedMethodDecl<Native, VTableSlot>,
        method: &JniCallbackMethod,
        version: JavaVersion,
        context: &RenderContext<Native>,
    ) -> Result<Self> {
        if source.target().as_str() != method.method().as_str() {
            return Err(JavaHost::broken_bridge_contract(
                "callback method matches the JNI registration",
            ));
        }
        if !matches!(source.callable().execution(), ExecutionDecl::Synchronous(_)) {
            return Err(JavaHost::unsupported("asynchronous callback method"));
        }
        Self::build(
            source.callable(),
            Name::new(source.name()),
            Name::new(source.name()).function(version)?,
            Identifier::parse_for(method.method().as_str(), version)?,
            method.success_out(),
            SuccessOutOrder::First,
            source.meta().doc().map(Javadoc::new),
            version,
            context,
        )
    }

    pub fn from_closure(
        closure: &IrClosureParameter<Native, IntoRust>,
        registration: &ClosureRegistration,
        version: JavaVersion,
        context: &RenderContext<Native>,
    ) -> Result<Self> {
        if closure.signature() != registration.signature() {
            return Err(JavaHost::broken_bridge_contract(
                "closure signature matches the JNI registration",
            ));
        }
        if !matches!(closure.invoke().execution(), ExecutionDecl::Synchronous(_)) {
            return Err(JavaHost::unsupported("asynchronous closure invocation"));
        }
        Self::build(
            closure.invoke(),
            Name::new(&CanonicalName::single("closure")),
            Identifier::known("invoke"),
            Identifier::known("call"),
            registration.success_out(),
            SuccessOutOrder::Last,
            None,
            version,
            context,
        )
    }

    fn build(
        callable: &CallableDecl<Native, ForeignBody>,
        source: Name,
        name: Identifier,
        jvm_name: Identifier,
        success_out: Option<SuccessOutArgument>,
        success_out_order: SuccessOutOrder,
        doc: Option<Javadoc>,
        version: JavaVersion,
        context: &RenderContext<Native>,
    ) -> Result<Self> {
        let fallible =
            FallibleReturn::from_channel(source.clone(), callable.error().channel(), success_out)?;
        let parameters = callable
            .params()
            .iter()
            .map(|parameter| InvocationParameter::from_declaration(parameter, version, context))
            .collect::<Result<Vec<_>>>()?;
        let returned = callable
            .returns()
            .plan()
            .render_with(&mut InvocationReturnRender {
                source,
                version,
                context,
                fallible: fallible.is_some(),
            })?;
        let implementation = Expression::identifier(Identifier::known("implementation"));
        let call = implementation.call(
            name.clone(),
            parameters
                .iter()
                .map(|parameter| parameter.argument.clone())
                .collect(),
        );
        let success_out = fallible
            .as_ref()
            .and_then(|fallible| fallible.success_out.as_ref())
            .map(|success_out| -> Result<_> {
                Ok(Parameter::new(
                    Identifier::parse_for(success_out.name().as_str(), version)?,
                    ValueType::Primitive(Primitive::from(success_out.jni_type())),
                ))
            })
            .transpose()?;
        let jvm_parameters = match success_out_order {
            SuccessOutOrder::First => success_out
                .into_iter()
                .chain(parameters.iter().map(|parameter| parameter.jvm.clone()))
                .collect(),
            SuccessOutOrder::Last => parameters
                .iter()
                .map(|parameter| parameter.jvm.clone())
                .chain(success_out)
                .collect(),
        };
        Ok(Self {
            name,
            jvm_name,
            public_parameters: parameters
                .iter()
                .map(|parameter| parameter.public.clone())
                .collect(),
            jvm_parameters,
            public_return: returned.public.clone(),
            jvm_return: match fallible {
                Some(_) => {
                    ReturnType::Value(ValueType::Reference(InvocationParameterRender::byte_array()))
                }
                None => returned.jvm.clone(),
            },
            setup: parameters
                .iter()
                .flat_map(|parameter| parameter.setup.iter().cloned())
                .collect(),
            body: match &fallible {
                Some(fallible) => returned.fallible_statements(call, fallible, version, context)?,
                None => returned.statements(call, version, context)?,
            },
            doc,
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

    pub fn jvm_name(&self) -> &Identifier {
        &self.jvm_name
    }

    pub fn public_parameters(&self) -> &[Parameter<ValueType>] {
        &self.public_parameters
    }

    pub fn jvm_parameters(&self) -> &[Parameter<ValueType>] {
        &self.jvm_parameters
    }

    pub fn public_return(&self) -> &ReturnType {
        &self.public_return
    }

    pub fn jvm_return(&self) -> &ReturnType {
        &self.jvm_return
    }

    pub fn setup(&self) -> &[Statement] {
        &self.setup
    }

    pub fn body(&self) -> &[Statement] {
        &self.body
    }

    pub fn doc(&self) -> Option<&Javadoc> {
        self.doc.as_ref()
    }

    pub fn requires_wire_runtime(&self) -> bool {
        self.wire_runtime
    }

    pub fn requires_direct_vector_runtime(&self) -> bool {
        self.direct_vector_runtime
    }
}

impl InvocationParameter {
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
            .ok_or(JavaHost::unsupported("callback closure parameter"))?;
        plan.render_with(&mut InvocationParameterRender {
            source,
            name,
            version,
            context,
        })
    }
}

impl<'plan> ParamPlanRender<'plan, Native, OutOfRust> for InvocationParameterRender<'_> {
    type Output = Result<InvocationParameter>;

    fn direct(
        &mut self,
        ty: &'plan DirectValueType,
        _: <OutOfRust as Direction>::Receive,
    ) -> Self::Output {
        let value = Expression::identifier(self.name.clone());
        match ty {
            DirectValueType::Primitive(primitive) => {
                let primitive = Primitive::try_from(*primitive)?;
                let parameter = Parameter::new(self.name.clone(), ValueType::Primitive(primitive));
                Ok(InvocationParameter {
                    public: parameter.clone(),
                    jvm: parameter,
                    setup: Vec::new(),
                    argument: value,
                    wire_runtime: false,
                    direct_vector_runtime: false,
                })
            }
            DirectValueType::Record(record) => {
                let record = Record::type_name_for(*record, self.context, self.version)?;
                Ok(InvocationParameter {
                    public: Parameter::new(
                        self.name.clone(),
                        ValueType::Reference(TypeName::named(record.clone())),
                    ),
                    jvm: Parameter::new(
                        self.name.clone(),
                        ValueType::Reference(Self::byte_array()),
                    ),
                    setup: Vec::new(),
                    argument: Expression::static_call(
                        TypeName::named(record),
                        Identifier::known("fromByteArray"),
                        [value].into_iter().collect(),
                    ),
                    wire_runtime: false,
                    direct_vector_runtime: false,
                })
            }
            DirectValueType::Enum(enumeration) => {
                let id = *enumeration;
                let enumeration = Enumeration::type_name_for(id, self.context, self.version)?;
                let carrier = Enumeration::c_style_primitive(id, self.context)?;
                Ok(InvocationParameter {
                    public: Parameter::new(
                        self.name.clone(),
                        ValueType::Reference(TypeName::named(enumeration.clone())),
                    ),
                    jvm: Parameter::new(self.name.clone(), ValueType::Primitive(carrier)),
                    setup: Vec::new(),
                    argument: Expression::static_call(
                        TypeName::named(enumeration),
                        Identifier::known("fromValue"),
                        [value].into_iter().collect(),
                    ),
                    wire_runtime: false,
                    direct_vector_runtime: false,
                })
            }
            _ => Err(JavaHost::unsupported("callback direct parameter")),
        }
    }

    fn encoded(
        &mut self,
        ty: &'plan TypeRef,
        codec: &'plan <OutOfRust as Direction>::Codec,
        _: native::BufferShape,
        _: <OutOfRust as Direction>::Receive,
    ) -> Self::Output {
        let reader = self.source.generated("reader", self.version)?;
        let decoded = self.source.generated("value", self.version)?;
        let value = codec
            .render_with(&mut Reader::new(reader.clone(), self.version, self.context))?
            .into_expression();
        Ok(InvocationParameter {
            public: Parameter::new(
                self.name.clone(),
                ValueType::Reference(JavaType::type_ref(ty, self.version, self.context)?),
            ),
            jvm: Parameter::new(self.name.clone(), ValueType::Reference(Self::byte_array())),
            setup: vec![
                Statement::value(
                    TypeName::named(TypeIdentifier::known("WireReader", self.version)),
                    reader,
                    Expression::construct(
                        TypeName::named(TypeIdentifier::known("WireReader", self.version)),
                        [Expression::identifier(self.name.clone())]
                            .into_iter()
                            .collect(),
                    ),
                ),
                Statement::value(
                    JavaType::type_ref(ty, self.version, self.context)?,
                    decoded.clone(),
                    value,
                ),
            ],
            argument: Expression::identifier(decoded),
            wire_runtime: true,
            direct_vector_runtime: false,
        })
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
                Ok(InvocationParameter {
                    public: Parameter::new(
                        self.name.clone(),
                        ValueType::Reference(handle.ty().clone()),
                    ),
                    jvm: Parameter::new(self.name.clone(), ValueType::Primitive(handle.carrier())),
                    setup: Vec::new(),
                    argument: handle.value_expression(value)?,
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
                Ok(InvocationParameter {
                    public: Parameter::new(
                        self.name.clone(),
                        ValueType::Reference(handle.ty().clone()),
                    ),
                    jvm: Parameter::new(self.name.clone(), ValueType::Primitive(handle.carrier())),
                    setup: Vec::new(),
                    argument: handle.value_expression(value)?,
                    wire_runtime: false,
                    direct_vector_runtime: false,
                })
            }
            _ => Err(JavaHost::unsupported("callback handle parameter")),
        }
    }

    fn scalar_option(&mut self, primitive: BindingPrimitive) -> Self::Output {
        let primitive = Primitive::try_from(primitive)?;
        let reader = self.source.generated("reader", self.version)?;
        let decoded = self.source.generated("value", self.version)?;
        let reader_value = Expression::identifier(reader.clone());
        let value = reader_value.clone().call(
            Identifier::known("readOptional"),
            [Expression::lambda(
                [],
                reader_value.call(
                    Identifier::parse_for(
                        format!("read{}", primitive.wire_method_suffix()),
                        self.version,
                    )?,
                    ArgumentList::default(),
                ),
            )]
            .into_iter()
            .collect(),
        );
        let ty = JavaType::optional_primitive(primitive, self.version);
        Ok(InvocationParameter {
            public: Parameter::new(self.name.clone(), ValueType::Reference(ty.clone())),
            jvm: Parameter::new(self.name.clone(), ValueType::Reference(Self::byte_array())),
            setup: vec![
                Statement::value(
                    TypeName::named(TypeIdentifier::known("WireReader", self.version)),
                    reader,
                    Expression::construct(
                        TypeName::named(TypeIdentifier::known("WireReader", self.version)),
                        [Expression::identifier(self.name.clone())]
                            .into_iter()
                            .collect(),
                    ),
                ),
                Statement::value(ty, decoded.clone(), value),
            ],
            argument: Expression::identifier(decoded),
            wire_runtime: true,
            direct_vector_runtime: false,
        })
    }

    fn direct_vector(
        &mut self,
        element: &'plan boltffi_binding::DirectVectorElementType,
    ) -> Self::Output {
        let vector = DirectVector::from_element(element, self.version, self.context)?;
        Ok(InvocationParameter {
            public: Parameter::new(self.name.clone(), ValueType::Reference(vector.ty().clone())),
            jvm: Parameter::new(
                self.name.clone(),
                ValueType::Reference(vector.parameter_jvm_type()),
            ),
            setup: Vec::new(),
            argument: vector.decoded_argument(Expression::identifier(self.name.clone())),
            wire_runtime: false,
            direct_vector_runtime: true,
        })
    }
}

impl InvocationParameterRender<'_> {
    fn byte_array() -> TypeName {
        TypeName::array(TypeName::primitive(Primitive::Byte))
    }
}

impl<'plan> ReturnPlanRender<'plan, Native, boltffi_binding::IntoRust>
    for InvocationReturnRender<'_>
{
    type Output = Result<InvocationReturn>;

    fn void(&mut self) -> Self::Output {
        Ok(InvocationReturn {
            public: ReturnType::Void,
            jvm: ReturnType::Void,
            conversion: InvocationReturnConversion::Void,
            wire_runtime: false,
            direct_vector_runtime: false,
        })
    }

    fn direct(&mut self, slot: ReturnValueSlot, ty: &'plan DirectValueType) -> Self::Output {
        if slot != ReturnValueSlot::ReturnSlot
            && !(self.fallible && slot == ReturnValueSlot::OutPointer)
        {
            return Err(JavaHost::unsupported("callback out-pointer return"));
        }
        match ty {
            DirectValueType::Primitive(primitive) => {
                let value =
                    ReturnType::Value(ValueType::Primitive(Primitive::try_from(*primitive)?));
                Ok(InvocationReturn {
                    public: value.clone(),
                    jvm: value,
                    conversion: InvocationReturnConversion::Direct,
                    wire_runtime: false,
                    direct_vector_runtime: false,
                })
            }
            DirectValueType::Record(record) => {
                let record = Record::type_name_for(*record, self.context, self.version)?;
                Ok(InvocationReturn {
                    public: ReturnType::Value(ValueType::Reference(TypeName::named(record))),
                    jvm: ReturnType::Value(ValueType::Reference(
                        InvocationParameterRender::byte_array(),
                    )),
                    conversion: InvocationReturnConversion::DirectRecord,
                    wire_runtime: false,
                    direct_vector_runtime: false,
                })
            }
            DirectValueType::Enum(enumeration) => {
                let name = Enumeration::type_name_for(*enumeration, self.context, self.version)?;
                let carrier = Enumeration::c_style_primitive(*enumeration, self.context)?;
                Ok(InvocationReturn {
                    public: ReturnType::Value(ValueType::Reference(TypeName::named(name))),
                    jvm: ReturnType::Value(ValueType::Primitive(carrier)),
                    conversion: InvocationReturnConversion::DirectEnum,
                    wire_runtime: false,
                    direct_vector_runtime: false,
                })
            }
            _ => Err(JavaHost::unsupported("callback direct return")),
        }
    }

    fn encoded(
        &mut self,
        slot: ReturnValueSlot,
        ty: &'plan TypeRef,
        codec: &'plan <IntoRust as Direction>::Codec,
        _: native::BufferShape,
    ) -> Self::Output {
        if slot != ReturnValueSlot::ReturnSlot
            && !(self.fallible && slot == ReturnValueSlot::OutPointer)
        {
            return Err(JavaHost::unsupported("callback encoded return slot"));
        }
        let ty = JavaType::type_ref(ty, self.version, self.context)?;
        Ok(InvocationReturn {
            public: ReturnType::Value(ValueType::Reference(ty.clone())),
            jvm: ReturnType::Value(ValueType::Reference(InvocationParameterRender::byte_array())),
            conversion: InvocationReturnConversion::Encoded {
                source: self.source.clone(),
                ty,
                codec: codec.clone(),
            },
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
        if slot != ReturnValueSlot::ReturnSlot
            && !(self.fallible && slot == ReturnValueSlot::OutPointer)
        {
            return Err(JavaHost::unsupported("callback handle return slot"));
        }
        match target {
            HandleTarget::Class(class) => {
                let handle =
                    ClassHandle::new(*class, carrier, presence, self.version, self.context, None)?;
                Ok(InvocationReturn {
                    public: ReturnType::Value(ValueType::Reference(handle.ty().clone())),
                    jvm: ReturnType::Value(ValueType::Primitive(handle.carrier())),
                    conversion: InvocationReturnConversion::ClassHandle(handle),
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
                Ok(InvocationReturn {
                    public: ReturnType::Value(ValueType::Reference(handle.ty().clone())),
                    jvm: ReturnType::Value(ValueType::Primitive(handle.carrier())),
                    conversion: InvocationReturnConversion::CallbackHandle(handle),
                    wire_runtime: false,
                    direct_vector_runtime: false,
                })
            }
            _ => Err(JavaHost::unsupported("callback handle return")),
        }
    }

    fn scalar_option(&mut self, primitive: BindingPrimitive) -> Self::Output {
        let primitive = Primitive::try_from(primitive)?;
        Ok(InvocationReturn {
            public: ReturnType::Value(ValueType::Reference(JavaType::optional_primitive(
                primitive,
                self.version,
            ))),
            jvm: ReturnType::Value(ValueType::Reference(InvocationParameterRender::byte_array())),
            conversion: InvocationReturnConversion::ScalarOption {
                source: self.source.clone(),
                primitive,
            },
            wire_runtime: true,
            direct_vector_runtime: false,
        })
    }

    fn direct_vector(
        &mut self,
        element: &'plan boltffi_binding::DirectVectorElementType,
    ) -> Self::Output {
        let vector = DirectVector::from_element(element, self.version, self.context)?;
        Ok(InvocationReturn {
            public: ReturnType::Value(ValueType::Reference(vector.ty().clone())),
            jvm: ReturnType::Value(ValueType::Reference(InvocationParameterRender::byte_array())),
            conversion: InvocationReturnConversion::DirectVector(vector),
            wire_runtime: false,
            direct_vector_runtime: true,
        })
    }

    fn closure(
        &mut self,
        _: &'plan boltffi_binding::ClosureReturn<Native, boltffi_binding::IntoRust>,
    ) -> Self::Output {
        Err(JavaHost::unsupported("callback closure return"))
    }
}

impl InvocationReturn {
    fn fallible_statements(
        &self,
        call: Expression,
        fallible: &FallibleReturn,
        version: JavaVersion,
        context: &RenderContext<Native>,
    ) -> Result<Vec<Statement>> {
        let success = self.fallible_success(call, fallible, version, context)?;
        let error = Identifier::known("__boltffi_error");
        let recovery =
            fallible.error_statements(Expression::identifier(error.clone()), version, context)?;
        Ok(vec![Statement::try_catch(
            success,
            fallible.catch_type(version, context)?,
            error,
            recovery,
        )])
    }

    fn fallible_success(
        &self,
        call: Expression,
        fallible: &FallibleReturn,
        version: JavaVersion,
        context: &RenderContext<Native>,
    ) -> Result<Vec<Statement>> {
        let Some(success_out) = fallible.success_out.as_ref() else {
            return match self.conversion {
                InvocationReturnConversion::Void => Ok(vec![
                    Statement::expression(call),
                    Statement::return_value(Self::empty_bytes()),
                ]),
                _ => Err(JavaHost::broken_bridge_contract(
                    "fallible callback value return has a success out-pointer",
                )),
            };
        };
        let result = Identifier::known("__boltffi_result");
        let result_value = Expression::identifier(result.clone());
        let writer_method = Identifier::parse_for(success_out.writer().as_str(), version)?;
        let success_name = Identifier::parse_for(success_out.name().as_str(), version)?;
        let write = |payload| {
            Statement::expression(Expression::static_call(
                TypeName::named(TypeIdentifier::known("Native", version)),
                writer_method.clone(),
                [Expression::identifier(success_name.clone()), payload]
                    .into_iter()
                    .collect(),
            ))
        };
        let direct = |payload| {
            Ok(vec![
                Statement::value(self.public_type()?, result.clone(), call.clone()),
                write(payload),
                Statement::return_value(Self::empty_bytes()),
            ])
        };
        match &self.conversion {
            InvocationReturnConversion::Direct => direct(result_value),
            InvocationReturnConversion::DirectRecord => {
                direct(result_value.call(Identifier::known("toByteArray"), ArgumentList::default()))
            }
            InvocationReturnConversion::DirectEnum => {
                direct(result_value.call(Identifier::known("nativeValue"), ArgumentList::default()))
            }
            InvocationReturnConversion::DirectVector(vector) => {
                direct(vector.callback_return_expression(result_value))
            }
            InvocationReturnConversion::ClassHandle(handle) => {
                direct(handle.native_argument(result_value)?)
            }
            InvocationReturnConversion::CallbackHandle(handle) => {
                direct(handle.native_argument(result_value)?)
            }
            InvocationReturnConversion::Encoded { source, ty, codec } => {
                let write_buffer =
                    WireBuffer::new(source, version)?.write(codec, result_value, context)?;
                Ok(std::iter::once(Statement::value(ty.clone(), result, call))
                    .chain(Self::bytes_write(write_buffer, write))
                    .collect())
            }
            InvocationReturnConversion::ScalarOption { source, primitive } => {
                let writer = source.generated("writer", version)?;
                let payload = source.generated("value", version)?;
                let result_value = Expression::identifier(result.clone());
                let write_buffer = WireBuffer::new(source, version)?.write_statements(
                    Expression::integer(1).add(
                        result_value
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
                                result_value,
                                Expression::lambda_statement(
                                    [payload.clone()],
                                    Statement::expression(Expression::identifier(writer).call(
                                        Identifier::parse_for(
                                            format!("write{}", primitive.wire_method_suffix()),
                                            version,
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
                Ok(
                    std::iter::once(Statement::value(self.public_type()?, result, call))
                        .chain(Self::bytes_write(write_buffer, write))
                        .collect(),
                )
            }
            InvocationReturnConversion::Void => Err(JavaHost::broken_bridge_contract(
                "fallible void callback has a success out-pointer",
            )),
        }
    }

    fn statements(
        &self,
        call: Expression,
        version: JavaVersion,
        context: &RenderContext<Native>,
    ) -> Result<Vec<Statement>> {
        match &self.conversion {
            InvocationReturnConversion::Void => Ok(vec![Statement::expression(call)]),
            InvocationReturnConversion::Direct => Ok(vec![Statement::return_value(call)]),
            InvocationReturnConversion::DirectRecord => Ok(vec![Statement::return_value(
                call.call(Identifier::known("toByteArray"), ArgumentList::default()),
            )]),
            InvocationReturnConversion::DirectEnum => Ok(vec![Statement::return_value(
                call.call(Identifier::known("nativeValue"), ArgumentList::default()),
            )]),
            InvocationReturnConversion::DirectVector(vector) => Ok(vec![Statement::return_value(
                vector.callback_return_expression(call),
            )]),
            InvocationReturnConversion::Encoded { source, ty, codec } => {
                self.encoded_statements(source, ty.clone(), codec, call, version, context)
            }
            InvocationReturnConversion::ScalarOption { source, primitive } => {
                let value = Identifier::known("__boltffi_result");
                let writer = source.generated("writer", version)?;
                let payload = source.generated("value", version)?;
                let result = Expression::identifier(value.clone());
                let write = WireBuffer::new(source, version)?.write_statements(
                    Expression::integer(1).add(
                        result
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
                                result,
                                Expression::lambda_statement(
                                    [payload.clone()],
                                    Statement::expression(Expression::identifier(writer).call(
                                        Identifier::parse_for(
                                            format!("write{}", primitive.wire_method_suffix()),
                                            version,
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
                Ok(
                    std::iter::once(Statement::value(self.public_type()?, value, call))
                        .chain(Self::bytes_return(write))
                        .collect(),
                )
            }
            InvocationReturnConversion::ClassHandle(handle) => handle
                .native_argument(call)
                .map(Statement::return_value)
                .map(|statement| vec![statement]),
            InvocationReturnConversion::CallbackHandle(handle) => handle
                .native_argument(call)
                .map(Statement::return_value)
                .map(|statement| vec![statement]),
        }
    }

    fn encoded_statements(
        &self,
        source: &Name,
        ty: TypeName,
        codec: &WritePlan,
        call: Expression,
        version: JavaVersion,
        context: &RenderContext<Native>,
    ) -> Result<Vec<Statement>> {
        let result = Identifier::known("__boltffi_result");
        let write = WireBuffer::new(source, version)?.write(
            codec,
            Expression::identifier(result.clone()),
            context,
        )?;
        Ok(std::iter::once(Statement::value(ty, result, call))
            .chain(Self::bytes_return(write))
            .collect())
    }

    fn bytes_return(write: crate::target::java::codec::EncodedWrite) -> Vec<Statement> {
        let (acquire, prepare, bytes, cleanup) = write.into_bytes_parts();
        acquire
            .into_iter()
            .chain(std::iter::once(Statement::try_finally(
                prepare
                    .into_iter()
                    .chain(std::iter::once(Statement::return_value(bytes)))
                    .collect(),
                cleanup,
            )))
            .collect()
    }

    fn bytes_write(
        write: crate::target::java::codec::EncodedWrite,
        consume: impl FnOnce(Expression) -> Statement,
    ) -> Vec<Statement> {
        let (acquire, prepare, bytes, cleanup) = write.into_bytes_parts();
        acquire
            .into_iter()
            .chain(std::iter::once(Statement::try_finally(
                prepare
                    .into_iter()
                    .chain([consume(bytes), Statement::return_value(Self::empty_bytes())])
                    .collect(),
                cleanup,
            )))
            .collect()
    }

    fn empty_bytes() -> Expression {
        Expression::array(TypeName::primitive(Primitive::Byte), Expression::integer(0))
    }

    fn public_type(&self) -> Result<TypeName> {
        match &self.public {
            ReturnType::Value(ValueType::Primitive(primitive)) => {
                Ok(TypeName::primitive(*primitive))
            }
            ReturnType::Value(ValueType::Record(record)) => Ok(TypeName::named(record.clone())),
            ReturnType::Value(ValueType::Reference(ty)) => Ok(ty.clone()),
            _ => Err(JavaHost::broken_bridge_contract(
                "encoded callback return has a reference API type",
            )),
        }
    }
}

impl FallibleReturn {
    fn from_channel(
        source: Name,
        channel: ErrorChannel<'_, Native, IntoRust>,
        success_out: Option<SuccessOutArgument>,
    ) -> Result<Option<Self>> {
        match channel {
            ErrorChannel::None => Ok(None),
            ErrorChannel::Encoded {
                placement: ErrorPlacement::ReturnSlot,
                ty,
                codec,
                ..
            } => Ok(Some(Self {
                source,
                error_ty: ty.clone(),
                error_codec: codec.clone(),
                success_out,
            })),
            ErrorChannel::Encoded { .. } | ErrorChannel::Status => {
                Err(JavaHost::unsupported("callback method error return"))
            }
            _ => Err(JavaHost::unsupported(
                "unknown callback method error return",
            )),
        }
    }

    fn catch_type(
        &self,
        version: JavaVersion,
        context: &RenderContext<Native>,
    ) -> Result<TypeName> {
        match &self.error_ty {
            TypeRef::String => Ok(TypeName::named(TypeIdentifier::known(
                "RuntimeException",
                version,
            ))),
            TypeRef::Record(record) => {
                Record::type_name_for(*record, context, version).map(TypeName::named)
            }
            TypeRef::Enum(enumeration) => {
                let name =
                    TypeName::named(Enumeration::type_name_for(*enumeration, context, version)?);
                match context
                    .enumeration(*enumeration)
                    .ok_or(JavaHost::broken_bridge_contract(
                        "callback enum error was not found in render context",
                    ))? {
                    boltffi_binding::EnumDecl::CStyle(_) => Ok(TypeName::nested(
                        name,
                        TypeIdentifier::known("Exception", version),
                    )),
                    boltffi_binding::EnumDecl::Data(enumeration)
                        if enumeration.variants().iter().all(|variant| {
                            matches!(variant.payload(), DataVariantPayload::Unit)
                        }) =>
                    {
                        Ok(TypeName::nested(
                            name,
                            TypeIdentifier::known("Exception", version),
                        ))
                    }
                    boltffi_binding::EnumDecl::Data(_) => Ok(name),
                    _ => Err(JavaHost::unsupported("callback enum error type")),
                }
            }
            _ => Err(JavaHost::unsupported("callback error type")),
        }
    }

    fn error_statements(
        &self,
        error: Expression,
        version: JavaVersion,
        context: &RenderContext<Native>,
    ) -> Result<Vec<Statement>> {
        let payload = match &self.error_ty {
            TypeRef::String => error.call(Identifier::known("getMessage"), ArgumentList::default()),
            TypeRef::Record(_) => error,
            TypeRef::Enum(enumeration) => {
                match context
                    .enumeration(*enumeration)
                    .ok_or(JavaHost::broken_bridge_contract(
                        "callback enum error was not found in render context",
                    ))? {
                    boltffi_binding::EnumDecl::CStyle(_) => {
                        error.call(Identifier::known("getError"), ArgumentList::default())
                    }
                    boltffi_binding::EnumDecl::Data(enumeration)
                        if enumeration.variants().iter().all(|variant| {
                            matches!(variant.payload(), DataVariantPayload::Unit)
                        }) =>
                    {
                        error.call(Identifier::known("getError"), ArgumentList::default())
                    }
                    boltffi_binding::EnumDecl::Data(_) => error,
                    _ => return Err(JavaHost::unsupported("callback enum error type")),
                }
            }
            _ => return Err(JavaHost::unsupported("callback error type")),
        };
        WireBuffer::new(&self.source, version)
            .and_then(|buffer| buffer.write(&self.error_codec, payload, context))
            .map(InvocationReturn::bytes_return)
    }
}
