use crate::bridge::{c::Identifier, jni::ClosureHandleArgument};

pub struct ClosureHandleArgumentView {
    pub handle: Identifier,
    pub call: Identifier,
    pub context: Identifier,
    pub release: Identifier,
    pub handle_new: Identifier,
    pub handle_release: Identifier,
}

impl ClosureHandleArgumentView {
    pub fn from_argument(argument: &ClosureHandleArgument) -> Self {
        Self {
            handle: argument.handle().clone(),
            call: argument.call().clone(),
            context: argument.context().clone(),
            release: argument.release().clone(),
            handle_new: argument.handle_new().clone(),
            handle_release: argument.handle_release().clone(),
        }
    }
}
