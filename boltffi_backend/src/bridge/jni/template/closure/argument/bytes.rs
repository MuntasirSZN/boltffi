use crate::bridge::{c::Identifier, jni::ClosureBytesArgument};

pub struct ClosureBytesArgumentView {
    pub name: Identifier,
    pub pointer: Identifier,
    pub length: Identifier,
    pub buffer: Identifier,
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
