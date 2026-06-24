use boltffi_binding::{
    DirectValueType, Direction, HandlePresence, HandleTarget, IntoRust, Native, OutOfRust,
    ParamPlanRender, Primitive, ReturnPlanRender, ReturnValueSlot, TypeRef,
};

use crate::{
    bridge::jni::{DirectVectorParameter, JniType, NativeParameterKind, NativeReturn},
    core::{Error, Result},
    target::kotlin::syntax::TypeName,
};

const KOTLIN_TARGET: &str = "kotlin";

pub struct KotlinType;

impl KotlinType {
    pub fn primitive(primitive: Primitive) -> Result<TypeName> {
        match primitive {
            Primitive::Bool => Ok(TypeName::boolean()),
            Primitive::I8 | Primitive::U8 => Ok(TypeName::byte()),
            Primitive::I16 | Primitive::U16 => Ok(TypeName::short()),
            Primitive::I32 | Primitive::U32 => Ok(TypeName::int()),
            Primitive::I64 | Primitive::U64 | Primitive::ISize | Primitive::USize => {
                Ok(TypeName::long())
            }
            Primitive::F32 => Ok(TypeName::float()),
            Primitive::F64 => Ok(TypeName::double()),
            _ => Err(Error::UnsupportedTarget {
                target: KOTLIN_TARGET,
                shape: "unknown primitive type",
            }),
        }
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

    fn direct_vector(parameter: &DirectVectorParameter) -> Result<TypeName> {
        Self::jni_array(parameter.jni_type())
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
        _ty: &'plan TypeRef,
        _codec: &'plan <IntoRust as Direction>::Codec,
        _shape: <Native as boltffi_binding::Surface>::BufferShape,
        _receive: <IntoRust as Direction>::Receive,
    ) -> Self::Output {
        Err(Error::UnsupportedTarget {
            target: KOTLIN_TARGET,
            shape: "encoded function parameter",
        })
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

pub struct ReturnType;

impl<'plan> ReturnPlanRender<'plan, Native, OutOfRust> for ReturnType {
    type Output = Result<Option<TypeName>>;

    fn void(&mut self) -> Self::Output {
        Ok(None)
    }

    fn direct(&mut self, slot: ReturnValueSlot, ty: &'plan DirectValueType) -> Self::Output {
        match (slot, ty) {
            (ReturnValueSlot::ReturnSlot, DirectValueType::Primitive(primitive)) => {
                KotlinType::primitive(*primitive).map(Some)
            }
            (ReturnValueSlot::ReturnSlot, DirectValueType::Record(_)) => {
                Err(Error::UnsupportedTarget {
                    target: KOTLIN_TARGET,
                    shape: "direct record function return",
                })
            }
            (ReturnValueSlot::ReturnSlot, DirectValueType::Enum(_)) => {
                Err(Error::UnsupportedTarget {
                    target: KOTLIN_TARGET,
                    shape: "direct enum function return",
                })
            }
            (ReturnValueSlot::OutPointer, _) => Err(Error::UnsupportedTarget {
                target: KOTLIN_TARGET,
                shape: "out-pointer function return",
            }),
            _ => Err(Error::UnsupportedTarget {
                target: KOTLIN_TARGET,
                shape: "unknown direct function return",
            }),
        }
    }

    fn encoded(
        &mut self,
        _slot: ReturnValueSlot,
        _ty: &'plan TypeRef,
        _codec: &'plan <OutOfRust as Direction>::Codec,
        _shape: <Native as boltffi_binding::Surface>::BufferShape,
    ) -> Self::Output {
        Err(Error::UnsupportedTarget {
            target: KOTLIN_TARGET,
            shape: "encoded function return",
        })
    }

    fn handle(
        &mut self,
        _slot: ReturnValueSlot,
        _target: &'plan HandleTarget,
        _carrier: <Native as boltffi_binding::Surface>::HandleCarrier,
        _presence: HandlePresence,
    ) -> Self::Output {
        Err(Error::UnsupportedTarget {
            target: KOTLIN_TARGET,
            shape: "handle function return",
        })
    }

    fn scalar_option(&mut self, _primitive: Primitive) -> Self::Output {
        Err(Error::UnsupportedTarget {
            target: KOTLIN_TARGET,
            shape: "optional scalar function return",
        })
    }

    fn direct_vector(
        &mut self,
        _element: &'plan boltffi_binding::DirectVectorElementType,
    ) -> Self::Output {
        Err(Error::UnsupportedTarget {
            target: KOTLIN_TARGET,
            shape: "direct-vector function return",
        })
    }

    fn closure(
        &mut self,
        _closure: &'plan boltffi_binding::ClosureReturn<Native, OutOfRust>,
    ) -> Self::Output {
        Err(Error::UnsupportedTarget {
            target: KOTLIN_TARGET,
            shape: "closure function return",
        })
    }
}
