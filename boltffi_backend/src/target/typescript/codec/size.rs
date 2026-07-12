use boltffi_binding::{
    BinderId, BuiltinType, CallbackId, ClassId, CodecSize, CustomTypeId, ElementCount, EnumId,
    MapKind, Op, Primitive, RecordId, ValueRef, Wasm32,
};

use crate::core::{Error, Result};

use super::super::syntax::{ArgumentList, Expression, Identifier};
use super::value::ValueExpression;

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum SizeKind {
    Primitive(Primitive),
    String,
    Bytes,
}

pub struct SizeExpression {
    expression: Expression,
    kind: Option<SizeKind>,
}

pub struct Sizer {
    current: Expression,
}

impl Sizer {
    pub fn new(current: Expression) -> Self {
        Self { current }
    }

    fn value(&self, value: &ValueRef) -> Result<Expression> {
        ValueExpression::new(value, self.current.clone()).render()
    }

    fn unsupported(shape: &'static str) -> Result<SizeExpression> {
        Err(Error::UnsupportedTarget {
            target: "typescript",
            shape,
        })
    }
}

impl SizeExpression {
    pub fn kind(&self) -> Option<SizeKind> {
        self.kind
    }

    pub fn into_expression(self) -> Expression {
        self.expression
    }

    fn string(expression: Expression) -> Self {
        Self {
            expression,
            kind: Some(SizeKind::String),
        }
    }

    fn bytes(expression: Expression) -> Self {
        Self {
            expression,
            kind: Some(SizeKind::Bytes),
        }
    }

    fn primitive(primitive: Primitive) -> Self {
        Self {
            expression: Expression::integer(primitive.byte_size::<Wasm32>().get()),
            kind: Some(SizeKind::Primitive(primitive)),
        }
    }

    fn dynamic(expression: Expression) -> Self {
        Self {
            expression,
            kind: None,
        }
    }
}

impl CodecSize for Sizer {
    type Expr = Result<SizeExpression>;

    fn primitive(&mut self, primitive: Primitive, _value: &ValueRef) -> Self::Expr {
        Ok(SizeExpression::primitive(primitive))
    }

    fn string(&mut self, value: &ValueRef) -> Self::Expr {
        Ok(SizeExpression::string(Expression::invoke(
            Identifier::known("wireStringSize"),
            [self.value(value)?].into_iter().collect::<ArgumentList>(),
        )))
    }

    fn bytes(&mut self, value: &ValueRef) -> Self::Expr {
        Ok(SizeExpression::bytes(Expression::integer(4).add(
            Expression::property(self.value(value)?, Identifier::known("length")),
        )))
    }

    fn direct_record(&mut self, _id: RecordId, _value: &ValueRef) -> Self::Expr {
        Self::unsupported("direct record codec size")
    }

    fn encoded_record(&mut self, _id: RecordId, _value: &ValueRef) -> Self::Expr {
        Self::unsupported("encoded record codec size")
    }

    fn c_style_enum(&mut self, _id: EnumId, _value: &ValueRef) -> Self::Expr {
        Self::unsupported("C-style enum codec size")
    }

    fn data_enum(&mut self, _id: EnumId, _value: &ValueRef) -> Self::Expr {
        Self::unsupported("data enum codec size")
    }

    fn class_handle(&mut self, _id: ClassId, _value: &ValueRef) -> Self::Expr {
        Self::unsupported("class handle codec size")
    }

    fn callback_handle(&mut self, _id: CallbackId, _value: &ValueRef) -> Self::Expr {
        Self::unsupported("callback handle codec size")
    }

    fn custom<F>(&mut self, _id: CustomTypeId, _value: &ValueRef, _representation: F) -> Self::Expr
    where
        F: FnOnce(&mut Self, &ValueRef) -> Self::Expr,
    {
        Self::unsupported("custom codec size")
    }

    fn builtin(&mut self, _kind: BuiltinType, _value: &ValueRef) -> Self::Expr {
        Self::unsupported("builtin codec size")
    }

    fn optional(&mut self, value: &ValueRef, binder: BinderId, inner: Self::Expr) -> Self::Expr {
        Ok(SizeExpression::dynamic(Expression::invoke(
            Identifier::known("wireOptionalSize"),
            [
                self.value(value)?,
                Expression::parameter_lambda(
                    ValueExpression::binder(binder)?,
                    inner?.into_expression(),
                ),
            ]
            .into_iter()
            .collect::<ArgumentList>(),
        )))
    }

    fn sequence(
        &mut self,
        _value: &ValueRef,
        _len: &Op<ElementCount>,
        _binder: BinderId,
        _element: Self::Expr,
    ) -> Self::Expr {
        Self::unsupported("sequence codec size")
    }

    fn tuple(&mut self, _value: &ValueRef, _elements: Vec<Self::Expr>) -> Self::Expr {
        Self::unsupported("tuple codec size")
    }

    fn result(
        &mut self,
        _value: &ValueRef,
        _binder: BinderId,
        _ok: Self::Expr,
        _err: Self::Expr,
    ) -> Self::Expr {
        Self::unsupported("result codec size")
    }

    fn map(
        &mut self,
        _kind: MapKind,
        _value: &ValueRef,
        _key_binder: BinderId,
        _key: Self::Expr,
        _value_binder: BinderId,
        _map_value: Self::Expr,
    ) -> Self::Expr {
        Self::unsupported("map codec size")
    }
}
