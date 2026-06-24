use boltffi_binding::{DirectValueType, FieldKey, IntrinsicOp, OpRender, ValueRef, ValueRoot};

use crate::{
    core::{Error, Result},
    target::kotlin::{
        codec::value::ValueExpression,
        primitive::KotlinPrimitive,
        syntax::{ArgumentList, Expression, Identifier},
    },
};

pub struct Operation {
    current_value: ValueRef,
    current_expression: Expression,
}

impl Operation {
    pub fn new(current_value: &ValueRef, current_expression: Expression) -> Self {
        Self {
            current_value: current_value.clone(),
            current_expression,
        }
    }

    fn value_ref(&self, value: &ValueRef) -> ValueRef {
        match value.root() {
            ValueRoot::SelfValue => self.current_value.clone(),
            _ => value.clone(),
        }
    }

    fn single_argument(args: Vec<Result<Expression>>) -> Result<Expression> {
        let mut args = args.into_iter().collect::<Result<Vec<_>>>()?;
        match args.len() {
            1 => Ok(args.remove(0)),
            _ => Err(Error::UnsupportedTarget {
                target: "kotlin",
                shape: "kotlin operation with invalid arity",
            }),
        }
    }
}

impl OpRender for Operation {
    type Expr = Result<Expression>;

    fn value(&mut self, value: &ValueRef) -> Self::Expr {
        ValueExpression::new(&self.value_ref(value))?
            .current(self.current_expression.clone())
            .render()
    }

    fn byte_count(&mut self, bytes: u64) -> Self::Expr {
        Ok(Expression::integer(bytes))
    }

    fn integer(&mut self, value: i128) -> Self::Expr {
        Ok(Expression::integer(value))
    }

    fn add(&mut self, left: Self::Expr, right: Self::Expr) -> Self::Expr {
        Ok(left?.add(right?))
    }

    fn mul(&mut self, left: Self::Expr, right: Self::Expr) -> Self::Expr {
        Ok(left?.multiply(right?))
    }

    fn eq(&mut self, left: Self::Expr, right: Self::Expr) -> Self::Expr {
        Ok(left?.equal(right?))
    }

    fn field(&mut self, base: Self::Expr, field: &FieldKey) -> Self::Expr {
        ValueExpression::field(base?, field)
    }

    fn intrinsic(&mut self, intrinsic: IntrinsicOp, args: Vec<Self::Expr>) -> Self::Expr {
        let value = Self::single_argument(args)?;
        match intrinsic {
            IntrinsicOp::Utf8ByteCount => Ok(Expression::call(
                "Utf8Codec",
                Identifier::parse("maxBytes")?,
                [value].into_iter().collect::<ArgumentList>(),
            )),
            IntrinsicOp::SequenceLen => Ok(Expression::property(value, Identifier::parse("size")?)),
            IntrinsicOp::WireSize => Ok(Expression::call(
                value,
                Identifier::parse("wireSize")?,
                ArgumentList::default(),
            )),
            _ => Err(Error::UnsupportedTarget {
                target: "kotlin",
                shape: "unknown kotlin operation",
            }),
        }
    }

    fn size_of(&mut self, ty: &DirectValueType) -> Self::Expr {
        match ty {
            DirectValueType::Primitive(primitive) => KotlinPrimitive::new(*primitive)
                .wire_size()
                .map(Expression::integer),
            DirectValueType::Enum(_) | DirectValueType::Record(_) => {
                Err(Error::UnsupportedTarget {
                    target: "kotlin",
                    shape: "kotlin type-size operation",
                })
            }
            _ => Err(Error::UnsupportedTarget {
                target: "kotlin",
                shape: "unknown kotlin type-size operation",
            }),
        }
    }
}
