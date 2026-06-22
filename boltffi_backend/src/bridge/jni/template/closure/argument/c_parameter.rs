use crate::bridge::{c::Statement, jni::ClosureCParameter};

pub struct ClosureCParameterView {
    pub declaration: Statement,
}

impl ClosureCParameterView {
    pub fn from_parameter(parameter: ClosureCParameter) -> Self {
        Self {
            declaration: parameter.declaration().clone(),
        }
    }
}
