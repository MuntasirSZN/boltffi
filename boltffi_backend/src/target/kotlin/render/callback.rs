use askama::Template as AskamaTemplate;
use boltffi_binding::{
    CallbackDecl, CallbackId, ClosureReturn, DirectValueType, DirectVectorElementType, Direction,
    EnumId, ErrorChannel, ExecutionDecl, HandlePresence, HandleTarget, ImportedMethodDecl,
    IntoRust, Native, OutOfRust, OutgoingParam, ParamDecl, ParamPlanRender, Primitive,
    ReturnPlanRender, ReturnValueSlot, Surface, TypeRef, VTableSlot,
};

use crate::{
    bridge::jni::{CallbackMethod as JniCallbackMethod, CallbackRegistration, JniBridgeContract},
    core::{Emitted, Error, RenderContext, Result},
    target::kotlin::{
        name_style::Name,
        primitive::KotlinPrimitive,
        render::enumeration::Enumeration,
        syntax::{ArgumentList, Expression, Identifier, Statement, TypeName},
    },
};

const KOTLIN_TARGET: &str = "kotlin";

#[derive(AskamaTemplate)]
#[template(path = "target/kotlin/callback.kt", escape = "none")]
struct CallbackTemplate {
    callback: Callback,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Callback {
    name: TypeName,
    map_name: TypeName,
    callbacks_name: TypeName,
    bridge_name: TypeName,
    methods: Vec<Method>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Method {
    name: Identifier,
    jvm_name: Identifier,
    public_parameters: Vec<Parameter>,
    jvm_parameters: Vec<Parameter>,
    public_return: Option<TypeName>,
    jvm_return: Option<TypeName>,
    call_return: Vec<Statement>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Parameter {
    name: Identifier,
    ty: TypeName,
}

pub struct CallbackHandle {
    ty: TypeName,
    bridge: TypeName,
    presence: HandlePresence,
}

struct CallbackRegistrationSet<'bridge> {
    registrations: &'bridge [CallbackRegistration],
}

struct ParameterRender<'context> {
    name: Identifier,
    context: &'context RenderContext<'context, Native>,
}

struct ReturnRender<'context> {
    context: &'context RenderContext<'context, Native>,
}

struct ReturnValue {
    public_ty: Option<TypeName>,
    jvm_ty: Option<TypeName>,
    conversion: ReturnConversion,
}

enum ReturnConversion {
    Void,
    Direct(DirectValueType),
    DirectEnum { repr: Primitive },
}

impl Callback {
    pub fn from_declaration(
        decl: &CallbackDecl<Native>,
        bridge: &JniBridgeContract,
        context: &RenderContext<Native>,
    ) -> Result<Self> {
        let registration = CallbackRegistrationSet::new(bridge.callbacks()).get(decl)?;
        let name = Name::new(decl.name()).type_name();
        if decl.protocol().vtable().methods().len() != registration.methods().len() {
            return Err(Error::BrokenBridgeContract {
                bridge: "jni",
                invariant: "callback declaration method count does not match JNI registration",
            });
        }
        let methods = decl
            .protocol()
            .vtable()
            .methods()
            .iter()
            .zip(registration.methods())
            .map(|(source, method)| Method::from_declaration(source, method, context))
            .collect::<Result<Vec<_>>>()?;
        Ok(Self {
            map_name: TypeName::new(format!("{name}Map")),
            callbacks_name: TypeName::new(format!("{name}Callbacks")),
            bridge_name: TypeName::new(format!("{name}Bridge")),
            name,
            methods,
        })
    }

    pub fn render(self) -> Result<Emitted> {
        Ok(Emitted::primary(
            CallbackTemplate { callback: self }.render()?,
        ))
    }

    pub fn name(&self) -> &TypeName {
        &self.name
    }

    pub fn map_name(&self) -> &TypeName {
        &self.map_name
    }

    pub fn callbacks_name(&self) -> &TypeName {
        &self.callbacks_name
    }

    pub fn bridge_name(&self) -> &TypeName {
        &self.bridge_name
    }

    pub fn methods(&self) -> &[Method] {
        &self.methods
    }
}

impl Method {
    pub fn name(&self) -> &Identifier {
        &self.name
    }

    pub fn jvm_name(&self) -> &Identifier {
        &self.jvm_name
    }

    pub fn public_parameters(&self) -> &[Parameter] {
        &self.public_parameters
    }

    pub fn jvm_parameters(&self) -> &[Parameter] {
        &self.jvm_parameters
    }

    pub fn public_return(&self) -> Option<&TypeName> {
        self.public_return.as_ref()
    }

    pub fn jvm_return(&self) -> Option<&TypeName> {
        self.jvm_return.as_ref()
    }

    pub fn call_return(&self) -> &[Statement] {
        &self.call_return
    }
}

impl Method {
    fn from_declaration(
        source: &ImportedMethodDecl<Native, VTableSlot>,
        method: &JniCallbackMethod,
        context: &RenderContext<Native>,
    ) -> Result<Self> {
        let callable = source.callable();
        if !matches!(callable.error().channel(), ErrorChannel::None) {
            return Err(Error::UnsupportedTarget {
                target: KOTLIN_TARGET,
                shape: "callback method error return",
            });
        }
        if !matches!(callable.execution(), ExecutionDecl::Synchronous(_)) {
            return Err(Error::UnsupportedTarget {
                target: KOTLIN_TARGET,
                shape: "async callback method",
            });
        }
        if source.target().as_str() != method.method().as_str() {
            return Err(Error::BrokenBridgeContract {
                bridge: "jni",
                invariant: "callback declaration method does not match JNI registration method",
            });
        }
        let parameters = callable
            .params()
            .iter()
            .map(|parameter| MethodParameter::from_declaration(parameter, context))
            .collect::<Result<Vec<_>>>()?;
        let return_value = callable
            .returns()
            .plan()
            .render_with(&mut ReturnRender { context })?;
        let implementation = Expression::identifier(Identifier::parse("impl")?);
        let interface_call = Expression::call(
            implementation,
            Name::new(source.name()).function()?,
            parameters
                .iter()
                .map(|parameter| parameter.call_argument.clone())
                .collect::<ArgumentList>(),
        );
        Ok(Self {
            name: Name::new(source.name()).function()?,
            jvm_name: Identifier::escape(method.method().as_str())?,
            public_parameters: parameters
                .iter()
                .map(|parameter| parameter.public.clone())
                .collect(),
            jvm_parameters: parameters
                .into_iter()
                .map(|parameter| parameter.jvm)
                .collect(),
            public_return: return_value.public_ty.clone(),
            jvm_return: return_value.jvm_ty.clone(),
            call_return: return_value.statements(interface_call)?,
        })
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

impl CallbackHandle {
    pub fn new(
        id: CallbackId,
        presence: HandlePresence,
        context: &RenderContext<Native>,
    ) -> Result<Self> {
        let callback = context.callback(id).ok_or(Error::UnsupportedTarget {
            target: KOTLIN_TARGET,
            shape: "callback handle without declaration",
        })?;
        let ty = Name::new(callback.name()).type_name();
        Ok(Self {
            bridge: TypeName::new(format!("{ty}Bridge")),
            ty,
            presence,
        })
    }

    pub fn ty(&self) -> Result<TypeName> {
        match self.presence {
            HandlePresence::Required => Ok(self.ty.clone()),
            HandlePresence::Nullable => Ok(self.ty.clone().nullable()),
            _ => Err(Error::UnsupportedTarget {
                target: KOTLIN_TARGET,
                shape: "unknown callback handle presence",
            }),
        }
    }

    pub fn parameter_argument(&self, value: Expression) -> Result<Expression> {
        let create = Identifier::parse("create")?;
        let required = Expression::call(
            self.bridge.clone(),
            create.clone(),
            [value.clone()].into_iter().collect::<ArgumentList>(),
        );
        Ok(match self.presence {
            HandlePresence::Required => required,
            HandlePresence::Nullable => value.let_or_else(
                Identifier::parse("__boltffi_callback")?,
                Expression::call(
                    self.bridge.clone(),
                    create,
                    [Expression::identifier(Identifier::parse(
                        "__boltffi_callback",
                    )?)]
                    .into_iter()
                    .collect::<ArgumentList>(),
                ),
                Expression::long(0),
            ),
            _ => {
                return Err(Error::UnsupportedTarget {
                    target: KOTLIN_TARGET,
                    shape: "unknown callback handle presence",
                });
            }
        })
    }
}

impl<'bridge> CallbackRegistrationSet<'bridge> {
    fn new(registrations: &'bridge [CallbackRegistration]) -> Self {
        Self { registrations }
    }

    fn get(&self, decl: &CallbackDecl<Native>) -> Result<&'bridge CallbackRegistration> {
        self.registrations
            .iter()
            .find(|registration| {
                registration.register().as_str() == decl.protocol().register().name().as_str()
            })
            .ok_or(Error::BrokenBridgeContract {
                bridge: "jni",
                invariant: "callback declaration has no JNI registration",
            })
    }
}

struct MethodParameter {
    public: Parameter,
    jvm: Parameter,
    call_argument: Expression,
}

impl MethodParameter {
    fn from_declaration(
        parameter: &ParamDecl<Native, OutOfRust>,
        context: &RenderContext<Native>,
    ) -> Result<Self> {
        let OutgoingParam::Value(plan) = parameter.payload() else {
            return Err(Error::UnsupportedTarget {
                target: KOTLIN_TARGET,
                shape: "callback method closure parameter",
            });
        };
        let source_name = Name::new(parameter.name());
        let name = source_name.parameter()?;
        plan.render_with(&mut ParameterRender { name, context })
    }
}

impl<'plan> ParamPlanRender<'plan, Native, OutOfRust> for ParameterRender<'_> {
    type Output = Result<MethodParameter>;

    fn direct(
        &mut self,
        ty: &'plan DirectValueType,
        _receive: <OutOfRust as Direction>::Receive,
    ) -> Self::Output {
        match ty {
            DirectValueType::Primitive(primitive) => {
                let value = Expression::identifier(self.name.clone());
                Ok(MethodParameter {
                    public: Parameter {
                        name: self.name.clone(),
                        ty: KotlinPrimitive::new(*primitive).api_type()?,
                    },
                    jvm: Parameter {
                        name: self.name.clone(),
                        ty: KotlinPrimitive::new(*primitive).native_type()?,
                    },
                    call_argument: KotlinPrimitive::new(*primitive).public_return(value)?,
                })
            }
            DirectValueType::Enum(enumeration) => {
                let enumeration = Enumeration::from_id(*enumeration, self.context)?;
                let repr = enumeration.repr()?;
                let value = KotlinPrimitive::new(repr)
                    .public_return(Expression::identifier(self.name.clone()))?;
                Ok(MethodParameter {
                    public: Parameter {
                        name: self.name.clone(),
                        ty: enumeration.name().clone(),
                    },
                    jvm: Parameter {
                        name: self.name.clone(),
                        ty: KotlinPrimitive::new(repr).native_type()?,
                    },
                    call_argument: Expression::call(
                        enumeration.name().clone(),
                        Identifier::parse("fromValue")?,
                        [value].into_iter().collect::<ArgumentList>(),
                    ),
                })
            }
            _ => Err(Error::UnsupportedTarget {
                target: KOTLIN_TARGET,
                shape: "callback method direct parameter",
            }),
        }
    }

    fn encoded(
        &mut self,
        _ty: &'plan TypeRef,
        _codec: &'plan <OutOfRust as Direction>::Codec,
        _shape: <Native as Surface>::BufferShape,
        _receive: <OutOfRust as Direction>::Receive,
    ) -> Self::Output {
        Err(Error::UnsupportedTarget {
            target: KOTLIN_TARGET,
            shape: "callback method encoded parameter",
        })
    }

    fn handle(
        &mut self,
        _target: &'plan HandleTarget,
        _carrier: <Native as Surface>::HandleCarrier,
        _presence: HandlePresence,
        _receive: <OutOfRust as Direction>::Receive,
    ) -> Self::Output {
        Err(Error::UnsupportedTarget {
            target: KOTLIN_TARGET,
            shape: "callback method handle parameter",
        })
    }

    fn scalar_option(&mut self, _primitive: Primitive) -> Self::Output {
        Err(Error::UnsupportedTarget {
            target: KOTLIN_TARGET,
            shape: "callback method optional scalar parameter",
        })
    }

    fn direct_vector(&mut self, _element: &'plan DirectVectorElementType) -> Self::Output {
        Err(Error::UnsupportedTarget {
            target: KOTLIN_TARGET,
            shape: "callback method direct vector parameter",
        })
    }
}

impl<'plan> ReturnPlanRender<'plan, Native, IntoRust> for ReturnRender<'_> {
    type Output = Result<ReturnValue>;

    fn void(&mut self) -> Self::Output {
        Ok(ReturnValue {
            public_ty: None,
            jvm_ty: None,
            conversion: ReturnConversion::Void,
        })
    }

    fn direct(&mut self, slot: ReturnValueSlot, ty: &'plan DirectValueType) -> Self::Output {
        if !matches!(slot, ReturnValueSlot::ReturnSlot) {
            return Err(Error::UnsupportedTarget {
                target: KOTLIN_TARGET,
                shape: "callback method out-pointer return",
            });
        }
        match ty {
            DirectValueType::Primitive(primitive) => Ok(ReturnValue {
                public_ty: Some(KotlinPrimitive::new(*primitive).api_type()?),
                jvm_ty: Some(KotlinPrimitive::new(*primitive).native_type()?),
                conversion: ReturnConversion::Direct(ty.clone()),
            }),
            DirectValueType::Enum(enumeration) => Self::direct_enum(*enumeration, self.context),
            _ => Err(Error::UnsupportedTarget {
                target: KOTLIN_TARGET,
                shape: "callback method direct return",
            }),
        }
    }

    fn encoded(
        &mut self,
        _slot: ReturnValueSlot,
        _ty: &'plan TypeRef,
        _codec: &'plan <IntoRust as Direction>::Codec,
        _shape: <Native as Surface>::BufferShape,
    ) -> Self::Output {
        Err(Error::UnsupportedTarget {
            target: KOTLIN_TARGET,
            shape: "callback method encoded return",
        })
    }

    fn handle(
        &mut self,
        _slot: ReturnValueSlot,
        _target: &'plan HandleTarget,
        _carrier: <Native as Surface>::HandleCarrier,
        _presence: HandlePresence,
    ) -> Self::Output {
        Err(Error::UnsupportedTarget {
            target: KOTLIN_TARGET,
            shape: "callback method handle return",
        })
    }

    fn scalar_option(&mut self, _primitive: Primitive) -> Self::Output {
        Err(Error::UnsupportedTarget {
            target: KOTLIN_TARGET,
            shape: "callback method optional scalar return",
        })
    }

    fn direct_vector(&mut self, _element: &'plan DirectVectorElementType) -> Self::Output {
        Err(Error::UnsupportedTarget {
            target: KOTLIN_TARGET,
            shape: "callback method direct vector return",
        })
    }

    fn closure(&mut self, _closure: &'plan ClosureReturn<Native, IntoRust>) -> Self::Output {
        Err(Error::UnsupportedTarget {
            target: KOTLIN_TARGET,
            shape: "callback method closure return",
        })
    }
}

impl ReturnRender<'_> {
    fn direct_enum(enumeration: EnumId, context: &RenderContext<Native>) -> Result<ReturnValue> {
        let enumeration = Enumeration::from_id(enumeration, context)?;
        let repr = enumeration.repr()?;
        Ok(ReturnValue {
            public_ty: Some(enumeration.name().clone()),
            jvm_ty: Some(KotlinPrimitive::new(repr).native_type()?),
            conversion: ReturnConversion::DirectEnum { repr },
        })
    }
}

impl ReturnValue {
    fn statements(&self, call: Expression) -> Result<Vec<Statement>> {
        match &self.conversion {
            ReturnConversion::Void => Ok(vec![Statement::expression(call)]),
            ReturnConversion::Direct(DirectValueType::Primitive(primitive)) => {
                KotlinPrimitive::new(*primitive)
                    .native_argument(call)
                    .map(Statement::return_value)
                    .map(|statement| vec![statement])
            }
            ReturnConversion::DirectEnum { repr, .. } => KotlinPrimitive::new(*repr)
                .native_argument(Expression::property(call, Identifier::parse("value")?))
                .map(Statement::return_value)
                .map(|statement| vec![statement]),
            ReturnConversion::Direct(_) => Err(Error::UnsupportedTarget {
                target: KOTLIN_TARGET,
                shape: "callback method direct return",
            }),
        }
    }
}
