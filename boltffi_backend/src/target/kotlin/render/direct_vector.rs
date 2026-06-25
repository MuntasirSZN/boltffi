use boltffi_binding::{DirectVectorElementType, DirectVectorPrimitive, Primitive};

use crate::{
    core::{Error, Result},
    target::kotlin::{
        primitive::KotlinPrimitive,
        syntax::{ArgumentList, Expression, Identifier, Literal, Statement, TypeName},
    },
};

const KOTLIN_TARGET: &str = "kotlin";

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DirectVector {
    ty: TypeName,
    decoder: Identifier,
    carrier_conversion: Option<Identifier>,
}

impl DirectVector {
    pub fn from_element(element: &DirectVectorElementType) -> Result<Self> {
        match element {
            DirectVectorElementType::Primitive(primitive) => Self::from_primitive(*primitive),
            DirectVectorElementType::Record(_) => Err(Error::UnsupportedTarget {
                target: KOTLIN_TARGET,
                shape: "direct-record vector type",
            }),
            _ => Err(Error::UnsupportedTarget {
                target: KOTLIN_TARGET,
                shape: "unknown direct-vector type",
            }),
        }
    }

    pub fn ty(&self) -> &TypeName {
        &self.ty
    }

    pub fn value_statements(&self, call: Expression) -> Result<Vec<Statement>> {
        let result = Identifier::parse("__boltffi_result")?;
        let payload = call.or_else(Expression::throw_illegal_state(Literal::string(
            "null buffer returned",
        )));
        let decoded = Expression::call(
            "DirectVectorCodec",
            self.decoder.clone(),
            [Expression::identifier(result.clone())]
                .into_iter()
                .collect::<ArgumentList>(),
        );
        Ok(vec![
            Statement::value(result, payload),
            Statement::expression(decoded),
        ])
    }

    pub fn carrier_expression(&self, value: Expression) -> Expression {
        match &self.carrier_conversion {
            Some(method) => value.convert(method.clone()),
            None => value,
        }
    }

    fn from_primitive(primitive: DirectVectorPrimitive) -> Result<Self> {
        let primitive = primitive.primitive();
        Ok(Self {
            ty: KotlinPrimitive::new(primitive).array_type()?,
            decoder: Identifier::parse(Self::decoder_name(primitive)?)?,
            carrier_conversion: Self::carrier_conversion_name(primitive)
                .map(Identifier::parse)
                .transpose()?,
        })
    }

    fn decoder_name(primitive: Primitive) -> Result<&'static str> {
        match primitive {
            Primitive::Bool => Ok("readBooleanArray"),
            Primitive::I8 => Ok("readByteArray"),
            Primitive::I16 => Ok("readShortArray"),
            Primitive::I32 => Ok("readIntArray"),
            Primitive::I64 | Primitive::ISize => Ok("readLongArray"),
            Primitive::F32 => Ok("readFloatArray"),
            Primitive::F64 => Ok("readDoubleArray"),
            Primitive::U16 => Ok("readUShortArray"),
            Primitive::U32 => Ok("readUIntArray"),
            Primitive::U64 | Primitive::USize => Ok("readULongArray"),
            Primitive::U8 => Err(Error::UnsupportedTarget {
                target: KOTLIN_TARGET,
                shape: "u8 direct-vector primitive",
            }),
            _ => Err(Error::UnsupportedTarget {
                target: KOTLIN_TARGET,
                shape: "unknown direct-vector primitive",
            }),
        }
    }

    fn carrier_conversion_name(primitive: Primitive) -> Option<&'static str> {
        match primitive {
            Primitive::U16 => Some("toShortArray"),
            Primitive::U32 => Some("toIntArray"),
            Primitive::U64 | Primitive::USize => Some("toLongArray"),
            _ => None,
        }
    }
}
