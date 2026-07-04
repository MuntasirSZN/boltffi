use crate::{
    core::Result,
    target::swift::{
        name_style::Name,
        syntax::{ArgumentList, Expression, Identifier, Statement, TypeName},
    },
};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ArgumentBuffer {
    bytes: Identifier,
    buffer: Identifier,
    writer: Identifier,
    statements: Vec<Statement>,
}

impl ArgumentBuffer {
    pub fn new(name: &Name) -> Result<Self> {
        Ok(Self {
            bytes: name.generated("bytes")?,
            buffer: name.generated("buffer")?,
            writer: name.generated("writer")?,
            statements: Vec::new(),
        })
    }

    pub fn with_statements(mut self, statements: Vec<Statement>) -> Self {
        self.statements = statements;
        self
    }

    pub fn with_statement(mut self, statement: Statement) -> Self {
        self.statements.push(statement);
        self
    }

    pub fn writer(&self) -> &Identifier {
        &self.writer
    }

    pub fn arguments(&self) -> Vec<Expression> {
        vec![
            Expression::forced(Expression::member(&self.buffer, "baseAddress")),
            Expression::call(
                TypeName::uint(),
                [Expression::member(&self.buffer, "count")]
                    .into_iter()
                    .collect::<ArgumentList>(),
            ),
        ]
    }

    pub fn bytes_statement(&self) -> Statement {
        Statement::let_value(&self.bytes, self.encode_call())
    }

    pub fn with_buffer_scope(&self, body: String, indent: &str, returns_value: bool) -> String {
        let prefix = if returns_value { "return " } else { "" };
        format!(
            "{indent}{prefix}{}.withUnsafeBufferPointer {{ {} in\n{}\n{indent}}}",
            self.bytes, self.buffer, body
        )
    }

    fn encode_call(&self) -> Expression {
        Expression::trailing_closure(
            "boltffiEncode",
            ArgumentList::default(),
            &self.writer,
            self.statements
                .iter()
                .map(ToString::to_string)
                .collect::<Vec<_>>()
                .join("; "),
        )
    }
}
