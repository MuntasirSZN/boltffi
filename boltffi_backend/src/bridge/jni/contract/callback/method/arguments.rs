use crate::bridge::{
    c::ArgumentList,
    jni::{
        CallbackArgument, CallbackBytesArgument, CallbackCParameter, CallbackClosureArgument,
        CallbackCompletionArgument, CallbackDirectVectorArgument, CallbackHandleArgument,
        CallbackRecordArgument,
    },
};

use super::CallbackMethod;

impl CallbackMethod {
    /// Returns generated C parameters.
    pub fn c_parameters(&self) -> Vec<CallbackCParameter> {
        self.arguments
            .iter()
            .flat_map(CallbackArgument::c_parameters)
            .collect()
    }

    /// Returns the arguments passed to the static JVM callback method.
    pub fn jni_arguments(&self) -> ArgumentList {
        ArgumentList::from_iter(
            self.arguments
                .iter()
                .flat_map(CallbackArgument::jni_arguments),
        )
    }

    /// Returns byte-array callback arguments.
    pub fn byte_arrays(&self) -> Vec<CallbackBytesArgument<'_>> {
        self.arguments
            .iter()
            .filter_map(CallbackArgument::bytes)
            .collect()
    }

    /// Returns direct-vector callback arguments.
    pub fn direct_vectors(&self) -> Vec<CallbackDirectVectorArgument<'_>> {
        self.arguments
            .iter()
            .filter_map(CallbackArgument::direct_vector)
            .collect()
    }

    /// Returns direct-record callback arguments.
    pub fn record_arrays(&self) -> Vec<CallbackRecordArgument<'_>> {
        self.arguments
            .iter()
            .filter_map(CallbackArgument::record)
            .collect()
    }

    /// Returns callback-handle callback arguments.
    pub fn callback_handles(&self) -> Vec<CallbackHandleArgument<'_>> {
        self.arguments
            .iter()
            .filter_map(CallbackArgument::callback_handle)
            .collect()
    }

    /// Returns closure-handle callback arguments.
    pub fn closure_handles(&self) -> Vec<CallbackClosureArgument<'_>> {
        self.arguments
            .iter()
            .filter_map(CallbackArgument::closure_handle)
            .collect()
    }

    /// Returns async callback completion arguments.
    pub fn completions(&self) -> Vec<CallbackCompletionArgument<'_>> {
        self.arguments
            .iter()
            .filter_map(CallbackArgument::completion)
            .collect()
    }
}
