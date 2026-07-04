use boltffi_binding::{BinderId, FieldKey, ValueRef, ValueRoot};

use crate::{
    core::Result,
    target::swift::{
        SwiftHost,
        name_style::Name,
        syntax::{Expression, Identifier},
    },
};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ValueExpression {
    value: ValueRef,
    scope: ValueScope,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ValueScope {
    Current(Expression),
    Fields(Vec<(FieldKey, Expression)>),
}

impl ValueExpression {
    pub fn new(value: &ValueRef, scope: impl Into<ValueScope>) -> Self {
        Self {
            value: value.clone(),
            scope: scope.into(),
        }
    }

    pub fn binder(binder: BinderId) -> Result<Identifier> {
        Identifier::parse(format!("__boltffi_value_{}", binder.raw()))
    }

    pub fn render(self) -> Result<Expression> {
        let root = match self.value.root() {
            ValueRoot::SelfValue => return self.scope.render(self.value.path()),
            ValueRoot::Named(name) | ValueRoot::Local(name) => {
                Expression::identifier(Name::new(name).parameter()?)
            }
            ValueRoot::Binder(binder) => Expression::identifier(Self::binder(*binder)?),
            _ => return Err(SwiftHost::unsupported("unknown codec value root")),
        };
        self.value.path().iter().try_fold(root, Self::field)
    }

    fn field(expression: Expression, field: &FieldKey) -> Result<Expression> {
        match field {
            FieldKey::Named(name) => Ok(Expression::member(expression, Name::new(name).field()?)),
            FieldKey::Position(position) => {
                Ok(Expression::member(expression, format!("field{position}")))
            }
            _ => Err(SwiftHost::unsupported("unknown codec value field")),
        }
    }
}

impl ValueScope {
    pub fn fields(fields: Vec<(FieldKey, Expression)>) -> Self {
        Self::Fields(fields)
    }

    fn render(self, path: &[FieldKey]) -> Result<Expression> {
        match self {
            Self::Current(current) => path.iter().try_fold(current, ValueExpression::field),
            Self::Fields(fields) => Self::render_field(fields, path),
        }
    }

    fn render_field(fields: Vec<(FieldKey, Expression)>, path: &[FieldKey]) -> Result<Expression> {
        match path.split_first() {
            Some((field, rest)) => fields
                .into_iter()
                .find_map(|(key, value)| (key == *field).then_some(value))
                .ok_or(SwiftHost::unsupported("unknown codec payload field"))
                .and_then(|value| rest.iter().try_fold(value, ValueExpression::field)),
            None => Err(SwiftHost::unsupported("whole payload value")),
        }
    }
}

impl From<Expression> for ValueScope {
    fn from(expression: Expression) -> Self {
        Self::Current(expression)
    }
}
