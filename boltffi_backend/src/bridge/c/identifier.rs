use std::fmt;

use crate::core::{Error, Result};

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct Identifier(String);

impl Identifier {
    pub fn parse(identifier: impl Into<String>) -> Result<Self> {
        let identifier = identifier.into();
        if valid(&identifier) {
            Ok(Self(identifier))
        } else {
            Err(Error::InvalidCIdentifier { identifier })
        }
    }

    pub fn escape(identifier: impl Into<String>) -> Result<Self> {
        let identifier = identifier.into();
        match keyword(&identifier) {
            true => Self::parse(format!("{identifier}_")),
            false => Self::parse(identifier),
        }
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for Identifier {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

fn valid(identifier: &str) -> bool {
    let mut characters = identifier.chars();
    characters
        .next()
        .is_some_and(|character| character == '_' || character.is_ascii_alphabetic())
        && characters.all(|character| character == '_' || character.is_ascii_alphanumeric())
}

fn keyword(identifier: &str) -> bool {
    matches!(
        identifier,
        "auto"
            | "break"
            | "case"
            | "char"
            | "const"
            | "continue"
            | "default"
            | "do"
            | "double"
            | "else"
            | "enum"
            | "extern"
            | "float"
            | "for"
            | "goto"
            | "if"
            | "inline"
            | "int"
            | "long"
            | "register"
            | "restrict"
            | "return"
            | "short"
            | "signed"
            | "sizeof"
            | "static"
            | "struct"
            | "switch"
            | "typedef"
            | "union"
            | "unsigned"
            | "void"
            | "volatile"
            | "while"
    )
}
