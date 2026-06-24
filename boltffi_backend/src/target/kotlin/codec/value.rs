use boltffi_binding::{BinderId, FieldKey, ValueRef, ValueRoot};

use crate::{
    core::{Error, Result},
    target::kotlin::{
        name_style::Name,
        syntax::{Expression, Identifier},
    },
};

pub struct ValueExpression {
    value: ValueRef,
}

impl ValueExpression {
    pub fn new(value: &ValueRef) -> Self {
        Self {
            value: value.clone(),
        }
    }

    pub fn binder(binder: BinderId) -> Result<Identifier> {
        Identifier::parse(format!("__boltffi_value_{}", binder.raw()))
    }

    pub fn render(self) -> Result<Expression> {
        let root = match self.value.root() {
            ValueRoot::SelfValue => Identifier::parse("value").map(Expression::identifier)?,
            ValueRoot::Named(name) | ValueRoot::Local(name) => {
                Name::new(name).parameter().map(Expression::identifier)?
            }
            ValueRoot::Binder(binder) => Expression::identifier(Self::binder(*binder)?),
            _ => {
                return Err(Error::UnsupportedTarget {
                    target: "kotlin",
                    shape: "unknown codec value root",
                });
            }
        };
        self.value.path().iter().try_fold(root, Self::field)
    }

    fn field(expression: Expression, field: &FieldKey) -> Result<Expression> {
        match field {
            FieldKey::Named(name) => Name::new(name)
                .parameter()
                .map(|field| Expression::property(expression, field)),
            _ => Err(Error::UnsupportedTarget {
                target: "kotlin",
                shape: "unknown codec value field",
            }),
        }
    }
}
