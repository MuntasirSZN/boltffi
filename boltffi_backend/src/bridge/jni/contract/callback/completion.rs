use crate::bridge::c::{ArgumentList, Identifier};

/// Completion callback invoked when async JVM callback dispatch fails.
#[derive(Clone, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub struct CallbackCompletionArgument<'argument> {
    callback: &'argument Identifier,
    failure_arguments: ArgumentList,
}

impl<'argument> CallbackCompletionArgument<'argument> {
    pub(in crate::bridge::jni::contract::callback) fn new(
        callback: &'argument Identifier,
        failure_arguments: ArgumentList,
    ) -> Self {
        Self {
            callback,
            failure_arguments,
        }
    }

    /// Returns the C completion callback parameter.
    pub fn callback(&self) -> &Identifier {
        self.callback
    }

    /// Returns arguments that complete the async callback with failure.
    pub fn failure_arguments(&self) -> &ArgumentList {
        &self.failure_arguments
    }
}
