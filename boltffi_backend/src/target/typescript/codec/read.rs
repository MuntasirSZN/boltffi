use boltffi_binding::{
    BuiltinType, CallbackId, ClassId, CodecRead, CustomTypeId, ElementCount, EnumId, MapKind, Op,
    Primitive, RecordId,
};

use crate::core::{Error, Result};

use super::super::{
    primitive::Scalar,
    syntax::{ArgumentList, Expression, Identifier},
};

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum ReadKind {
    Primitive(Primitive),
    String,
    Bytes,
}

pub struct ReadExpression {
    expression: Expression,
    kind: Option<ReadKind>,
}

pub struct Reader {
    reader: Identifier,
}

impl Reader {
    pub fn new(reader: Identifier) -> Self {
        Self { reader }
    }

    fn unsupported(shape: &'static str) -> Result<ReadExpression> {
        Err(Error::UnsupportedTarget {
            target: "typescript",
            shape,
        })
    }
}

impl ReadExpression {
    pub fn kind(&self) -> Option<ReadKind> {
        self.kind
    }

    pub fn into_expression(self) -> Expression {
        self.expression
    }

    fn string(expression: Expression) -> Self {
        Self {
            expression,
            kind: Some(ReadKind::String),
        }
    }

    fn bytes(expression: Expression) -> Self {
        Self {
            expression,
            kind: Some(ReadKind::Bytes),
        }
    }

    fn primitive(primitive: Primitive, expression: Expression) -> Self {
        Self {
            expression,
            kind: Some(ReadKind::Primitive(primitive)),
        }
    }

    fn dynamic(expression: Expression) -> Self {
        Self {
            expression,
            kind: None,
        }
    }
}

impl CodecRead for Reader {
    type Expr = Result<ReadExpression>;

    fn primitive(&mut self, primitive: Primitive) -> Self::Expr {
        Ok(ReadExpression::primitive(
            primitive,
            Expression::call(
                Expression::identifier(self.reader.clone()),
                Scalar::new(primitive)?.read_method(),
                ArgumentList::default(),
            ),
        ))
    }

    fn string(&mut self) -> Self::Expr {
        Ok(ReadExpression::string(Expression::call(
            Expression::identifier(self.reader.clone()),
            Identifier::known("readString"),
            ArgumentList::default(),
        )))
    }

    fn bytes(&mut self) -> Self::Expr {
        Ok(ReadExpression::bytes(Expression::call(
            Expression::identifier(self.reader.clone()),
            Identifier::known("readBytes"),
            ArgumentList::default(),
        )))
    }

    fn direct_record(&mut self, _id: RecordId) -> Self::Expr {
        Self::unsupported("direct record codec read")
    }

    fn encoded_record(&mut self, _id: RecordId) -> Self::Expr {
        Self::unsupported("encoded record codec read")
    }

    fn c_style_enum(&mut self, _id: EnumId) -> Self::Expr {
        Self::unsupported("C-style enum codec read")
    }

    fn data_enum(&mut self, _id: EnumId) -> Self::Expr {
        Self::unsupported("data enum codec read")
    }

    fn class_handle(&mut self, _id: ClassId) -> Self::Expr {
        Self::unsupported("class handle codec read")
    }

    fn callback_handle(&mut self, _id: CallbackId) -> Self::Expr {
        Self::unsupported("callback handle codec read")
    }

    fn custom(&mut self, _id: CustomTypeId, _representation: Self::Expr) -> Self::Expr {
        Self::unsupported("custom codec read")
    }

    fn builtin(&mut self, _kind: BuiltinType) -> Self::Expr {
        Self::unsupported("builtin codec read")
    }

    fn optional(&mut self, inner: Self::Expr) -> Self::Expr {
        Ok(ReadExpression::dynamic(Expression::call(
            Expression::identifier(self.reader.clone()),
            Identifier::known("readOptional"),
            [Expression::lambda(inner?.into_expression())]
                .into_iter()
                .collect::<ArgumentList>(),
        )))
    }

    fn sequence(&mut self, _len: &Op<ElementCount>, _element: Self::Expr) -> Self::Expr {
        Self::unsupported("sequence codec read")
    }

    fn tuple(&mut self, _elements: Vec<Self::Expr>) -> Self::Expr {
        Self::unsupported("tuple codec read")
    }

    fn result(&mut self, _ok: Self::Expr, _err: Self::Expr) -> Self::Expr {
        Self::unsupported("result codec read")
    }

    fn map(&mut self, _kind: MapKind, _key: Self::Expr, _value: Self::Expr) -> Self::Expr {
        Self::unsupported("map codec read")
    }
}
