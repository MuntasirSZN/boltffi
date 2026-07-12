use std::fmt;

use crate::core::syntax::sealed;

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct StringLiteral(String);

impl StringLiteral {
    pub fn new(value: &str) -> Self {
        Self(serde_json::to_string(value).expect("string literal serializes"))
    }
}

impl fmt::Display for StringLiteral {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl sealed::SyntaxFragment for StringLiteral {}
