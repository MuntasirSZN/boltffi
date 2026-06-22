use crate::bridge::{
    c::{Identifier, Statement, TypeFragment},
    jni::{ClosureBytesArgument, ClosureCParameter, ClosureDirectVectorArgument},
};

pub struct ClosureCParameterView {
    pub declaration: Statement,
}

pub struct ClosureBytesArgumentView {
    pub name: Identifier,
    pub pointer: Identifier,
    pub length: Identifier,
    pub buffer: Identifier,
}

pub struct ClosureDirectVectorArgumentView {
    pub name: Identifier,
    pub pointer: Identifier,
    pub length: Identifier,
    pub pointer_local: Identifier,
    pub length_local: Identifier,
    pub array_type: TypeFragment,
    pub element_type: TypeFragment,
    pub new_array: &'static str,
    pub set_region: &'static str,
    pub getter: &'static str,
    pub releaser: &'static str,
}

impl ClosureCParameterView {
    pub fn from_parameter(parameter: ClosureCParameter) -> Self {
        Self {
            declaration: parameter.declaration().clone(),
        }
    }
}

impl ClosureBytesArgumentView {
    pub fn from_argument(argument: &ClosureBytesArgument) -> Self {
        Self {
            name: argument.name().clone(),
            pointer: argument.pointer().clone(),
            length: argument.length().clone(),
            buffer: argument.buffer().clone(),
        }
    }
}

impl ClosureDirectVectorArgumentView {
    pub fn from_argument(argument: &ClosureDirectVectorArgument) -> Self {
        Self {
            name: argument.name().clone(),
            pointer: argument.pointer().clone(),
            length: argument.length().clone(),
            pointer_local: argument.pointer_local().clone(),
            length_local: argument.length_local().clone(),
            array_type: argument.array_type(),
            element_type: argument.element_type(),
            new_array: argument.new_array(),
            set_region: argument.set_region(),
            getter: argument.getter(),
            releaser: argument.releaser(),
        }
    }
}
