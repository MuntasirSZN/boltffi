use crate::bridge::c::Identifier;

/// Borrowed byte-array argument passed from Rust into a JVM callback method.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub struct CallbackBytesArgument<'argument> {
    name: &'argument Identifier,
    pointer: &'argument Identifier,
    length: &'argument Identifier,
}

impl<'argument> CallbackBytesArgument<'argument> {
    pub(in crate::bridge::jni::contract::callback) fn new(
        name: &'argument Identifier,
        pointer: &'argument Identifier,
        length: &'argument Identifier,
    ) -> Self {
        Self {
            name,
            pointer,
            length,
        }
    }

    /// Returns the local JNI byte-array variable.
    pub fn name(&self) -> &Identifier {
        self.name
    }

    /// Returns the C byte pointer parameter.
    pub fn pointer(&self) -> &Identifier {
        self.pointer
    }

    /// Returns the C byte length parameter.
    pub fn length(&self) -> &Identifier {
        self.length
    }
}
