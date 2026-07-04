use boltffi_binding::{ClassId, EnumId, Native, Primitive, RecordId, TypeRef, native};

use crate::{
    core::{Error, RenderContext, Result},
    target::swift::{SwiftHost, name_style::Name, primitive::SwiftPrimitive, syntax::TypeName},
};

pub struct SwiftType;

impl SwiftType {
    pub fn primitive(primitive: Primitive) -> Result<TypeName> {
        SwiftPrimitive::new(primitive).api_type()
    }

    pub fn record(id: RecordId, context: &RenderContext<Native>) -> Result<TypeName> {
        context
            .record(id)
            .map(|record| Name::new(record.name()).type_name())
            .ok_or(Error::BrokenBridgeContract {
                bridge: SwiftHost::TARGET,
                invariant: "missing record type in Swift render context",
            })
    }

    pub fn enumeration(id: EnumId, context: &RenderContext<Native>) -> Result<TypeName> {
        context
            .enumeration(id)
            .map(|enumeration| Name::new(enumeration.name()).type_name())
            .ok_or(Error::BrokenBridgeContract {
                bridge: SwiftHost::TARGET,
                invariant: "missing enum type in Swift render context",
            })
    }

    pub fn class(id: ClassId, context: &RenderContext<Native>) -> Result<TypeName> {
        context
            .class(id)
            .map(|class| Name::new(class.name()).type_name())
            .ok_or(Error::BrokenBridgeContract {
                bridge: SwiftHost::TARGET,
                invariant: "missing class type in Swift render context",
            })
    }

    pub fn handle_carrier(carrier: native::HandleCarrier) -> Result<TypeName> {
        match carrier {
            native::HandleCarrier::U64 => Ok(TypeName::uint64()),
            native::HandleCarrier::USize => Ok(TypeName::uint()),
            native::HandleCarrier::CallbackHandle => {
                Err(SwiftHost::unsupported("callback handle carrier"))
            }
            _ => Err(SwiftHost::unsupported("unknown handle carrier")),
        }
    }

    pub fn type_ref(ty: &TypeRef, context: &RenderContext<Native>) -> Result<TypeName> {
        match ty {
            TypeRef::Primitive(primitive) => Self::primitive(*primitive),
            TypeRef::String => Ok(TypeName::string()),
            TypeRef::Bytes => Ok(TypeName::data()),
            TypeRef::Record(record) => Self::record(*record, context),
            TypeRef::Enum(enumeration) => Self::enumeration(*enumeration, context),
            TypeRef::Optional(inner) => Self::type_ref(inner, context).map(TypeName::optional),
            TypeRef::Sequence(inner) => Self::type_ref(inner, context).map(TypeName::array),
            _ => Err(SwiftHost::unsupported("Swift type reference")),
        }
    }
}
