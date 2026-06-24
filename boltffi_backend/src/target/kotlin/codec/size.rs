use boltffi_binding::{
    BinderId, BuiltinType, CallbackId, ClassId, CodecSize, CustomTypeId, ElementCount, EnumId,
    MapKind, Op, Primitive, RecordId, ValueRef,
};

use crate::{
    core::{Error, Result},
    target::kotlin::{
        codec::value::ValueExpression,
        primitive::KotlinPrimitive,
        syntax::{ArgumentList, Expression, Identifier},
    },
};

pub struct Sizer;

impl Sizer {
    fn value(value: &ValueRef) -> Result<Expression> {
        ValueExpression::new(value).render()
    }

    fn fixed(bytes: u64) -> Result<Expression> {
        Ok(Expression::integer(bytes))
    }

    fn string_size(value: &ValueRef) -> Result<Expression> {
        Ok(Self::fixed(4)?.add(Expression::call(
            "Utf8Codec",
            Identifier::parse("maxBytes")?,
            [Self::value(value)?].into_iter().collect::<ArgumentList>(),
        )))
    }

    fn bytes_size(value: &ValueRef) -> Result<Expression> {
        Ok(Self::fixed(4)?.add(Expression::property(
            Self::value(value)?,
            Identifier::parse("size")?,
        )))
    }

    fn primitive_size(primitive: Primitive) -> Result<Expression> {
        KotlinPrimitive::new(primitive)
            .wire_size()
            .map(Expression::integer)
    }

    fn unsupported(shape: &'static str) -> Result<Expression> {
        Err(Error::UnsupportedTarget {
            target: "kotlin",
            shape,
        })
    }
}

impl CodecSize for Sizer {
    type Expr = Result<Expression>;

    fn primitive(&mut self, primitive: Primitive, _value: &ValueRef) -> Self::Expr {
        Self::primitive_size(primitive)
    }

    fn string(&mut self, value: &ValueRef) -> Self::Expr {
        Self::string_size(value)
    }

    fn bytes(&mut self, value: &ValueRef) -> Self::Expr {
        Self::bytes_size(value)
    }

    fn direct_record(&mut self, _id: RecordId, _value: &ValueRef) -> Self::Expr {
        Self::unsupported("direct-record wire size")
    }

    fn encoded_record(&mut self, _id: RecordId, _value: &ValueRef) -> Self::Expr {
        Self::unsupported("encoded-record wire size")
    }

    fn c_style_enum(&mut self, _id: EnumId, _value: &ValueRef) -> Self::Expr {
        Self::unsupported("c-style enum wire size")
    }

    fn data_enum(&mut self, _id: EnumId, _value: &ValueRef) -> Self::Expr {
        Self::unsupported("data enum wire size")
    }

    fn class_handle(&mut self, _id: ClassId, _value: &ValueRef) -> Self::Expr {
        Self::unsupported("class handle wire size")
    }

    fn callback_handle(&mut self, _id: CallbackId, _value: &ValueRef) -> Self::Expr {
        Self::unsupported("callback handle wire size")
    }

    fn custom(
        &mut self,
        _id: CustomTypeId,
        _value: &ValueRef,
        representation: Self::Expr,
    ) -> Self::Expr {
        representation
    }

    fn builtin(&mut self, _kind: BuiltinType, _value: &ValueRef) -> Self::Expr {
        Self::unsupported("builtin wire size")
    }

    fn optional(&mut self, _value: &ValueRef, _binder: BinderId, _inner: Self::Expr) -> Self::Expr {
        Self::unsupported("optional wire size")
    }

    fn sequence(
        &mut self,
        _value: &ValueRef,
        _len: &Op<ElementCount>,
        _binder: BinderId,
        _element: Self::Expr,
    ) -> Self::Expr {
        Self::unsupported("sequence wire size")
    }

    fn tuple(&mut self, _value: &ValueRef, _elements: Vec<Self::Expr>) -> Self::Expr {
        Self::unsupported("tuple wire size")
    }

    fn result(
        &mut self,
        _value: &ValueRef,
        _binder: BinderId,
        _ok: Self::Expr,
        _err: Self::Expr,
    ) -> Self::Expr {
        Self::unsupported("result wire size")
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
        Self::unsupported("map wire size")
    }
}
