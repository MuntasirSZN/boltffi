use boltffi_binding::{BinderId, FieldKey, ValueRef, ValueRoot};

use crate::{
    core::{Error, Result},
    target::python::{
        name_style::Name,
        syntax::{Expression, Identifier, Literal},
    },
};

pub struct ValueExpression {
    value: ValueRef,
    position_fields: PositionFieldAccess,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PositionFieldAccess {
    Attribute,
    Subscript,
}

impl ValueExpression {
    pub fn with_position_fields(value: &ValueRef, position_fields: PositionFieldAccess) -> Self {
        Self {
            value: value.clone(),
            position_fields,
        }
    }

    pub fn root(value: &ValueRef) -> Result<Expression> {
        match value.root() {
            ValueRoot::SelfValue => Identifier::parse("self").map(Expression::identifier),
            ValueRoot::Named(name) | ValueRoot::Local(name) => {
                Name::new(name).function().map(Expression::identifier)
            }
            ValueRoot::Binder(binder) => Ok(Expression::identifier(Self::binder(*binder)?)),
            _ => Err(Error::UnsupportedTarget {
                target: "python",
                shape: "unknown codec value root",
            }),
        }
    }

    pub fn binder(binder: BinderId) -> Result<Identifier> {
        Identifier::parse(format!("__boltffi_value_{}", binder.raw()))
    }

    pub fn render(self) -> Result<Expression> {
        let root = Self::root(&self.value)?;
        self.value
            .path()
            .iter()
            .try_fold(root, |expression, field| {
                Self::field_with_position_fields(expression, field, self.position_fields)
            })
    }

    pub fn field_with_position_fields(
        expression: Expression,
        field: &FieldKey,
        position_fields: PositionFieldAccess,
    ) -> Result<Expression> {
        Ok(match field {
            FieldKey::Named(name) => Expression::attribute(expression, Name::new(name).function()?),
            FieldKey::Position(position) => position_fields.expression(expression, *position)?,
            _ => {
                return Err(Error::UnsupportedTarget {
                    target: "python",
                    shape: "unknown codec value field",
                });
            }
        })
    }
}

impl PositionFieldAccess {
    fn expression(self, expression: Expression, position: u32) -> Result<Expression> {
        Ok(match self {
            Self::Attribute => Expression::attribute(expression, Name::position_field(position)?),
            Self::Subscript => Expression::subscript(
                expression,
                Expression::literal(Literal::integer(i128::from(position))),
            ),
        })
    }
}
