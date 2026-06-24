use boltffi_binding::{
    BuiltinType, CallbackId, ClassId, CustomTypeId, DirectValueType, Direction, EnumId,
    HandlePresence, HandleTarget, IntoRust, Native, ParamPlanRender, Primitive, RecordId, TypeRef,
    TypeRefRender,
};

use crate::{
    bridge::jni::{DirectVectorParameter, JniType, NativeParameterKind, NativeReturn},
    core::{Error, Result},
    target::kotlin::{render::primitive::KotlinPrimitive, syntax::TypeName},
};

const KOTLIN_TARGET: &str = "kotlin";

pub struct KotlinType;

impl KotlinType {
    pub fn primitive(primitive: Primitive) -> Result<TypeName> {
        KotlinPrimitive::new(primitive).api_type()
    }

    pub fn jni(jni_type: JniType) -> Result<TypeName> {
        match jni_type {
            JniType::Boolean => Ok(TypeName::boolean()),
            JniType::Byte => Ok(TypeName::byte()),
            JniType::Short => Ok(TypeName::short()),
            JniType::Int => Ok(TypeName::int()),
            JniType::Long => Ok(TypeName::long()),
            JniType::Float => Ok(TypeName::float()),
            JniType::Double => Ok(TypeName::double()),
        }
    }

    pub fn jni_array(jni_type: JniType) -> Result<TypeName> {
        match jni_type {
            JniType::Boolean => Ok(TypeName::new("BooleanArray")),
            JniType::Byte => Ok(TypeName::new("ByteArray")),
            JniType::Short => Ok(TypeName::new("ShortArray")),
            JniType::Int => Ok(TypeName::new("IntArray")),
            JniType::Long => Ok(TypeName::new("LongArray")),
            JniType::Float => Ok(TypeName::new("FloatArray")),
            JniType::Double => Ok(TypeName::new("DoubleArray")),
        }
    }

    pub fn native_parameter(kind: &NativeParameterKind) -> Result<TypeName> {
        match kind {
            NativeParameterKind::Scalar(parameter) => Self::jni(parameter.ty()),
            NativeParameterKind::Bytes(_) | NativeParameterKind::Record(_) => {
                Ok(TypeName::byte_array(false))
            }
            NativeParameterKind::DirectVector(parameter) => Self::direct_vector(parameter),
            NativeParameterKind::Callback(_)
            | NativeParameterKind::Closure(_)
            | NativeParameterKind::Continuation(_) => Ok(TypeName::long()),
        }
    }

    pub fn native_return(return_value: &NativeReturn) -> Result<TypeName> {
        match return_value {
            NativeReturn::Void | NativeReturn::Status => Ok(TypeName::unit()),
            NativeReturn::Value(scalar) => Self::jni(scalar.jni_type()),
            NativeReturn::Bytes | NativeReturn::Record(_) => Ok(TypeName::byte_array(true)),
            NativeReturn::Callback(_) => Ok(TypeName::long()),
        }
    }

    pub fn type_ref(ty: &TypeRef) -> Result<TypeName> {
        ty.render_with(&mut KotlinTypeRef)
    }

    fn direct_vector(parameter: &DirectVectorParameter) -> Result<TypeName> {
        Self::jni_array(parameter.jni_type())
    }
}

pub struct KotlinTypeRef;

impl TypeRefRender for KotlinTypeRef {
    type Output = Result<TypeName>;

    fn primitive(&mut self, primitive: Primitive) -> Self::Output {
        KotlinType::primitive(primitive)
    }

    fn string(&mut self) -> Self::Output {
        Ok(TypeName::string())
    }

    fn bytes(&mut self) -> Self::Output {
        Ok(TypeName::byte_array(false))
    }

    fn record(&mut self, _id: RecordId) -> Self::Output {
        Err(Error::UnsupportedTarget {
            target: KOTLIN_TARGET,
            shape: "record type",
        })
    }

    fn enumeration(&mut self, _id: EnumId) -> Self::Output {
        Err(Error::UnsupportedTarget {
            target: KOTLIN_TARGET,
            shape: "enum type",
        })
    }

    fn class(&mut self, _id: ClassId) -> Self::Output {
        Err(Error::UnsupportedTarget {
            target: KOTLIN_TARGET,
            shape: "class type",
        })
    }

    fn callback(&mut self, _id: CallbackId) -> Self::Output {
        Err(Error::UnsupportedTarget {
            target: KOTLIN_TARGET,
            shape: "callback type",
        })
    }

    fn custom(&mut self, _id: CustomTypeId) -> Self::Output {
        Err(Error::UnsupportedTarget {
            target: KOTLIN_TARGET,
            shape: "custom type",
        })
    }

    fn builtin(&mut self, _kind: BuiltinType) -> Self::Output {
        Err(Error::UnsupportedTarget {
            target: KOTLIN_TARGET,
            shape: "builtin type",
        })
    }

    fn optional(&mut self, inner: Self::Output) -> Self::Output {
        inner.map(TypeName::nullable)
    }

    fn sequence(&mut self, element: Self::Output) -> Self::Output {
        element.map(TypeName::list)
    }

    fn tuple(&mut self, _elements: Vec<Self::Output>) -> Self::Output {
        Err(Error::UnsupportedTarget {
            target: KOTLIN_TARGET,
            shape: "tuple type",
        })
    }

    fn result(&mut self, _ok: Self::Output, _err: Self::Output) -> Self::Output {
        Err(Error::UnsupportedTarget {
            target: KOTLIN_TARGET,
            shape: "result type",
        })
    }

    fn map(&mut self, _key: Self::Output, _value: Self::Output) -> Self::Output {
        Err(Error::UnsupportedTarget {
            target: KOTLIN_TARGET,
            shape: "map type",
        })
    }
}

pub struct ParameterType;

impl<'plan> ParamPlanRender<'plan, Native, IntoRust> for ParameterType {
    type Output = Result<TypeName>;

    fn direct(
        &mut self,
        ty: &'plan DirectValueType,
        _receive: <IntoRust as Direction>::Receive,
    ) -> Self::Output {
        match ty {
            DirectValueType::Primitive(primitive) => KotlinType::primitive(*primitive),
            DirectValueType::Record(_) => Err(Error::UnsupportedTarget {
                target: KOTLIN_TARGET,
                shape: "direct record function parameter",
            }),
            DirectValueType::Enum(_) => Err(Error::UnsupportedTarget {
                target: KOTLIN_TARGET,
                shape: "direct enum function parameter",
            }),
            _ => Err(Error::UnsupportedTarget {
                target: KOTLIN_TARGET,
                shape: "unknown direct function parameter",
            }),
        }
    }

    fn encoded(
        &mut self,
        ty: &'plan TypeRef,
        _codec: &'plan <IntoRust as Direction>::Codec,
        _shape: <Native as boltffi_binding::Surface>::BufferShape,
        _receive: <IntoRust as Direction>::Receive,
    ) -> Self::Output {
        KotlinType::type_ref(ty)
    }

    fn handle(
        &mut self,
        _target: &'plan HandleTarget,
        _carrier: <Native as boltffi_binding::Surface>::HandleCarrier,
        _presence: HandlePresence,
        _receive: <IntoRust as Direction>::Receive,
    ) -> Self::Output {
        Err(Error::UnsupportedTarget {
            target: KOTLIN_TARGET,
            shape: "handle function parameter",
        })
    }

    fn scalar_option(&mut self, _primitive: Primitive) -> Self::Output {
        Err(Error::UnsupportedTarget {
            target: KOTLIN_TARGET,
            shape: "optional scalar function parameter",
        })
    }

    fn direct_vector(
        &mut self,
        _element: &'plan boltffi_binding::DirectVectorElementType,
    ) -> Self::Output {
        Err(Error::UnsupportedTarget {
            target: KOTLIN_TARGET,
            shape: "direct-vector function parameter",
        })
    }
}
