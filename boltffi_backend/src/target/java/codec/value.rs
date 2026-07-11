use boltffi_binding::{BinderId, FieldKey, ValueRef, ValueRoot};

use crate::{
    core::Result,
    target::java::{
        JavaHost, JavaVersion,
        name_style::Name,
        syntax::{ArgumentList, Expression, Identifier},
    },
};

pub struct ValueExpression {
    value: ValueRef,
    current: Expression,
    member_access: ValueMemberAccess,
    version: JavaVersion,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ValueMemberAccess {
    Accessor,
    Field,
}

impl ValueExpression {
    pub fn new(value: &ValueRef, version: JavaVersion) -> Self {
        Self {
            value: value.clone(),
            current: Expression::identifier(Identifier::known("value")),
            member_access: ValueMemberAccess::Accessor,
            version,
        }
    }

    pub fn binder(binder: BinderId, version: JavaVersion) -> Result<Identifier> {
        Identifier::parse_for(format!("__boltffi_value_{}", binder.raw()), version)
    }

    pub fn current(mut self, current: Expression) -> Self {
        self.current = current;
        self
    }

    pub fn member_access(mut self, member_access: ValueMemberAccess) -> Self {
        self.member_access = member_access;
        self
    }

    pub fn render(self) -> Result<Expression> {
        let self_value = matches!(self.value.root(), ValueRoot::SelfValue);
        let root = match self.value.root() {
            ValueRoot::SelfValue => self.current,
            ValueRoot::Named(name) | ValueRoot::Local(name) => Name::new(name)
                .parameter(self.version)
                .map(Expression::identifier)?,
            ValueRoot::Binder(binder) => {
                Expression::identifier(Self::binder(*binder, self.version)?)
            }
            _ => return Err(JavaHost::unsupported("unknown codec value root")),
        };
        self.value
            .path()
            .iter()
            .enumerate()
            .try_fold(root, |value, (depth, field)| {
                let field = match field {
                    FieldKey::Named(name) => Name::new(name).parameter(self.version)?,
                    FieldKey::Position(position) => {
                        Identifier::parse_for(format!("field{position}"), self.version)?
                    }
                    _ => return Err(JavaHost::unsupported("unknown codec value field")),
                };
                match self_value && depth == 0 && self.member_access == ValueMemberAccess::Field {
                    true => Ok(value.member(field)),
                    false => Ok(value.call(field, ArgumentList::default())),
                }
            })
    }
}
