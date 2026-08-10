use std::fmt;

use crate::core::{Error, LanguageSyntax, Result, syntax::sealed};

use super::Syntax;

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct Identifier(String);

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct MemberName(String);

impl Identifier {
    pub fn parse(identifier: impl Into<String>) -> Result<Self> {
        let identifier = identifier.into();
        match Self::valid(&identifier) && !Syntax::keyword(&identifier) {
            true => Ok(Self(identifier)),
            false => Err(Error::InvalidTypeScriptIdentifier { identifier }),
        }
    }

    pub fn escape(identifier: impl Into<String>) -> Result<Self> {
        let identifier = identifier.into();
        match Syntax::keyword(&identifier) {
            true => Self::parse(format!("_{identifier}")),
            false => Self::parse(identifier),
        }
    }

    pub fn known(identifier: &'static str) -> Self {
        Self::parse(identifier).expect("static TypeScript identifier must be valid")
    }

    fn valid(identifier: &str) -> bool {
        let mut characters = identifier.chars();
        characters
            .next()
            .is_some_and(|character| matches!(character, '_' | '$') || character.is_alphabetic())
            && characters
                .all(|character| matches!(character, '_' | '$') || character.is_alphanumeric())
    }
}

impl fmt::Display for Identifier {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl MemberName {
    pub fn parse(name: impl Into<String>) -> Result<Self> {
        let name = name.into();
        match Identifier::valid(&name) {
            true => Ok(Self(name)),
            false => Err(Error::InvalidTypeScriptIdentifier { identifier: name }),
        }
    }
}

impl MemberName {
    /// Whether this name has to be spelled as a string to declare a member.
    ///
    /// `new(key: string): void` inside an interface is a construct signature,
    /// not a method named `new`, so the member silently does not exist. Every
    /// other reserved word — `delete`, `class`, even `constructor` — reads as
    /// an ordinary member here and needs no quoting.
    pub(super) fn needs_quoting(&self) -> bool {
        self.0 == "new"
    }
}

impl fmt::Display for MemberName {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

/// A member name in the position where an interface declares it.
///
/// Only this position is ambiguous. A class body may declare `new(…) {}` as an
/// ordinary method, and `Counter.new(1)` reads it back; an interface may not.
pub struct InterfaceMemberName(MemberName);

impl InterfaceMemberName {
    pub fn new(name: MemberName) -> Self {
        Self(name)
    }
}

impl fmt::Display for InterfaceMemberName {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.0.needs_quoting() {
            true => write!(formatter, "\"{}\"", self.0),
            false => formatter.write_str(&self.0.0),
        }
    }
}

impl sealed::SyntaxFragment for Identifier {}
impl sealed::SyntaxFragment for MemberName {}
