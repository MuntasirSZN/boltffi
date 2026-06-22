use crate::{
    bridge::{
        c::{self, Expression, Identifier, TypeFragment},
        jni::JniType,
    },
    core::Result,
};

/// Scalar JNI parameter mapped to one scalar C bridge argument.
#[derive(Clone, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub struct ScalarParameter {
    name: Identifier,
    c_type: c::Type,
    jni_type: JniType,
}

/// Scalar JNI return value mapped from one scalar C bridge return.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
#[non_exhaustive]
pub struct ScalarReturn {
    c_type: c::Type,
    jni_type: JniType,
}

impl ScalarParameter {
    /// Returns the generated C parameter name.
    pub fn name(&self) -> &Identifier {
        &self.name
    }

    /// Returns the scalar JNI parameter type.
    pub fn ty(&self) -> JniType {
        self.jni_type
    }

    /// Returns the expression passed to the C bridge function.
    pub fn c_argument(&self) -> Result<Expression> {
        let value = Expression::identifier(self.name.clone());
        match self.needs_c_cast() {
            true => {
                TypeFragment::anonymous(&self.c_type).map(|c_type| Expression::cast(c_type, value))
            }
            false => Ok(value),
        }
    }

    /// Creates a scalar JNI parameter from one scalar C ABI parameter.
    pub fn from_c_parameter(parameter: &c::Parameter) -> Result<Self> {
        Ok(Self {
            name: Identifier::escape(parameter.name())?,
            c_type: parameter.ty().clone(),
            jni_type: JniType::from_c_type(parameter.ty())?,
        })
    }

    fn needs_c_cast(&self) -> bool {
        matches!(
            self.c_type,
            c::Type::CStyleEnum { .. }
                | c::Type::FutureHandle
                | c::Type::StreamPollResult
                | c::Type::WaitResult
                | c::Type::ConstPointer(_)
                | c::Type::MutPointer(_)
                | c::Type::FunctionPointer { .. }
        )
    }
}

impl ScalarReturn {
    /// Returns the scalar JNI return type.
    pub fn jni_type(&self) -> JniType {
        self.jni_type
    }

    /// Returns the C bridge result type used by the temporary result variable.
    pub fn c_result_type(&self) -> Result<TypeFragment> {
        TypeFragment::anonymous(&self.c_type)
    }

    /// Returns the expression returned from the JNI method.
    pub fn return_expression(&self, value: Expression) -> Expression {
        Expression::cast(self.jni_type.as_type_fragment(), value)
    }

    /// Creates a scalar JNI return from one scalar C ABI return type.
    pub fn from_c_type(ty: &c::Type) -> Result<Self> {
        Ok(Self {
            c_type: ty.clone(),
            jni_type: JniType::from_c_type(ty)?,
        })
    }
}
