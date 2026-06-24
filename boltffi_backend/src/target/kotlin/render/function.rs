use askama::Template as AskamaTemplate;
use boltffi_binding::{ExecutionDecl, FunctionDecl, IncomingParam, Native, ParamPlan};

use crate::{
    core::{Emitted, Error, RenderContext, Result},
    target::kotlin::{
        name_style::Name,
        render::{
            native::NativeCall,
            type_name::{ParameterType, ReturnType},
        },
        syntax::{Identifier, Statement, TypeName},
    },
};

const KOTLIN_TARGET: &str = "kotlin";

#[derive(AskamaTemplate)]
#[template(path = "target/kotlin/function.kt", escape = "none")]
struct FunctionTemplate {
    function: Function,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Function {
    name: Identifier,
    parameters: Vec<Parameter>,
    returns: Option<TypeName>,
    body: Vec<Statement>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Parameter {
    name: Identifier,
    ty: TypeName,
}

impl Function {
    pub fn from_declaration(
        decl: &FunctionDecl<Native>,
        _context: &RenderContext<Native>,
    ) -> Result<Self> {
        if !matches!(decl.callable().execution(), ExecutionDecl::Synchronous(_)) {
            return Err(Error::UnsupportedTarget {
                target: KOTLIN_TARGET,
                shape: "async function",
            });
        }

        let parameters = decl
            .callable()
            .params()
            .iter()
            .map(Parameter::from_declaration)
            .collect::<Result<Vec<_>>>()?;
        let returns = decl
            .callable()
            .returns()
            .plan()
            .render_with(&mut ReturnType)?;
        let call = NativeCall::new(
            Identifier::escape(decl.symbol().name().as_str())?,
            parameters
                .iter()
                .map(|parameter| parameter.name().clone())
                .collect(),
        );
        let body = match returns.is_some() {
            true => vec![Statement::return_value(call.expression())],
            false => vec![Statement::expression(call.expression())],
        };
        Ok(Self {
            name: Name::new(decl.name()).function()?,
            parameters,
            returns,
            body,
        })
    }

    pub fn render(self) -> Result<Emitted> {
        Ok(Emitted::primary(
            FunctionTemplate { function: self }.render()?,
        ))
    }

    pub fn name(&self) -> &Identifier {
        &self.name
    }

    pub fn parameters(&self) -> &[Parameter] {
        &self.parameters
    }

    pub fn returns(&self) -> Option<&TypeName> {
        self.returns.as_ref()
    }

    pub fn body(&self) -> &[Statement] {
        &self.body
    }
}

impl Parameter {
    pub fn from_declaration(
        parameter: &boltffi_binding::ParamDecl<Native, boltffi_binding::IntoRust>,
    ) -> Result<Self> {
        let IncomingParam::Value(plan) = parameter.payload() else {
            return Err(Error::UnsupportedTarget {
                target: KOTLIN_TARGET,
                shape: "closure function parameter",
            });
        };
        Ok(Self {
            name: Name::new(parameter.name()).parameter()?,
            ty: Self::type_name(plan)?,
        })
    }

    pub fn name(&self) -> &Identifier {
        &self.name
    }

    pub fn ty(&self) -> &TypeName {
        &self.ty
    }

    fn type_name(plan: &ParamPlan<Native, boltffi_binding::IntoRust>) -> Result<TypeName> {
        plan.render_with(&mut ParameterType)
    }
}
