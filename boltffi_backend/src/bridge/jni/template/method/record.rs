use crate::bridge::{c::Identifier, jni::RecordParameter};

#[derive(Clone)]
pub struct RecordParameterView {
    pub name: Identifier,
    pub c_type: Identifier,
    pub local: Identifier,
}

impl RecordParameterView {
    pub fn from_record(parameter: &RecordParameter) -> Self {
        Self {
            name: parameter.name().clone(),
            c_type: parameter.c_type().clone(),
            local: parameter.local().clone(),
        }
    }
}
