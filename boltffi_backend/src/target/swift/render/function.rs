use askama::Template;

use boltffi_binding::{
    ClosureReturn, DirectValueType, DirectVectorElementType, Direction, ErrorDecl, ExecutionDecl,
    ExportedCallable, ExportedMethodDecl, FunctionDecl, HandlePresence, HandleTarget, IntoRust,
    Native, NativeSymbol, OutOfRust, ParamDecl, ParamPlanRender, Primitive, Receive,
    ReturnPlanRender, ReturnValueSlot, Surface, TypeRef,
};

use crate::{
    bridge::c::{CBridgeContract, Function as CFunction},
    core::{Emitted, Error, RenderContext, Result},
    target::swift::{
        SwiftHost,
        name_style::Name,
        render::{Documentation, SwiftType},
        syntax::{ArgumentList, Expression, Identifier, TypeName},
    },
};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Function {
    documentation: Documentation,
    name: Identifier,
    parameters: Vec<Parameter>,
    body: String,
    returns: ReturnSignature,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AssociatedFunction {
    documentation: Documentation,
    static_: bool,
    name: Identifier,
    parameters: Vec<Parameter>,
    body: String,
    returns: ReturnSignature,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Receiver {
    argument: Expression,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Parameter {
    name: Identifier,
    ty: TypeName,
    argument: Expression,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct Return {
    ty: Option<TypeName>,
    conversion: ReturnConversion,
}

#[derive(Clone, Debug, Eq, PartialEq)]
enum ReturnConversion {
    Direct,
    FromC(TypeName),
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct Invocation {
    symbol: String,
    parameters: Vec<Parameter>,
    arguments: Vec<Expression>,
    returns: Return,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ReturnSignature(Option<TypeName>);

#[derive(Template)]
#[template(path = "target/swift/function.swift", escape = "none")]
struct FunctionTemplate<'a> {
    function: &'a Function,
}

impl Function {
    pub fn from_declaration(
        decl: &FunctionDecl<Native>,
        bridge: &CBridgeContract,
        context: &RenderContext<Native>,
    ) -> Result<Self> {
        let invocation =
            Invocation::from_callable(decl.symbol(), decl.callable(), None, bridge, context)?;
        let (parameters, body, returns) = invocation.into_rendered("    ");
        Ok(Self {
            documentation: Documentation::new(decl.meta().doc(), ""),
            name: Name::new(decl.name()).function()?,
            parameters,
            body,
            returns,
        })
    }

    pub fn render(&self) -> Result<Emitted> {
        let mut source = FunctionTemplate { function: self }.render()?;
        source.push_str("\n\n");
        Ok(Emitted::primary(source))
    }

    fn name(&self) -> &Identifier {
        &self.name
    }

    fn documentation(&self) -> &Documentation {
        &self.documentation
    }

    fn parameters(&self) -> &[Parameter] {
        &self.parameters
    }

    fn body(&self) -> &str {
        &self.body
    }

    fn returns(&self) -> &ReturnSignature {
        &self.returns
    }
}

impl AssociatedFunction {
    pub fn from_initializer(
        initializer: &boltffi_binding::InitializerDecl<Native>,
        bridge: &CBridgeContract,
        context: &RenderContext<Native>,
    ) -> Result<Self> {
        Self::from_parts(
            Documentation::new(initializer.meta().doc(), "    "),
            true,
            Name::new(initializer.name()).function()?,
            Invocation::from_callable(
                initializer.symbol(),
                initializer.callable(),
                None,
                bridge,
                context,
            )?,
        )
    }

    pub fn from_method(
        method: &ExportedMethodDecl<Native, NativeSymbol>,
        receiver: Option<Receiver>,
        bridge: &CBridgeContract,
        context: &RenderContext<Native>,
    ) -> Result<Self> {
        let static_ = receiver.is_none();
        Self::from_parts(
            Documentation::new(method.meta().doc(), "    "),
            static_,
            Name::new(method.name()).function()?,
            Invocation::from_callable(
                method.target(),
                method.callable(),
                receiver,
                bridge,
                context,
            )?,
        )
    }

    pub fn documentation(&self) -> &Documentation {
        &self.documentation
    }

    pub fn static_keyword(&self) -> &str {
        match self.static_ {
            true => "static ",
            false => "",
        }
    }

    pub fn name(&self) -> &Identifier {
        &self.name
    }

    pub fn parameters(&self) -> &[Parameter] {
        &self.parameters
    }

    pub fn body(&self) -> &str {
        &self.body
    }

    pub fn returns(&self) -> &ReturnSignature {
        &self.returns
    }

    fn from_parts(
        documentation: Documentation,
        static_: bool,
        name: Identifier,
        invocation: Invocation,
    ) -> Result<Self> {
        let (parameters, body, returns) = invocation.into_rendered("        ");
        Ok(Self {
            documentation,
            static_,
            name,
            parameters,
            body,
            returns,
        })
    }
}

impl Receiver {
    pub fn direct() -> Self {
        Self {
            argument: Expression::member("self", "cValue"),
        }
    }

    fn argument(self) -> Expression {
        self.argument
    }
}

impl Invocation {
    fn from_callable(
        symbol: &NativeSymbol,
        callable: &ExportedCallable<Native>,
        receiver: Option<Receiver>,
        bridge: &CBridgeContract,
        context: &RenderContext<Native>,
    ) -> Result<Self> {
        Self::check_execution(callable)?;
        Self::check_error(callable)?;
        Self::check_receiver(callable, receiver.as_ref())?;
        let c_function = Self::c_function(symbol, bridge)?;
        let parameters = callable
            .params()
            .iter()
            .map(|parameter| Parameter::from_decl(parameter, context))
            .collect::<Result<Vec<_>>>()?;
        let arguments = receiver
            .into_iter()
            .map(Receiver::argument)
            .chain(
                parameters
                    .iter()
                    .map(|parameter| parameter.argument().clone()),
            )
            .collect::<Vec<_>>();
        let returns = callable
            .returns()
            .plan()
            .render_with(&mut ReturnPlan { context })?;
        Ok(Self {
            symbol: c_function.name().to_owned(),
            parameters,
            arguments,
            returns,
        })
    }

    fn into_rendered(self, indent: &str) -> (Vec<Parameter>, String, ReturnSignature) {
        let body = self.returns.body(self.call(), indent);
        let returns = self.returns.signature();
        (self.parameters, body, returns)
    }

    fn call(&self) -> Expression {
        Expression::call(
            &self.symbol,
            self.arguments.iter().cloned().collect::<ArgumentList>(),
        )
    }

    fn c_function<'bridge>(
        symbol: &NativeSymbol,
        bridge: &'bridge CBridgeContract,
    ) -> Result<&'bridge CFunction> {
        bridge
            .functions()
            .iter()
            .find(|function| function.name() == symbol.name().as_str())
            .ok_or(Error::BrokenBridgeContract {
                bridge: SwiftHost::TARGET,
                invariant: "missing C function for Swift function",
            })
    }

    fn check_execution(callable: &ExportedCallable<Native>) -> Result<()> {
        match callable.execution() {
            ExecutionDecl::Synchronous(_) => Ok(()),
            ExecutionDecl::Asynchronous(_) => Err(SwiftHost::unsupported("async function")),
            _ => Err(SwiftHost::unsupported("unknown function execution")),
        }
    }

    fn check_error(callable: &ExportedCallable<Native>) -> Result<()> {
        match callable.error() {
            ErrorDecl::None(_) => Ok(()),
            _ => Err(SwiftHost::unsupported("fallible function")),
        }
    }

    fn check_receiver(
        callable: &ExportedCallable<Native>,
        receiver: Option<&Receiver>,
    ) -> Result<()> {
        match (callable.receiver(), receiver) {
            (None, None) => Ok(()),
            (Some(Receive::ByValue | Receive::ByRef), Some(_)) => Ok(()),
            (Some(Receive::ByMutRef), Some(_)) => {
                Err(SwiftHost::unsupported("mutable value receiver"))
            }
            _ => Err(SwiftHost::unsupported("method receiver mismatch")),
        }
    }
}

impl Parameter {
    fn from_decl(
        decl: &ParamDecl<Native, IntoRust>,
        context: &RenderContext<Native>,
    ) -> Result<Self> {
        decl.payload()
            .as_value()
            .ok_or(SwiftHost::unsupported("closure parameter"))?
            .render_with(&mut ParameterPlan {
                name: Name::new(decl.name()).parameter()?,
                context,
            })
    }

    pub fn signature(&self) -> String {
        format!("{}: {}", self.name, self.ty)
    }

    fn argument(&self) -> &Expression {
        &self.argument
    }
}

struct ParameterPlan<'context, 'bindings> {
    name: Identifier,
    context: &'context RenderContext<'bindings, Native>,
}

impl<'plan, 'context, 'bindings> ParamPlanRender<'plan, Native, IntoRust>
    for ParameterPlan<'context, 'bindings>
{
    type Output = Result<Parameter>;

    fn direct(&mut self, ty: &'plan DirectValueType, receive: Receive) -> Self::Output {
        if receive != Receive::ByValue {
            return Err(SwiftHost::unsupported("borrowed direct parameter"));
        }
        let ty = match ty {
            DirectValueType::Primitive(primitive) => SwiftType::primitive(*primitive)?,
            DirectValueType::Record(record) => {
                return Ok(Parameter {
                    name: self.name.clone(),
                    ty: SwiftType::record(*record, self.context)?,
                    argument: Expression::member(&self.name, "cValue"),
                });
            }
            DirectValueType::Enum(enumeration) => {
                return Ok(Parameter {
                    name: self.name.clone(),
                    ty: SwiftType::enumeration(*enumeration, self.context)?,
                    argument: Expression::member(&self.name, "cValue"),
                });
            }
            _ => return Err(SwiftHost::unsupported("unknown direct parameter")),
        };
        Ok(Parameter {
            name: self.name.clone(),
            ty,
            argument: Expression::new(self.name.to_string()),
        })
    }

    fn encoded(
        &mut self,
        _ty: &'plan TypeRef,
        _codec: &'plan <IntoRust as Direction>::Codec,
        _shape: <Native as Surface>::BufferShape,
        _receive: Receive,
    ) -> Self::Output {
        Err(SwiftHost::unsupported("encoded parameter"))
    }

    fn handle(
        &mut self,
        _target: &'plan HandleTarget,
        _carrier: <Native as Surface>::HandleCarrier,
        _presence: HandlePresence,
        _receive: Receive,
    ) -> Self::Output {
        Err(SwiftHost::unsupported("handle parameter"))
    }

    fn scalar_option(&mut self, _primitive: Primitive) -> Self::Output {
        Err(SwiftHost::unsupported("scalar option parameter"))
    }

    fn direct_vector(&mut self, _element: &'plan DirectVectorElementType) -> Self::Output {
        Err(SwiftHost::unsupported("direct vector parameter"))
    }
}

impl Return {
    fn signature(&self) -> ReturnSignature {
        ReturnSignature(self.ty.clone())
    }

    fn body(&self, call: Expression, indent: &str) -> String {
        match self.ty {
            Some(_) => match &self.conversion {
                ReturnConversion::Direct => format!("{indent}return {call}"),
                ReturnConversion::FromC(ty) => format!("{indent}return {}(fromC: {call})", ty),
            },
            None => format!("{indent}{call}"),
        }
    }
}

impl ReturnSignature {
    pub fn signature(&self) -> String {
        self.0
            .as_ref()
            .map(|ty| format!(" -> {ty}"))
            .unwrap_or_default()
    }
}

struct ReturnPlan<'context, 'bindings> {
    context: &'context RenderContext<'bindings, Native>,
}

impl<'plan, 'context, 'bindings> ReturnPlanRender<'plan, Native, OutOfRust>
    for ReturnPlan<'context, 'bindings>
{
    type Output = Result<Return>;

    fn void(&mut self) -> Self::Output {
        Ok(Return {
            ty: None,
            conversion: ReturnConversion::Direct,
        })
    }

    fn direct(&mut self, slot: ReturnValueSlot, ty: &'plan DirectValueType) -> Self::Output {
        if slot != ReturnValueSlot::ReturnSlot {
            return Err(SwiftHost::unsupported("out pointer return"));
        }
        let ty = match ty {
            DirectValueType::Primitive(primitive) => SwiftType::primitive(*primitive)?,
            DirectValueType::Record(record) => {
                let ty = SwiftType::record(*record, self.context)?;
                return Ok(Return {
                    ty: Some(ty.clone()),
                    conversion: ReturnConversion::FromC(ty),
                });
            }
            DirectValueType::Enum(enumeration) => {
                let ty = SwiftType::enumeration(*enumeration, self.context)?;
                return Ok(Return {
                    ty: Some(ty.clone()),
                    conversion: ReturnConversion::FromC(ty),
                });
            }
            _ => return Err(SwiftHost::unsupported("unknown direct return")),
        };
        Ok(Return {
            ty: Some(ty),
            conversion: ReturnConversion::Direct,
        })
    }

    fn encoded(
        &mut self,
        _slot: ReturnValueSlot,
        _ty: &'plan TypeRef,
        _codec: &'plan <OutOfRust as Direction>::Codec,
        _shape: <Native as Surface>::BufferShape,
    ) -> Self::Output {
        Err(SwiftHost::unsupported("encoded return"))
    }

    fn handle(
        &mut self,
        _slot: ReturnValueSlot,
        _target: &'plan HandleTarget,
        _carrier: <Native as Surface>::HandleCarrier,
        _presence: HandlePresence,
    ) -> Self::Output {
        Err(SwiftHost::unsupported("handle return"))
    }

    fn scalar_option(&mut self, _primitive: Primitive) -> Self::Output {
        Err(SwiftHost::unsupported("scalar option return"))
    }

    fn direct_vector(&mut self, _element: &'plan DirectVectorElementType) -> Self::Output {
        Err(SwiftHost::unsupported("direct vector return"))
    }

    fn closure(&mut self, _closure: &'plan ClosureReturn<Native, OutOfRust>) -> Self::Output {
        Err(SwiftHost::unsupported("closure return"))
    }
}
