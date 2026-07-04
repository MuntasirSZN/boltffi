use boltffi_binding::Primitive;

use crate::{
    core::Result,
    target::swift::{SwiftHost, syntax::TypeName},
};

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct SwiftPrimitive {
    primitive: Primitive,
}

impl SwiftPrimitive {
    pub fn new(primitive: Primitive) -> Self {
        Self { primitive }
    }

    pub fn api_type(self) -> Result<TypeName> {
        match self.primitive {
            Primitive::Bool => Ok(TypeName::bool()),
            Primitive::I8 => Ok(TypeName::int8()),
            Primitive::U8 => Ok(TypeName::uint8()),
            Primitive::I16 => Ok(TypeName::int16()),
            Primitive::U16 => Ok(TypeName::uint16()),
            Primitive::I32 => Ok(TypeName::int32()),
            Primitive::U32 => Ok(TypeName::uint32()),
            Primitive::I64 => Ok(TypeName::int64()),
            Primitive::U64 => Ok(TypeName::uint64()),
            Primitive::ISize => Ok(TypeName::int()),
            Primitive::USize => Ok(TypeName::uint()),
            Primitive::F32 => Ok(TypeName::float()),
            Primitive::F64 => Ok(TypeName::double()),
            _ => Err(SwiftHost::unsupported("unknown primitive")),
        }
    }
}
