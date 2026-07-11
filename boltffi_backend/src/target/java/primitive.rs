use std::fmt;

use boltffi_binding::Primitive as BindingPrimitive;

use crate::{
    bridge::jni::JniType,
    core::{Error, Result},
    target::java::syntax::{ArgumentList, Expression, Identifier, TypeIdentifier, TypeName},
    target::jvm::method::{Parameter, SlotWidth},
};

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum Primitive {
    Boolean,
    Byte,
    Short,
    Int,
    Long,
    Float,
    Double,
}

impl Primitive {
    pub const fn wire_size(self) -> u64 {
        match self {
            Self::Boolean | Self::Byte => 1,
            Self::Short => 2,
            Self::Int | Self::Float => 4,
            Self::Long | Self::Double => 8,
        }
    }

    pub const fn wire_method_suffix(self) -> &'static str {
        match self {
            Self::Boolean => "Boolean",
            Self::Byte => "Byte",
            Self::Short => "Short",
            Self::Int => "Int",
            Self::Long => "Long",
            Self::Float => "Float",
            Self::Double => "Double",
        }
    }

    pub const fn slot_width(self) -> SlotWidth {
        match self {
            Self::Long | Self::Double => SlotWidth::Double,
            Self::Boolean | Self::Byte | Self::Short | Self::Int | Self::Float => SlotWidth::Single,
        }
    }

    pub fn buffer_read(self, buffer: Expression, offset: u64) -> Expression {
        let offset = [Expression::integer(offset)]
            .into_iter()
            .collect::<ArgumentList>();
        match self {
            Self::Boolean => buffer
                .call(Identifier::known("get"), offset)
                .not_equal(Expression::integer(0)),
            Self::Byte => buffer.call(Identifier::known("get"), offset),
            Self::Short => buffer.call(Identifier::known("getShort"), offset),
            Self::Int => buffer.call(Identifier::known("getInt"), offset),
            Self::Long => buffer.call(Identifier::known("getLong"), offset),
            Self::Float => buffer.call(Identifier::known("getFloat"), offset),
            Self::Double => buffer.call(Identifier::known("getDouble"), offset),
        }
    }

    pub fn buffer_write(self, buffer: Expression, offset: u64, value: Expression) -> Expression {
        let (method, value) = match self {
            Self::Boolean => (
                Identifier::known("put"),
                Expression::cast(
                    Self::Byte,
                    value.conditional(Expression::integer(1), Expression::integer(0)),
                ),
            ),
            Self::Byte => (Identifier::known("put"), value),
            Self::Short => (Identifier::known("putShort"), value),
            Self::Int => (Identifier::known("putInt"), value),
            Self::Long => (Identifier::known("putLong"), value),
            Self::Float => (Identifier::known("putFloat"), value),
            Self::Double => (Identifier::known("putDouble"), value),
        };
        buffer.call(
            method,
            [Expression::integer(offset), value].into_iter().collect(),
        )
    }

    pub fn equals(self, left: Expression, right: Expression) -> Expression {
        match self {
            Self::Float | Self::Double => Expression::static_call(
                TypeName::named(TypeIdentifier::known(
                    match self {
                        Self::Float => "Float",
                        Self::Double => "Double",
                        _ => unreachable!(),
                    },
                    crate::target::java::JavaVersion::JAVA_8,
                )),
                Identifier::known("compare"),
                [left, right].into_iter().collect(),
            )
            .equal(Expression::integer(0)),
            _ => left.equal(right),
        }
    }

    pub fn hash(self, value: Expression) -> Expression {
        Expression::static_call(
            TypeName::named(TypeIdentifier::known(
                match self {
                    Self::Boolean => "Boolean",
                    Self::Byte => "Byte",
                    Self::Short => "Short",
                    Self::Int => "Integer",
                    Self::Long => "Long",
                    Self::Float => "Float",
                    Self::Double => "Double",
                },
                crate::target::java::JavaVersion::JAVA_8,
            )),
            Identifier::known("hashCode"),
            [value].into_iter().collect(),
        )
    }

    fn unsupported(shape: &'static str) -> Error {
        Error::UnsupportedTarget {
            target: "java",
            shape,
        }
    }
}

impl fmt::Display for Primitive {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::Boolean => "boolean",
            Self::Byte => "byte",
            Self::Short => "short",
            Self::Int => "int",
            Self::Long => "long",
            Self::Float => "float",
            Self::Double => "double",
        })
    }
}

impl Parameter for Primitive {
    fn slot_width(&self) -> SlotWidth {
        (*self).slot_width()
    }
}

impl TryFrom<BindingPrimitive> for Primitive {
    type Error = Error;

    fn try_from(primitive: BindingPrimitive) -> Result<Self> {
        match primitive {
            BindingPrimitive::Bool => Ok(Self::Boolean),
            BindingPrimitive::I8 | BindingPrimitive::U8 => Ok(Self::Byte),
            BindingPrimitive::I16 | BindingPrimitive::U16 => Ok(Self::Short),
            BindingPrimitive::I32 | BindingPrimitive::U32 => Ok(Self::Int),
            BindingPrimitive::I64
            | BindingPrimitive::U64
            | BindingPrimitive::ISize
            | BindingPrimitive::USize => Ok(Self::Long),
            BindingPrimitive::F32 => Ok(Self::Float),
            BindingPrimitive::F64 => Ok(Self::Double),
            _ => Err(Self::unsupported("primitive Java representation")),
        }
    }
}

impl From<JniType> for Primitive {
    fn from(jni_type: JniType) -> Self {
        match jni_type {
            JniType::Boolean => Self::Boolean,
            JniType::Byte => Self::Byte,
            JniType::Short => Self::Short,
            JniType::Int => Self::Int,
            JniType::Long => Self::Long,
            JniType::Float => Self::Float,
            JniType::Double => Self::Double,
        }
    }
}

#[cfg(test)]
mod tests {
    use boltffi_binding::Primitive as BindingPrimitive;

    use crate::{bridge::jni::JniType, target::jvm::method::SlotWidth};

    use super::Primitive;

    #[test]
    fn maps_public_and_jni_carrier_types() {
        let primitive = Primitive::try_from(BindingPrimitive::U32).unwrap();
        assert_eq!(primitive.to_string(), "int");
        assert_eq!(Primitive::from(JniType::Int), Primitive::Int);
        assert_eq!(primitive.slot_width(), SlotWidth::Single);
    }
}
