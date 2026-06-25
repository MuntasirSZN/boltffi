use askama::Template as AskamaTemplate;
use boltffi_binding::{
    CanonicalName, ClassDecl, ClassId, ExportedMethodDecl, HandlePresence, InitializerDecl, Native,
    NativeSymbol, Receive,
};

use crate::{
    bridge::jni::JniBridgeContract,
    core::{Emitted, Error, RenderContext, Result},
    target::kotlin::{
        name_style::Name,
        render::function::ExportedCall,
        syntax::{Expression, Identifier, Statement, TypeName},
    },
};

const KOTLIN_TARGET: &str = "kotlin";

#[derive(AskamaTemplate)]
#[template(path = "target/kotlin/class.kt", escape = "none")]
struct ClassTemplate {
    class: Class,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Class {
    name: TypeName,
    release: Identifier,
    initializers: Vec<Initializer>,
    static_methods: Vec<ExportedCall>,
    instance_methods: Vec<ExportedCall>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Initializer {
    call: ExportedCall,
}

impl Class {
    pub fn from_declaration(
        decl: &ClassDecl<Native>,
        bridge: &JniBridgeContract,
        context: &RenderContext<Native>,
    ) -> Result<Self> {
        Ok(Self {
            name: Self::type_name(decl.name())?,
            release: Identifier::escape(decl.release().name().as_str())?,
            initializers: decl
                .initializers()
                .iter()
                .map(|initializer| Initializer::from_declaration(initializer, bridge, context))
                .collect::<Result<Vec<_>>>()?,
            static_methods: Self::methods(decl.methods(), None, bridge, context)?,
            instance_methods: Self::methods(
                decl.methods(),
                Some(Expression::property("this", Identifier::parse("handle")?)),
                bridge,
                context,
            )?,
        })
    }

    pub fn render(self) -> Result<Emitted> {
        Ok(Emitted::primary(ClassTemplate { class: self }.render()?))
    }

    pub fn type_name_from_id(id: ClassId, context: &RenderContext<Native>) -> Result<TypeName> {
        context
            .class(id)
            .ok_or(Error::BrokenBridgeContract {
                bridge: KOTLIN_TARGET,
                invariant: "class handle target has no class declaration",
            })
            .and_then(|decl| Self::type_name(decl.name()))
    }

    pub fn name(&self) -> &TypeName {
        &self.name
    }

    pub fn release(&self) -> &Identifier {
        &self.release
    }

    pub fn initializers(&self) -> &[Initializer] {
        &self.initializers
    }

    pub fn static_methods(&self) -> &[ExportedCall] {
        &self.static_methods
    }

    pub fn instance_methods(&self) -> &[ExportedCall] {
        &self.instance_methods
    }

    fn type_name(name: &CanonicalName) -> Result<TypeName> {
        Ok(Name::new(name).type_name())
    }

    fn methods(
        methods: &[ExportedMethodDecl<Native, NativeSymbol>],
        receiver: Option<Expression>,
        bridge: &JniBridgeContract,
        context: &RenderContext<Native>,
    ) -> Result<Vec<ExportedCall>> {
        methods
            .iter()
            .filter(|method| method.callable().receiver().is_some() == receiver.is_some())
            .map(|method| {
                let native_prefix = match (method.callable().receiver(), receiver.clone()) {
                    (Some(Receive::ByRef | Receive::ByMutRef | Receive::ByValue), Some(handle)) => {
                        vec![handle]
                    }
                    (None, None) => Vec::new(),
                    _ => {
                        return Err(Error::UnsupportedTarget {
                            target: KOTLIN_TARGET,
                            shape: "class method receiver",
                        });
                    }
                };
                ExportedCall::new(
                    Name::new(method.name()).function()?,
                    method.target(),
                    method.callable(),
                    native_prefix,
                    bridge,
                    context,
                )
            })
            .collect()
    }
}

impl Initializer {
    pub fn from_declaration(
        initializer: &InitializerDecl<Native>,
        bridge: &JniBridgeContract,
        context: &RenderContext<Native>,
    ) -> Result<Self> {
        ExportedCall::new(
            Name::new(initializer.name()).function()?,
            initializer.symbol(),
            initializer.callable(),
            Vec::new(),
            bridge,
            context,
        )
        .map(|call| Self { call })
    }

    pub fn call(&self) -> &ExportedCall {
        &self.call
    }
}

pub struct ClassHandle {
    ty: TypeName,
    presence: HandlePresence,
}

impl ClassHandle {
    pub fn new(
        id: ClassId,
        presence: HandlePresence,
        context: &RenderContext<Native>,
    ) -> Result<Self> {
        Ok(Self {
            ty: Class::type_name_from_id(id, context)?,
            presence,
        })
    }

    pub fn ty(&self) -> Result<TypeName> {
        match self.presence {
            HandlePresence::Required => Ok(self.ty.clone()),
            HandlePresence::Nullable => Ok(self.ty.clone().nullable()),
            _ => Err(Error::UnsupportedTarget {
                target: KOTLIN_TARGET,
                shape: "unknown class handle presence",
            }),
        }
    }

    pub fn parameter_argument(&self, value: Expression) -> Result<Expression> {
        let handle = Identifier::parse("handle")?;
        match self.presence {
            HandlePresence::Required => Ok(Expression::property(value, handle)),
            HandlePresence::Nullable => {
                Ok(Expression::safe_property(value, handle).or_else(Expression::long(0)))
            }
            _ => Err(Error::UnsupportedTarget {
                target: KOTLIN_TARGET,
                shape: "unknown class handle presence",
            }),
        }
    }

    pub fn value_expression(&self, value: Expression) -> Result<Expression> {
        match self.presence {
            HandlePresence::Required => Ok(Expression::construct(
                self.ty.clone(),
                [value].into_iter().collect(),
            )),
            HandlePresence::Nullable => Ok(Expression::conditional(
                value.clone().equal(Expression::long(0)),
                Expression::null(),
                Expression::construct(self.ty.clone(), [value].into_iter().collect()),
            )),
            _ => Err(Error::UnsupportedTarget {
                target: KOTLIN_TARGET,
                shape: "unknown class handle presence",
            }),
        }
    }

    pub fn value_statements(&self, value: Expression) -> Result<Vec<Statement>> {
        match self.presence {
            HandlePresence::Required => self
                .value_expression(value)
                .map(Statement::expression)
                .map(|statement| vec![statement]),
            HandlePresence::Nullable => {
                let raw_handle = Identifier::parse("__boltffi_handle")?;
                Ok(vec![
                    Statement::value(raw_handle, value),
                    Statement::expression(self.value_expression(Expression::identifier(
                        Identifier::parse("__boltffi_handle")?,
                    ))?),
                ])
            }
            _ => Err(Error::UnsupportedTarget {
                target: KOTLIN_TARGET,
                shape: "unknown class handle presence",
            }),
        }
    }
}
