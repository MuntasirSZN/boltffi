use boltffi_binding::{
    BuiltinType, CallbackId, ClassId, CustomTypeId, EnumId, Primitive, RecordId, TypeRef,
    TypeRefRender,
};

use crate::core::{Error, Result};

use super::super::{primitive::Scalar, syntax::TypeName};

pub struct Type;

impl Type {
    pub fn from_ref(ty: &TypeRef) -> Result<TypeName> {
        ty.render_with(&mut Self)
    }

    pub fn primitive(primitive: Primitive) -> Result<TypeName> {
        Scalar::new(primitive).map(Scalar::ty)
    }

    fn unsupported<T>(shape: &'static str) -> Result<T> {
        Err(Error::UnsupportedTarget {
            target: "typescript",
            shape,
        })
    }
}

impl TypeRefRender for Type {
    type Output = Result<TypeName>;

    fn primitive(&mut self, primitive: Primitive) -> Self::Output {
        Self::primitive(primitive)
    }

    fn string(&mut self) -> Self::Output {
        Ok(TypeName::string())
    }

    fn bytes(&mut self) -> Self::Output {
        Ok(TypeName::named("Uint8Array"))
    }

    fn record(&mut self, _id: RecordId) -> Self::Output {
        Self::unsupported("record type")
    }

    fn enumeration(&mut self, _id: EnumId) -> Self::Output {
        Self::unsupported("enum type")
    }

    fn class(&mut self, _id: ClassId) -> Self::Output {
        Self::unsupported("class type")
    }

    fn callback(&mut self, _id: CallbackId) -> Self::Output {
        Self::unsupported("callback type")
    }

    fn custom(&mut self, _id: CustomTypeId) -> Self::Output {
        Self::unsupported("custom type")
    }

    fn builtin(&mut self, _kind: BuiltinType) -> Self::Output {
        Self::unsupported("builtin type")
    }

    fn optional(&mut self, inner: Self::Output) -> Self::Output {
        inner.map(TypeName::nullable)
    }

    fn sequence(&mut self, _element: Self::Output) -> Self::Output {
        Self::unsupported("sequence type")
    }

    fn tuple(&mut self, _elements: Vec<Self::Output>) -> Self::Output {
        Self::unsupported("tuple type")
    }

    fn result(&mut self, _ok: Self::Output, _err: Self::Output) -> Self::Output {
        Self::unsupported("result type")
    }

    fn map(&mut self, _key: Self::Output, _value: Self::Output) -> Self::Output {
        Self::unsupported("map type")
    }
}
