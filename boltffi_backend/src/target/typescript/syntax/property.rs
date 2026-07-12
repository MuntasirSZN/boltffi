use std::fmt;

use crate::core::Result;

use super::{Expression, Identifier};

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub enum PropertyKey {
    Named(Identifier),
    Position(u32),
}

impl PropertyKey {
    pub fn named(identifier: Identifier) -> Self {
        Self::Named(identifier)
    }

    pub fn position(position: u32) -> Self {
        Self::Position(position)
    }

    pub fn access(&self, receiver: Expression) -> Expression {
        match self {
            Self::Named(identifier) => Expression::property(receiver, identifier.clone()),
            Self::Position(position) => Expression::index(receiver, *position),
        }
    }

    pub fn local(&self) -> Result<Identifier> {
        match self {
            Self::Named(identifier) => Ok(identifier.clone()),
            Self::Position(position) => Identifier::parse(format!("__boltffiField{position}")),
        }
    }
}

impl fmt::Display for PropertyKey {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Named(identifier) => identifier.fmt(formatter),
            Self::Position(position) => position.fmt(formatter),
        }
    }
}
