use boltffi_binding::{
    HandlePresence, HandleTarget, Native, OutOfRust, ReturnPlan, TypeRef, native,
};

use crate::{
    bridge::python_cext::PythonCExtBridgeContract,
    core::{Error, RenderContext, Result},
    target::python::cpython::render::{custom, enumeration, primitive, record},
};

pub struct Conversion {
    pub void: bool,
    pub converter: String,
    primitive: Option<primitive::Runtime>,
    owned_buffer: Option<OwnedBuffer>,
}

impl Conversion {
    pub fn from_plan(
        plan: &ReturnPlan<Native, OutOfRust>,
        bridge: &PythonCExtBridgeContract,
        context: &RenderContext<Native>,
    ) -> Result<Self> {
        match plan {
            ReturnPlan::Void => Ok(Self {
                void: true,
                converter: String::new(),
                primitive: None,
                owned_buffer: None,
            }),
            ReturnPlan::DirectViaReturnSlot {
                ty: TypeRef::Primitive(primitive),
            } => {
                let primitive = primitive::Runtime::new(*primitive);
                Ok(Self {
                    void: false,
                    converter: primitive.boxer()?.to_owned(),
                    primitive: Some(primitive),
                    owned_buffer: None,
                })
            }
            ReturnPlan::DirectViaReturnSlot {
                ty: TypeRef::Record(record),
            } => Ok(Self {
                void: false,
                converter: record::Symbols::from_record_id(*record, bridge, context)?
                    .boxer()
                    .to_owned(),
                primitive: None,
                owned_buffer: None,
            }),
            ReturnPlan::DirectViaReturnSlot {
                ty: TypeRef::Enum(enumeration),
            } => Ok(Self {
                void: false,
                converter: enumeration::Symbols::from_enum_id(*enumeration, bridge, context)?
                    .boxer()
                    .to_owned(),
                primitive: None,
                owned_buffer: None,
            }),
            ReturnPlan::EncodedViaReturnSlot {
                ty: TypeRef::String,
                shape: native::BufferShape::Buffer,
                ..
            } => Self::from_owned_buffer(OwnedBuffer::String),
            ReturnPlan::EncodedViaReturnSlot {
                ty: TypeRef::Bytes,
                shape: native::BufferShape::Buffer,
                ..
            } => Self::from_owned_buffer(OwnedBuffer::Bytes),
            ReturnPlan::EncodedViaReturnSlot {
                ty: TypeRef::Custom(custom_type),
                shape: native::BufferShape::Buffer,
                ..
            } => {
                let custom_types = custom::CustomTypes::from_context(context);
                Self::from_encoded_type(custom_types.representation(*custom_type)?)
            }
            ReturnPlan::HandleViaReturnSlot {
                target: HandleTarget::Class(_),
                carrier,
                presence: HandlePresence::Required,
            } => {
                let carrier = primitive::Runtime::native_handle(*carrier)?;
                Ok(Self {
                    void: false,
                    converter: carrier.boxer()?.to_owned(),
                    primitive: Some(carrier),
                    owned_buffer: None,
                })
            }
            ReturnPlan::DirectViaReturnSlot { .. }
            | ReturnPlan::EncodedViaReturnSlot { .. }
            | ReturnPlan::HandleViaReturnSlot { .. }
            | ReturnPlan::ScalarOptionViaReturnSlot { .. }
            | ReturnPlan::DirectVecViaReturnSlot { .. }
            | ReturnPlan::DirectViaOutPointer { .. }
            | ReturnPlan::EncodedViaOutPointer { .. }
            | ReturnPlan::HandleViaOutPointer { .. }
            | ReturnPlan::ClosureViaOutPointer(_) => Err(Error::UnsupportedTarget {
                target: "python",
                shape: "non-primitive return",
            }),
            _ => Err(Error::UnsupportedTarget {
                target: "python",
                shape: "unknown return plan",
            }),
        }
    }

    pub fn primitive(&self) -> Option<primitive::Runtime> {
        self.primitive
    }

    pub fn owned_buffer(&self) -> Option<OwnedBuffer> {
        self.owned_buffer
    }

    pub fn is_void(&self) -> bool {
        self.void
    }

    fn from_owned_buffer(buffer: OwnedBuffer) -> Result<Self> {
        Ok(Self {
            void: false,
            converter: buffer.converter()?,
            primitive: buffer.primitive(),
            owned_buffer: Some(buffer),
        })
    }

    fn from_encoded_type(ty: &TypeRef) -> Result<Self> {
        match ty {
            TypeRef::Primitive(primitive) => {
                Self::from_owned_buffer(OwnedBuffer::Primitive(primitive::Runtime::new(*primitive)))
            }
            TypeRef::String => Self::from_owned_buffer(OwnedBuffer::String),
            TypeRef::Bytes => Self::from_owned_buffer(OwnedBuffer::Bytes),
            _ => Err(Error::UnsupportedTarget {
                target: "python",
                shape: "unsupported custom representation return",
            }),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum OwnedBuffer {
    String,
    Bytes,
    Primitive(primitive::Runtime),
}

impl OwnedBuffer {
    pub fn converter(self) -> Result<String> {
        match self {
            Self::String => Ok("boltffi_python_decode_owned_utf8".to_owned()),
            Self::Bytes => Ok("boltffi_python_decode_owned_bytes".to_owned()),
            Self::Primitive(primitive) => primitive.owned_wire_decoder(),
        }
    }

    pub fn primitive(self) -> Option<primitive::Runtime> {
        match self {
            Self::Primitive(primitive) => Some(primitive),
            Self::String | Self::Bytes => None,
        }
    }
}
