use std::collections::{BTreeMap, btree_map::Entry};

use askama::Template as AskamaTemplate;
use boltffi_binding::{
    CallableDecl, ClosureParameter, DirectValueType, DirectVectorElementType, Direction,
    ErrorChannel, ErrorPlacement, ExportedCallable, ForeignBody, HandlePresence, HandleTarget,
    IncomingParam, IntoRust, Native, OutOfRust, ParamDecl, ParamPlanRender, Primitive,
    ReturnPlanRender, ReturnValueSlot, Surface, TypeRef, WritePlan,
};

use crate::{
    bridge::jni::{ClosureRegistration, JniBridgeContract, SuccessOutArgument},
    core::{Error, RenderContext, RenderedDeclaration, Result},
    target::kotlin::{
        codec::{Reader, ScalarOption, WireBuffer},
        name_style::Name,
        primitive::KotlinPrimitive,
        render::{
            callback::CallbackHandle, class::ClassHandle, direct_vector::DirectVector,
            enumeration::Enumeration, record::Record, type_name::KotlinType,
        },
        syntax::{ArgumentList, Expression, Identifier, Statement, TypeName},
    },
};

const KOTLIN_TARGET: &str = "kotlin";
const JNI_BRIDGE: &str = "jni";

#[derive(AskamaTemplate)]
#[template(path = "target/kotlin/closure.kt", escape = "none")]
struct ClosureTemplate {
    closure: Closure,
}

pub struct Closure {
    name: TypeName,
    function_type: TypeName,
    parameters: Vec<Parameter>,
    returns: Option<TypeName>,
    setup: Vec<Statement>,
    call: Vec<Statement>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Parameter {
    name: Identifier,
    ty: TypeName,
}

struct ClosureParameterView {
    public: Parameter,
    jvm: Parameter,
    setup: Vec<Statement>,
    call_argument: Expression,
}

struct ClosureReturn {
    public_ty: Option<TypeName>,
    jvm_ty: Option<TypeName>,
    conversion: ReturnConversion,
}

enum ReturnConversion {
    Void,
    DirectPrimitive(Primitive),
    DirectRecord,
    DirectEnum {
        repr: Primitive,
    },
    ClassHandle(ClassHandle),
    CallbackHandle(CallbackHandle),
    Encoded {
        codec: <IntoRust as Direction>::Codec,
        source_name: Name,
    },
    ScalarOption {
        primitive: Primitive,
        source_name: Name,
    },
    DirectVector(DirectVector),
}

struct ParameterRender<'context> {
    source_name: Name,
    name: Identifier,
    context: &'context RenderContext<'context, Native>,
}

struct ReturnRender<'context> {
    source_name: Name,
    context: &'context RenderContext<'context, Native>,
    fallible_success_out: bool,
}

struct FallibleReturn<'error> {
    source_name: Name,
    success_out: Option<SuccessOutArgument>,
    error_codec: &'error WritePlan,
}

pub struct Closures {
    closures: Vec<Closure>,
}

impl Closures {
    pub fn from_declarations(
        declarations: &[RenderedDeclaration<'_, Native>],
        bridge: &JniBridgeContract,
        context: &RenderContext<Native>,
    ) -> Result<Self> {
        let registrations = ClosureRegistrations::new(bridge.closures());
        let closures = declarations
            .iter()
            .filter(|declaration| !declaration.emitted().primary_chunk().is_empty())
            .map(RenderedDeclaration::declaration)
            .flat_map(ClosureSource::from_declaration)
            .try_fold(BTreeMap::new(), |mut closures, closure| {
                match closures.entry(closure.signature().clone()) {
                    Entry::Occupied(_) => Ok::<_, Error>(closures),
                    Entry::Vacant(entry) => {
                        let registration = registrations.get(closure)?;
                        entry.insert(Closure::from_parameter(closure, registration, context)?);
                        Ok::<_, Error>(closures)
                    }
                }
            })?
            .into_values()
            .collect::<Vec<_>>();
        Ok(Self { closures })
    }

    pub fn render(self) -> Result<Vec<String>> {
        self.closures
            .into_iter()
            .map(|closure| ClosureTemplate { closure }.render().map_err(Error::from))
            .collect()
    }
}

impl Closure {
    pub fn type_name(
        closure: &ClosureParameter<Native, IntoRust>,
        context: &RenderContext<Native>,
    ) -> Result<TypeName> {
        let function_type = Self::invoke_type(closure.invoke(), context)?;
        match closure.presence() {
            HandlePresence::Required => Ok(function_type),
            HandlePresence::Nullable => Ok(function_type.nullable()),
            _ => Err(Error::UnsupportedTarget {
                target: KOTLIN_TARGET,
                shape: "unknown closure parameter presence",
            }),
        }
    }

    pub fn native_argument(
        closure: &ClosureParameter<Native, IntoRust>,
        value: Expression,
        bridge: &JniBridgeContract,
    ) -> Result<Expression> {
        let registration = ClosureRegistrations::new(bridge.closures()).get(closure)?;
        let bridge_name = TypeName::new(registration.class().class_name());
        let insert = Identifier::parse("insert")?;
        let required = Expression::call(
            bridge_name.clone(),
            insert.clone(),
            [value.clone()].into_iter().collect::<ArgumentList>(),
        );
        match closure.presence() {
            HandlePresence::Required => Ok(required),
            HandlePresence::Nullable => Ok(value.let_or_else(
                Identifier::parse("__boltffi_closure")?,
                Expression::call(
                    bridge_name,
                    insert,
                    [Expression::identifier(Identifier::parse(
                        "__boltffi_closure",
                    )?)]
                    .into_iter()
                    .collect::<ArgumentList>(),
                ),
                Expression::long(0),
            )),
            _ => Err(Error::UnsupportedTarget {
                target: KOTLIN_TARGET,
                shape: "unknown closure parameter presence",
            }),
        }
    }

    pub fn name(&self) -> &TypeName {
        &self.name
    }

    pub fn function_type(&self) -> &TypeName {
        &self.function_type
    }

    pub fn parameters(&self) -> &[Parameter] {
        &self.parameters
    }

    pub fn returns(&self) -> Option<&TypeName> {
        self.returns.as_ref()
    }

    pub fn setup(&self) -> &[Statement] {
        &self.setup
    }

    pub fn call(&self) -> &[Statement] {
        &self.call
    }

    fn from_parameter(
        closure: &ClosureParameter<Native, IntoRust>,
        registration: &ClosureRegistration,
        context: &RenderContext<Native>,
    ) -> Result<Self> {
        let callable = closure.invoke();
        let parameters = callable
            .params()
            .iter()
            .map(|parameter| ClosureParameterView::from_declaration(parameter, context))
            .collect::<Result<Vec<_>>>()?;
        let source_name = Name::new(&boltffi_binding::CanonicalName::single("closure"));
        let fallible = FallibleReturn::from_registration(
            source_name.clone(),
            callable.error().channel(),
            registration,
        )?;
        let returned = callable.returns().plan().render_with(&mut ReturnRender {
            source_name,
            context,
            fallible_success_out: fallible.is_some(),
        })?;
        let implementation = Expression::identifier(Identifier::parse("impl")?);
        let call = Expression::invoke(
            implementation,
            parameters
                .iter()
                .map(|parameter| parameter.call_argument.clone())
                .collect::<ArgumentList>(),
        );
        let jvm_parameters = fallible
            .as_ref()
            .map(FallibleReturn::parameter)
            .transpose()?
            .into_iter()
            .flatten()
            .collect::<Vec<_>>();
        let jvm_parameters = parameters
            .iter()
            .map(|parameter| parameter.jvm.clone())
            .chain(jvm_parameters)
            .collect();
        Ok(Self {
            name: TypeName::new(registration.class().class_name()),
            function_type: Self::invoke_type(callable, context)?,
            parameters: jvm_parameters,
            returns: fallible
                .as_ref()
                .map(|_| TypeName::byte_array(false))
                .or_else(|| returned.jvm_ty.clone()),
            setup: parameters
                .iter()
                .flat_map(|parameter| parameter.setup.iter().cloned())
                .collect(),
            call: match fallible {
                Some(fallible) => returned.fallible_statements(call, fallible, context)?,
                None => returned.statements(call, context)?,
            },
        })
    }

    fn invoke_type(
        callable: &CallableDecl<Native, ForeignBody>,
        context: &RenderContext<Native>,
    ) -> Result<TypeName> {
        let parameters = callable
            .params()
            .iter()
            .map(|parameter| ClosureParameterView::from_declaration(parameter, context))
            .map(|parameter| parameter.map(|parameter| parameter.public.ty))
            .collect::<Result<Vec<_>>>()?;
        let returns = callable
            .returns()
            .plan()
            .render_with(&mut ReturnRender {
                source_name: Name::new(&boltffi_binding::CanonicalName::single("closure")),
                context,
                fallible_success_out: matches!(
                    callable.error().channel(),
                    ErrorChannel::Encoded {
                        placement: ErrorPlacement::ReturnSlot,
                        ..
                    }
                ),
            })?
            .public_ty
            .unwrap_or_else(TypeName::unit);
        let returns = match callable.error().channel() {
            ErrorChannel::None => returns,
            ErrorChannel::Encoded {
                placement: ErrorPlacement::ReturnSlot,
                ty,
                ..
            } => TypeName::result(returns, KotlinType::type_ref(ty, context)?),
            ErrorChannel::Encoded { .. } | ErrorChannel::Status => {
                return Err(Error::UnsupportedTarget {
                    target: KOTLIN_TARGET,
                    shape: "closure error return",
                });
            }
            _ => {
                return Err(Error::UnsupportedTarget {
                    target: KOTLIN_TARGET,
                    shape: "unknown closure error return",
                });
            }
        };
        Ok(TypeName::function(parameters, returns))
    }
}

impl Parameter {
    pub fn name(&self) -> &Identifier {
        &self.name
    }

    pub fn ty(&self) -> &TypeName {
        &self.ty
    }
}

impl ClosureParameterView {
    fn from_declaration(
        parameter: &ParamDecl<Native, OutOfRust>,
        context: &RenderContext<Native>,
    ) -> Result<Self> {
        let boltffi_binding::OutgoingParam::Value(plan) = parameter.payload() else {
            return Err(Error::UnsupportedTarget {
                target: KOTLIN_TARGET,
                shape: "closure nested closure parameter",
            });
        };
        let source_name = Name::new(parameter.name());
        let name = source_name.parameter()?;
        plan.render_with(&mut ParameterRender {
            source_name,
            name,
            context,
        })
    }
}

impl<'error> FallibleReturn<'error> {
    fn from_registration(
        source_name: Name,
        channel: ErrorChannel<'error, Native, IntoRust>,
        registration: &ClosureRegistration,
    ) -> Result<Option<Self>> {
        match channel {
            ErrorChannel::None => Ok(None),
            ErrorChannel::Encoded {
                placement: ErrorPlacement::ReturnSlot,
                codec,
                ..
            } => Ok(Some(Self {
                source_name,
                success_out: registration.success_out(),
                error_codec: codec,
            })),
            ErrorChannel::Encoded { .. } | ErrorChannel::Status => Err(Error::UnsupportedTarget {
                target: KOTLIN_TARGET,
                shape: "closure error return",
            }),
            _ => Err(Error::UnsupportedTarget {
                target: KOTLIN_TARGET,
                shape: "unknown closure error return",
            }),
        }
    }

    fn parameter(&self) -> Result<Option<Parameter>> {
        self.success_out
            .as_ref()
            .map(|success_out| {
                Ok(Parameter {
                    name: Identifier::escape(success_out.name().as_str())?,
                    ty: KotlinType::jni(success_out.jni_type())?,
                })
            })
            .transpose()
    }
}

impl<'plan> ParamPlanRender<'plan, Native, OutOfRust> for ParameterRender<'_> {
    type Output = Result<ClosureParameterView>;

    fn direct(
        &mut self,
        ty: &'plan DirectValueType,
        _receive: <OutOfRust as Direction>::Receive,
    ) -> Self::Output {
        match ty {
            DirectValueType::Primitive(primitive) => {
                let value = Expression::identifier(self.name.clone());
                Ok(ClosureParameterView {
                    public: Parameter {
                        name: self.name.clone(),
                        ty: KotlinPrimitive::new(*primitive).api_type()?,
                    },
                    jvm: Parameter {
                        name: self.name.clone(),
                        ty: KotlinPrimitive::new(*primitive).native_type()?,
                    },
                    setup: Vec::new(),
                    call_argument: KotlinPrimitive::new(*primitive).public_return(value)?,
                })
            }
            DirectValueType::Enum(enumeration) => {
                let enumeration = Enumeration::from_id(*enumeration, self.context)?;
                let repr = enumeration.repr()?;
                let value = KotlinPrimitive::new(repr)
                    .public_return(Expression::identifier(self.name.clone()))?;
                Ok(ClosureParameterView {
                    public: Parameter {
                        name: self.name.clone(),
                        ty: enumeration.name().clone(),
                    },
                    jvm: Parameter {
                        name: self.name.clone(),
                        ty: KotlinPrimitive::new(repr).native_type()?,
                    },
                    setup: Vec::new(),
                    call_argument: Expression::call(
                        enumeration.name().clone(),
                        Identifier::parse("fromValue")?,
                        [value].into_iter().collect::<ArgumentList>(),
                    ),
                })
            }
            DirectValueType::Record(record) => {
                let ty = Record::type_name_from_id(*record, self.context)?;
                Ok(ClosureParameterView {
                    public: Parameter {
                        name: self.name.clone(),
                        ty: ty.clone(),
                    },
                    jvm: Parameter {
                        name: self.name.clone(),
                        ty: TypeName::byte_array(false),
                    },
                    setup: Vec::new(),
                    call_argument: Record::decode_expression(
                        ty,
                        Expression::identifier(self.name.clone()),
                    )?,
                })
            }
            _ => Err(Error::UnsupportedTarget {
                target: KOTLIN_TARGET,
                shape: "closure direct parameter",
            }),
        }
    }

    fn encoded(
        &mut self,
        ty: &'plan TypeRef,
        codec: &'plan <OutOfRust as Direction>::Codec,
        _shape: <Native as Surface>::BufferShape,
        _receive: <OutOfRust as Direction>::Receive,
    ) -> Self::Output {
        let reader = self.source_name.generated("reader")?;
        let value = self.source_name.generated("value")?;
        let expression = codec.render_with(&mut Reader::new(reader.clone(), self.context))?;
        Ok(ClosureParameterView {
            public: Parameter {
                name: self.name.clone(),
                ty: KotlinType::type_ref(ty, self.context)?,
            },
            jvm: Parameter {
                name: self.name.clone(),
                ty: TypeName::byte_array(false),
            },
            setup: vec![
                Statement::value(
                    reader,
                    Expression::construct(
                        TypeName::new("WireReader"),
                        [Expression::identifier(self.name.clone())]
                            .into_iter()
                            .collect::<ArgumentList>(),
                    ),
                ),
                Statement::value(value.clone(), expression),
            ],
            call_argument: Expression::identifier(value),
        })
    }

    fn handle(
        &mut self,
        target: &'plan HandleTarget,
        _carrier: <Native as Surface>::HandleCarrier,
        presence: HandlePresence,
        _receive: <OutOfRust as Direction>::Receive,
    ) -> Self::Output {
        let value = Expression::identifier(self.name.clone());
        match target {
            HandleTarget::Class(class) => {
                let handle = ClassHandle::new(*class, presence, self.context)?;
                Ok(ClosureParameterView {
                    public: Parameter {
                        name: self.name.clone(),
                        ty: handle.ty()?,
                    },
                    jvm: Parameter {
                        name: self.name.clone(),
                        ty: TypeName::long(),
                    },
                    setup: Vec::new(),
                    call_argument: handle.value_expression(value)?,
                })
            }
            HandleTarget::Callback(callback) => {
                let handle = CallbackHandle::new(*callback, presence, self.context)?;
                Ok(ClosureParameterView {
                    public: Parameter {
                        name: self.name.clone(),
                        ty: handle.ty()?,
                    },
                    jvm: Parameter {
                        name: self.name.clone(),
                        ty: TypeName::long(),
                    },
                    setup: Vec::new(),
                    call_argument: handle.value_expression(value)?,
                })
            }
            HandleTarget::Stream(_) => Err(Error::UnsupportedTarget {
                target: KOTLIN_TARGET,
                shape: "closure stream parameter",
            }),
            _ => Err(Error::UnsupportedTarget {
                target: KOTLIN_TARGET,
                shape: "unknown closure handle parameter",
            }),
        }
    }

    fn scalar_option(&mut self, primitive: Primitive) -> Self::Output {
        let reader = self.source_name.generated("reader")?;
        let value = self.source_name.generated("value")?;
        Ok(ClosureParameterView {
            public: Parameter {
                name: self.name.clone(),
                ty: ScalarOption::new(primitive).ty()?,
            },
            jvm: Parameter {
                name: self.name.clone(),
                ty: TypeName::byte_array(false),
            },
            setup: vec![
                Statement::value(
                    reader.clone(),
                    Expression::construct(
                        TypeName::new("WireReader"),
                        [Expression::identifier(self.name.clone())]
                            .into_iter()
                            .collect::<ArgumentList>(),
                    ),
                ),
                Statement::value(
                    value.clone(),
                    ScalarOption::new(primitive).read_expression(reader)?,
                ),
            ],
            call_argument: Expression::identifier(value),
        })
    }

    fn direct_vector(&mut self, element: &'plan DirectVectorElementType) -> Self::Output {
        let vector = DirectVector::from_element(element)?;
        Ok(ClosureParameterView {
            public: Parameter {
                name: self.name.clone(),
                ty: vector.ty().clone(),
            },
            jvm: Parameter {
                name: self.name.clone(),
                ty: vector.ty().clone(),
            },
            setup: Vec::new(),
            call_argument: Expression::identifier(self.name.clone()),
        })
    }
}

impl<'plan> ReturnPlanRender<'plan, Native, IntoRust> for ReturnRender<'_> {
    type Output = Result<ClosureReturn>;

    fn void(&mut self) -> Self::Output {
        Ok(ClosureReturn {
            public_ty: None,
            jvm_ty: None,
            conversion: ReturnConversion::Void,
        })
    }

    fn direct(&mut self, slot: ReturnValueSlot, ty: &'plan DirectValueType) -> Self::Output {
        self.require_supported_slot(slot, "closure out-pointer return")?;
        match ty {
            DirectValueType::Primitive(primitive) => Ok(ClosureReturn {
                public_ty: Some(KotlinPrimitive::new(*primitive).api_type()?),
                jvm_ty: Some(KotlinPrimitive::new(*primitive).native_type()?),
                conversion: ReturnConversion::DirectPrimitive(*primitive),
            }),
            DirectValueType::Record(record) => {
                let ty = Record::type_name_from_id(*record, self.context)?;
                Ok(ClosureReturn {
                    public_ty: Some(ty.clone()),
                    jvm_ty: Some(TypeName::byte_array(false)),
                    conversion: ReturnConversion::DirectRecord,
                })
            }
            DirectValueType::Enum(enumeration) => {
                let enumeration = Enumeration::from_id(*enumeration, self.context)?;
                let repr = enumeration.repr()?;
                Ok(ClosureReturn {
                    public_ty: Some(enumeration.name().clone()),
                    jvm_ty: Some(KotlinPrimitive::new(repr).native_type()?),
                    conversion: ReturnConversion::DirectEnum { repr },
                })
            }
            _ => Err(Error::UnsupportedTarget {
                target: KOTLIN_TARGET,
                shape: "closure direct return",
            }),
        }
    }

    fn encoded(
        &mut self,
        slot: ReturnValueSlot,
        ty: &'plan TypeRef,
        codec: &'plan <IntoRust as Direction>::Codec,
        _shape: <Native as Surface>::BufferShape,
    ) -> Self::Output {
        self.require_supported_slot(slot, "closure out-pointer encoded return")?;
        Ok(ClosureReturn {
            public_ty: Some(KotlinType::type_ref(ty, self.context)?),
            jvm_ty: Some(TypeName::byte_array(false)),
            conversion: ReturnConversion::Encoded {
                codec: codec.clone(),
                source_name: self.source_name.clone(),
            },
        })
    }

    fn handle(
        &mut self,
        slot: ReturnValueSlot,
        target: &'plan HandleTarget,
        _carrier: <Native as Surface>::HandleCarrier,
        presence: HandlePresence,
    ) -> Self::Output {
        self.require_supported_slot(slot, "closure out-pointer handle return")?;
        match target {
            HandleTarget::Class(class) => {
                let handle = ClassHandle::new(*class, presence, self.context)?;
                Ok(ClosureReturn {
                    public_ty: Some(handle.ty()?),
                    jvm_ty: Some(TypeName::long()),
                    conversion: ReturnConversion::ClassHandle(handle),
                })
            }
            HandleTarget::Callback(callback) => {
                let handle = CallbackHandle::new(*callback, presence, self.context)?;
                Ok(ClosureReturn {
                    public_ty: Some(handle.ty()?),
                    jvm_ty: Some(TypeName::long()),
                    conversion: ReturnConversion::CallbackHandle(handle),
                })
            }
            HandleTarget::Stream(_) => Err(Error::UnsupportedTarget {
                target: KOTLIN_TARGET,
                shape: "closure stream return",
            }),
            _ => Err(Error::UnsupportedTarget {
                target: KOTLIN_TARGET,
                shape: "unknown closure handle return",
            }),
        }
    }

    fn scalar_option(&mut self, primitive: Primitive) -> Self::Output {
        Ok(ClosureReturn {
            public_ty: Some(ScalarOption::new(primitive).ty()?),
            jvm_ty: Some(TypeName::byte_array(false)),
            conversion: ReturnConversion::ScalarOption {
                primitive,
                source_name: self.source_name.clone(),
            },
        })
    }

    fn direct_vector(&mut self, element: &'plan DirectVectorElementType) -> Self::Output {
        let vector = DirectVector::from_element(element)?;
        Ok(ClosureReturn {
            public_ty: Some(vector.ty().clone()),
            jvm_ty: Some(TypeName::byte_array(false)),
            conversion: ReturnConversion::DirectVector(vector),
        })
    }

    fn closure(
        &mut self,
        _closure: &'plan boltffi_binding::ClosureReturn<Native, IntoRust>,
    ) -> Self::Output {
        Err(Error::UnsupportedTarget {
            target: KOTLIN_TARGET,
            shape: "closure return from closure",
        })
    }
}

impl ReturnRender<'_> {
    fn require_supported_slot(&self, slot: ReturnValueSlot, shape: &'static str) -> Result<()> {
        match slot {
            ReturnValueSlot::ReturnSlot => Ok(()),
            ReturnValueSlot::OutPointer if self.fallible_success_out => Ok(()),
            ReturnValueSlot::OutPointer => Err(Error::UnsupportedTarget {
                target: KOTLIN_TARGET,
                shape,
            }),
            _ => Err(Error::UnsupportedTarget {
                target: KOTLIN_TARGET,
                shape: "unknown closure return slot",
            }),
        }
    }
}

impl ClosureReturn {
    fn fallible_statements(
        &self,
        call: Expression,
        fallible: FallibleReturn,
        context: &RenderContext<Native>,
    ) -> Result<Vec<Statement>> {
        let result = fallible.source_name.generated("result")?;
        let success = fallible.source_name.generated("success")?;
        let error = fallible.source_name.generated("error")?;
        Ok(vec![
            Statement::value(result.clone(), call),
            Statement::return_value(Expression::identifier(result).result_fold(
                success.clone(),
                self.fallible_success_expression(
                    Expression::identifier(success),
                    &fallible,
                    context,
                )?,
                error.clone(),
                fallible.error_expression(Expression::identifier(error), context)?,
            )),
        ])
    }

    fn statements(
        &self,
        call: Expression,
        context: &RenderContext<Native>,
    ) -> Result<Vec<Statement>> {
        match &self.conversion {
            ReturnConversion::Void => Ok(vec![Statement::expression(call)]),
            ReturnConversion::DirectPrimitive(primitive) => KotlinPrimitive::new(*primitive)
                .native_argument(call)
                .map(Statement::return_value)
                .map(|statement| vec![statement]),
            ReturnConversion::DirectRecord => Ok(vec![Statement::return_value(
                Record::encode_expression(call)?,
            )]),
            ReturnConversion::DirectEnum { repr } => KotlinPrimitive::new(*repr)
                .native_argument(Expression::property(call, Identifier::parse("value")?))
                .map(Statement::return_value)
                .map(|statement| vec![statement]),
            ReturnConversion::ClassHandle(handle) => handle
                .parameter_argument(call)
                .map(Statement::return_value)
                .map(|statement| vec![statement]),
            ReturnConversion::CallbackHandle(handle) => handle
                .parameter_argument(call)
                .map(Statement::return_value)
                .map(|statement| vec![statement]),
            ReturnConversion::Encoded { codec, source_name } => {
                let result = source_name.generated("result")?;
                let bytes = source_name.generated("bytes")?;
                let write = WireBuffer::new(source_name)?.write_value(
                    codec,
                    Expression::identifier(result.clone()),
                    context,
                )?;
                let (setup, expression, cleanup) = write.into_parts();
                Ok(std::iter::once(Statement::value(result, call))
                    .chain(setup)
                    .chain(std::iter::once(Statement::value(bytes.clone(), expression)))
                    .chain(cleanup)
                    .chain(std::iter::once(Statement::return_value(
                        Expression::identifier(bytes),
                    )))
                    .collect())
            }
            ReturnConversion::ScalarOption {
                primitive,
                source_name,
            } => {
                let result = source_name.generated("result")?;
                let bytes = source_name.generated("bytes")?;
                let write = ScalarOption::new(*primitive)
                    .write_value(source_name, Expression::identifier(result.clone()))?;
                let (setup, expression, cleanup) = write.into_parts();
                Ok(std::iter::once(Statement::value(result, call))
                    .chain(setup)
                    .chain(std::iter::once(Statement::value(bytes.clone(), expression)))
                    .chain(cleanup)
                    .chain(std::iter::once(Statement::return_value(
                        Expression::identifier(bytes),
                    )))
                    .collect())
            }
            ReturnConversion::DirectVector(vector) => Ok(vec![Statement::return_value(
                vector.byte_array_expression(call),
            )]),
        }
    }

    fn fallible_success_expression(
        &self,
        value: Expression,
        fallible: &FallibleReturn,
        context: &RenderContext<Native>,
    ) -> Result<Expression> {
        let ReturnConversion::Void = &self.conversion else {
            let success_out = fallible
                .success_out
                .as_ref()
                .ok_or(Error::BrokenBridgeContract {
                    bridge: "jni",
                    invariant: "fallible closure has no success out-pointer",
                })?;
            let (setup, value, cleanup) = self.success_value(value, context)?;
            let write = Statement::expression(Expression::call(
                "Native",
                Identifier::escape(success_out.writer().as_str())?,
                [
                    Expression::identifier(Identifier::escape(success_out.name().as_str())?),
                    value,
                ]
                .into_iter()
                .collect::<ArgumentList>(),
            ));
            return Ok(Expression::run(
                setup
                    .into_iter()
                    .chain(std::iter::once(write))
                    .chain(cleanup)
                    .collect(),
                Self::empty_error_payload(),
            ));
        };
        Ok(Self::empty_error_payload())
    }

    fn success_value(
        &self,
        value: Expression,
        context: &RenderContext<Native>,
    ) -> Result<(Vec<Statement>, Expression, Vec<Statement>)> {
        match &self.conversion {
            ReturnConversion::DirectPrimitive(primitive) => Ok((
                Vec::new(),
                KotlinPrimitive::new(*primitive).native_argument(value)?,
                Vec::new(),
            )),
            ReturnConversion::DirectEnum { repr } => Ok((
                Vec::new(),
                KotlinPrimitive::new(*repr)
                    .native_argument(Expression::property(value, Identifier::parse("value")?))?,
                Vec::new(),
            )),
            ReturnConversion::DirectRecord => {
                Ok((Vec::new(), Record::encode_expression(value)?, Vec::new()))
            }
            ReturnConversion::Encoded { codec, source_name } => Ok(WireBuffer::new(source_name)?
                .write_value(codec, value, context)?
                .into_parts()),
            ReturnConversion::ScalarOption {
                primitive,
                source_name,
            } => Ok(ScalarOption::new(*primitive)
                .write_value(source_name, value)?
                .into_parts()),
            ReturnConversion::DirectVector(vector) => {
                Ok((Vec::new(), vector.byte_array_expression(value), Vec::new()))
            }
            ReturnConversion::ClassHandle(_)
            | ReturnConversion::CallbackHandle(_)
            | ReturnConversion::Void => Err(Error::UnsupportedTarget {
                target: KOTLIN_TARGET,
                shape: "fallible closure success return",
            }),
        }
    }

    fn empty_error_payload() -> Expression {
        Expression::construct(
            TypeName::byte_array(false),
            [Expression::integer(0)]
                .into_iter()
                .collect::<ArgumentList>(),
        )
    }
}

impl FallibleReturn<'_> {
    fn error_expression(
        &self,
        value: Expression,
        context: &RenderContext<Native>,
    ) -> Result<Expression> {
        let bytes = self.source_name.generated("error_bytes")?;
        let write =
            WireBuffer::new(&self.source_name)?.write_value(self.error_codec, value, context)?;
        let (setup, expression, cleanup) = write.into_parts();
        Ok(Expression::run(
            setup
                .into_iter()
                .chain(std::iter::once(Statement::value(bytes.clone(), expression)))
                .chain(cleanup)
                .collect(),
            Expression::identifier(bytes),
        ))
    }
}

struct ClosureRegistrations<'bridge> {
    registrations: &'bridge [ClosureRegistration],
}

impl<'bridge> ClosureRegistrations<'bridge> {
    fn new(registrations: &'bridge [ClosureRegistration]) -> Self {
        Self { registrations }
    }

    fn get(
        &self,
        closure: &ClosureParameter<Native, IntoRust>,
    ) -> Result<&'bridge ClosureRegistration> {
        self.registrations
            .iter()
            .find(|registration| registration.signature() == closure.signature())
            .ok_or(Error::BrokenBridgeContract {
                bridge: JNI_BRIDGE,
                invariant: "closure parameter has no JNI registration",
            })
    }
}

struct ClosureSource;

impl ClosureSource {
    fn from_declaration<'binding>(
        declaration: boltffi_binding::DeclarationRef<'binding, Native>,
    ) -> Box<dyn Iterator<Item = &'binding ClosureParameter<Native, IntoRust>> + 'binding> {
        match declaration {
            boltffi_binding::DeclarationRef::Function(function) => {
                Self::from_callable(function.callable())
            }
            boltffi_binding::DeclarationRef::Class(class) => Box::new(
                class
                    .initializers()
                    .iter()
                    .flat_map(|initializer| Self::from_callable(initializer.callable()))
                    .chain(
                        class
                            .methods()
                            .iter()
                            .flat_map(|method| Self::from_callable(method.callable())),
                    ),
            ),
            _ => Box::new(std::iter::empty()),
        }
    }

    fn from_callable<'binding>(
        callable: &'binding ExportedCallable<Native>,
    ) -> Box<dyn Iterator<Item = &'binding ClosureParameter<Native, IntoRust>> + 'binding> {
        Box::new(callable.params().iter().filter_map(|parameter| {
            let IncomingParam::Closure(closure) = parameter.payload() else {
                return None;
            };
            Some(closure)
        }))
    }
}
