use askama::Template;

use boltffi_binding::{
    ClosureReturn, DirectValueType, DirectVectorElementType, Direction, ErrorDecl, ExecutionDecl,
    FunctionDecl, HandlePresence, HandleTarget, IntoRust, Native, OutOfRust, ParamDecl,
    ParamPlanRender, Primitive, Receive, ReturnPlanRender, ReturnValueSlot, Surface, TypeRef,
};

use crate::{
    bridge::c::CBridgeContract,
    core::{Emitted, Error, Result},
    target::swift::{
        SwiftHost,
        name_style::Name,
        render::SwiftType,
        syntax::{Expression, Identifier, TypeName},
    },
};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Function {
    name: Identifier,
    symbol: String,
    parameters: Vec<Parameter>,
    returns: Return,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct Parameter {
    name: Identifier,
    ty: TypeName,
    argument: Expression,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct Return {
    ty: Option<TypeName>,
}

#[derive(Template)]
#[template(path = "target/swift/function.swift", escape = "none")]
struct FunctionTemplate<'a> {
    function: &'a Function,
}

impl Function {
    pub fn from_declaration(decl: &FunctionDecl<Native>, bridge: &CBridgeContract) -> Result<Self> {
        let symbol = decl.symbol().name().as_str();
        let c_function = bridge
            .functions()
            .iter()
            .find(|function| function.name() == symbol)
            .ok_or(Error::BrokenBridgeContract {
                bridge: "swift",
                invariant: "missing C function for Swift function",
            })?;
        let callable = decl.callable();
        match callable.execution() {
            ExecutionDecl::Synchronous(_) => {}
            ExecutionDecl::Asynchronous(_) => {
                return Err(SwiftHost::unsupported("async function"));
            }
            _ => return Err(SwiftHost::unsupported("unknown function execution")),
        }
        match callable.error() {
            ErrorDecl::None(_) => {}
            _ => return Err(SwiftHost::unsupported("fallible function")),
        }
        let parameters = callable
            .params()
            .iter()
            .map(Parameter::from_decl)
            .collect::<Result<Vec<_>>>()?;
        let returns = callable.returns().plan().render_with(&mut ReturnPlan)?;
        Ok(Self {
            name: Name::new(decl.name()).function()?,
            symbol: c_function.name().to_owned(),
            parameters,
            returns,
        })
    }

    pub fn render(&self) -> Result<Emitted> {
        FunctionTemplate { function: self }
            .render()
            .map(Emitted::primary)
            .map_err(Error::from)
    }

    fn name(&self) -> &Identifier {
        &self.name
    }

    fn symbol(&self) -> &str {
        &self.symbol
    }

    fn parameters(&self) -> &[Parameter] {
        &self.parameters
    }

    fn returns(&self) -> &Return {
        &self.returns
    }
}

impl Parameter {
    fn from_decl(decl: &ParamDecl<Native, IntoRust>) -> Result<Self> {
        decl.payload()
            .as_value()
            .ok_or(SwiftHost::unsupported("closure parameter"))?
            .render_with(&mut ParameterPlan {
                name: Name::new(decl.name()).parameter()?,
            })
    }

    fn signature(&self) -> String {
        format!("{}: {}", self.name, self.ty)
    }

    fn argument(&self) -> &Expression {
        &self.argument
    }
}

struct ParameterPlan {
    name: Identifier,
}

impl<'plan> ParamPlanRender<'plan, Native, IntoRust> for ParameterPlan {
    type Output = Result<Parameter>;

    fn direct(&mut self, ty: &'plan DirectValueType, receive: Receive) -> Self::Output {
        if receive != Receive::ByValue {
            return Err(SwiftHost::unsupported("borrowed direct parameter"));
        }
        let ty = match ty {
            DirectValueType::Primitive(primitive) => SwiftType::primitive(*primitive)?,
            DirectValueType::Record(_) => {
                return Err(SwiftHost::unsupported("direct record parameter"));
            }
            DirectValueType::Enum(_) => {
                return Err(SwiftHost::unsupported("direct enum parameter"));
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
        _carrier: <Native as boltffi_binding::Surface>::HandleCarrier,
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
    fn signature(&self) -> String {
        self.ty
            .as_ref()
            .map(|ty| format!(" -> {ty}"))
            .unwrap_or_default()
    }

    fn call_body(&self, symbol: &str, arguments: &[Parameter]) -> String {
        let arguments = arguments
            .iter()
            .map(|parameter| parameter.argument().to_string())
            .collect::<Vec<_>>()
            .join(", ");
        let call = format!("{symbol}({arguments})");
        match self.ty {
            Some(_) => format!("    return {call}"),
            None => format!("    {call}"),
        }
    }
}

struct ReturnPlan;

impl<'plan> ReturnPlanRender<'plan, Native, OutOfRust> for ReturnPlan {
    type Output = Result<Return>;

    fn void(&mut self) -> Self::Output {
        Ok(Return { ty: None })
    }

    fn direct(&mut self, slot: ReturnValueSlot, ty: &'plan DirectValueType) -> Self::Output {
        if slot != ReturnValueSlot::ReturnSlot {
            return Err(SwiftHost::unsupported("out pointer return"));
        }
        let ty = match ty {
            DirectValueType::Primitive(primitive) => SwiftType::primitive(*primitive)?,
            DirectValueType::Record(_) => {
                return Err(SwiftHost::unsupported("direct record return"));
            }
            DirectValueType::Enum(_) => return Err(SwiftHost::unsupported("direct enum return")),
            _ => return Err(SwiftHost::unsupported("unknown direct return")),
        };
        Ok(Return { ty: Some(ty) })
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
        _carrier: <Native as boltffi_binding::Surface>::HandleCarrier,
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
