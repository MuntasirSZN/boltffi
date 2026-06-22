use crate::bridge::{
    c::{Identifier, TypeFragment},
    jni::NativeParameter,
};

pub struct NativeParameterView {
    pub name: Identifier,
    pub ty: TypeFragment,
}

impl NativeParameterView {
    pub fn from_parameter(parameter: &NativeParameter) -> Self {
        Self {
            name: parameter.name().clone(),
            ty: parameter.ty(),
        }
    }
}
