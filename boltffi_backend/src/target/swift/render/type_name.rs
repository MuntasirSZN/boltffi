use boltffi_binding::Primitive;

use crate::{
    core::Result,
    target::swift::{primitive::SwiftPrimitive, syntax::TypeName},
};

pub struct SwiftType;

impl SwiftType {
    pub fn primitive(primitive: Primitive) -> Result<TypeName> {
        SwiftPrimitive::new(primitive).api_type()
    }
}
