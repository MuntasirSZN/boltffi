use crate::{
    bridge::c::{self, TypeFragment},
    core::{Error, Result},
};

const JNI_BRIDGE: &str = "jni";

/// JNI scalar type used in a native method signature.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
#[non_exhaustive]
pub enum JniType {
    /// `jboolean`.
    Boolean,
    /// `jbyte`.
    Byte,
    /// `jshort`.
    Short,
    /// `jint`.
    Int,
    /// `jlong`.
    Long,
    /// `jfloat`.
    Float,
    /// `jdouble`.
    Double,
}

impl JniType {
    /// Returns the JNI type as C syntax.
    pub fn as_type_fragment(self) -> TypeFragment {
        TypeFragment::new(match self {
            Self::Boolean => "jboolean",
            Self::Byte => "jbyte",
            Self::Short => "jshort",
            Self::Int => "jint",
            Self::Long => "jlong",
            Self::Float => "jfloat",
            Self::Double => "jdouble",
        })
    }

    /// Returns whether this type is `jboolean`.
    pub fn is_boolean(self) -> bool {
        matches!(self, Self::Boolean)
    }

    /// Creates the JNI scalar type for a scalar C ABI type.
    pub fn from_c_type(ty: &c::Type) -> Result<Self> {
        match ty {
            c::Type::Bool => Ok(Self::Boolean),
            c::Type::CStyleEnum { repr, .. } => Self::from_c_type(repr),
            c::Type::Int8 | c::Type::Uint8 | c::Type::StreamPollResult => Ok(Self::Byte),
            c::Type::Int16 | c::Type::Uint16 => Ok(Self::Short),
            c::Type::Int32 | c::Type::Uint32 | c::Type::WaitResult => Ok(Self::Int),
            c::Type::Int64
            | c::Type::Uint64
            | c::Type::SignedPointerWidth
            | c::Type::PointerWidth
            | c::Type::FutureHandle
            | c::Type::ConstPointer(_)
            | c::Type::MutPointer(_)
            | c::Type::FunctionPointer { .. } => Ok(Self::Long),
            c::Type::Float32 => Ok(Self::Float),
            c::Type::Float64 => Ok(Self::Double),
            c::Type::CallbackHandle(_) => Err(Error::UnsupportedBridge {
                bridge: JNI_BRIDGE,
                shape: "callback handle C ABI",
            }),
            c::Type::Void
            | c::Type::Status
            | c::Type::Buffer
            | c::Type::String
            | c::Type::Span
            | c::Type::Named(_)
            | c::Type::DirectRecord(_) => Err(Error::UnsupportedBridge {
                bridge: JNI_BRIDGE,
                shape: "non-scalar C ABI function",
            }),
        }
    }
}
