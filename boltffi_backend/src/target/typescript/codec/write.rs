use boltffi_binding::{
    BinderId, BuiltinType, CallbackId, ClassId, CodecWrite, CustomTypeId, ElementCount, EnumId,
    MapKind, Op, Primitive, RecordId, ValueRef,
};

use crate::core::{Error, Result};

use super::super::primitive::Scalar;
use super::super::syntax::{ArgumentList, Expression, Identifier, Statement};
use super::value::ValueExpression;

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum WriteKind {
    Primitive(Primitive),
    String,
    Bytes,
}

pub struct WriteStatement {
    statement: Statement,
    kind: Option<WriteKind>,
}

pub struct Writer {
    writer: Identifier,
    current: Expression,
}

impl Writer {
    pub fn new(writer: Identifier, current: Expression) -> Self {
        Self { writer, current }
    }

    fn value(&self, value: &ValueRef) -> Result<Expression> {
        ValueExpression::new(value, self.current.clone()).render()
    }

    fn unsupported(shape: &'static str) -> Vec<Result<WriteStatement>> {
        vec![Err(Error::UnsupportedTarget {
            target: "typescript",
            shape,
        })]
    }
}

impl WriteStatement {
    pub fn kind(&self) -> Option<WriteKind> {
        self.kind
    }

    pub fn into_statement(self) -> Statement {
        self.statement
    }

    fn string(statement: Statement) -> Self {
        Self {
            statement,
            kind: Some(WriteKind::String),
        }
    }

    fn bytes(statement: Statement) -> Self {
        Self {
            statement,
            kind: Some(WriteKind::Bytes),
        }
    }

    fn primitive(primitive: Primitive, statement: Statement) -> Self {
        Self {
            statement,
            kind: Some(WriteKind::Primitive(primitive)),
        }
    }

    fn dynamic(statement: Statement) -> Self {
        Self {
            statement,
            kind: None,
        }
    }
}

impl CodecWrite for Writer {
    type Stmt = Result<WriteStatement>;

    fn primitive(&mut self, primitive: Primitive, value: &ValueRef) -> Vec<Self::Stmt> {
        vec![self.value(value).and_then(|value| {
            Ok(WriteStatement::primitive(
                primitive,
                Statement::expression(Expression::call(
                    Expression::identifier(self.writer.clone()),
                    Scalar::new(primitive)?.write_method(),
                    [value].into_iter().collect::<ArgumentList>(),
                )),
            ))
        })]
    }

    fn string(&mut self, value: &ValueRef) -> Vec<Self::Stmt> {
        vec![self.value(value).map(|value| {
            WriteStatement::string(Statement::expression(Expression::call(
                Expression::identifier(self.writer.clone()),
                Identifier::known("writeString"),
                [value].into_iter().collect::<ArgumentList>(),
            )))
        })]
    }

    fn bytes(&mut self, value: &ValueRef) -> Vec<Self::Stmt> {
        vec![self.value(value).map(|value| {
            WriteStatement::bytes(Statement::expression(Expression::call(
                Expression::identifier(self.writer.clone()),
                Identifier::known("writeBytes"),
                [value].into_iter().collect::<ArgumentList>(),
            )))
        })]
    }

    fn direct_record(&mut self, _id: RecordId, _value: &ValueRef) -> Vec<Self::Stmt> {
        Self::unsupported("direct record codec write")
    }

    fn encoded_record(&mut self, _id: RecordId, _value: &ValueRef) -> Vec<Self::Stmt> {
        Self::unsupported("encoded record codec write")
    }

    fn c_style_enum(&mut self, _id: EnumId, _value: &ValueRef) -> Vec<Self::Stmt> {
        Self::unsupported("C-style enum codec write")
    }

    fn data_enum(&mut self, _id: EnumId, _value: &ValueRef) -> Vec<Self::Stmt> {
        Self::unsupported("data enum codec write")
    }

    fn class_handle(&mut self, _id: ClassId, _value: &ValueRef) -> Vec<Self::Stmt> {
        Self::unsupported("class handle codec write")
    }

    fn callback_handle(&mut self, _id: CallbackId, _value: &ValueRef) -> Vec<Self::Stmt> {
        Self::unsupported("callback handle codec write")
    }

    fn custom<F>(
        &mut self,
        _id: CustomTypeId,
        _value: &ValueRef,
        _representation: F,
    ) -> Vec<Self::Stmt>
    where
        F: FnOnce(&mut Self, &ValueRef) -> Vec<Self::Stmt>,
    {
        Self::unsupported("custom codec write")
    }

    fn builtin(&mut self, _kind: BuiltinType, _value: &ValueRef) -> Vec<Self::Stmt> {
        Self::unsupported("builtin codec write")
    }

    fn optional(
        &mut self,
        value: &ValueRef,
        binder: BinderId,
        inner: Vec<Self::Stmt>,
    ) -> Vec<Self::Stmt> {
        vec![self.value(value).and_then(|value| {
            let mut statements = inner.into_iter();
            let Some(inner) = statements.next() else {
                return Err(Error::UnsupportedTarget {
                    target: "typescript",
                    shape: "empty optional codec write",
                });
            };
            if statements.next().is_some() {
                return Err(Error::UnsupportedTarget {
                    target: "typescript",
                    shape: "multi-statement optional codec write",
                });
            }
            let inner = inner?.into_statement();
            Ok(WriteStatement::dynamic(Statement::expression(
                Expression::call(
                    Expression::identifier(self.writer.clone()),
                    Identifier::known("writeOptional"),
                    [
                        value,
                        Expression::statement_lambda(ValueExpression::binder(binder)?, inner),
                    ]
                    .into_iter()
                    .collect::<ArgumentList>(),
                ),
            )))
        })]
    }

    fn sequence(
        &mut self,
        _value: &ValueRef,
        _len: &Op<ElementCount>,
        _binder: BinderId,
        _element: Vec<Self::Stmt>,
    ) -> Vec<Self::Stmt> {
        Self::unsupported("sequence codec write")
    }

    fn tuple(&mut self, _value: &ValueRef, _elements: Vec<Vec<Self::Stmt>>) -> Vec<Self::Stmt> {
        Self::unsupported("tuple codec write")
    }

    fn result(
        &mut self,
        _value: &ValueRef,
        _binder: BinderId,
        _ok: Vec<Self::Stmt>,
        _err: Vec<Self::Stmt>,
    ) -> Vec<Self::Stmt> {
        Self::unsupported("result codec write")
    }

    fn map(
        &mut self,
        _kind: MapKind,
        _value: &ValueRef,
        _key_binder: BinderId,
        _key: Vec<Self::Stmt>,
        _value_binder: BinderId,
        _map_value: Vec<Self::Stmt>,
    ) -> Vec<Self::Stmt> {
        Self::unsupported("map codec write")
    }
}
