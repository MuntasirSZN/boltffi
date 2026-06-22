//! Callback method arguments passed from Rust into the JVM.
//!
//! These arguments start as C callback slot parameters. The JNI bridge groups
//! them into the Java values that the static callback method expects: scalars,
//! byte arrays, direct vectors, direct records, callback handles, closure
//! handles, and async completion callbacks.

mod c_bridge;
mod jvm;
mod jvm_setup;

use crate::bridge::{
    c::Identifier,
    jni::{CallbackCParameter, CallbackCompletionPayload, JniType},
};

/// One callback vtable argument forwarded to a JVM callback method.
#[derive(Clone, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub struct CallbackArgument {
    kind: CallbackArgumentKind,
}

#[derive(Clone, Debug, Eq, PartialEq)]
enum CallbackArgumentKind {
    Value {
        parameter: CallbackCParameter,
        jni_type: JniType,
    },
    Bytes {
        name: Identifier,
        pointer: CallbackCParameter,
        length: CallbackCParameter,
    },
    DirectVector {
        array: Identifier,
        pointer: CallbackCParameter,
        length: CallbackCParameter,
        jni_type: JniType,
    },
    Record {
        array: Identifier,
        parameter: CallbackCParameter,
    },
    CallbackHandle {
        handle: Identifier,
        parameter: CallbackCParameter,
    },
    Closure {
        handle: Identifier,
        call: CallbackCParameter,
        context: CallbackCParameter,
        release: CallbackCParameter,
        handle_new: Identifier,
        handle_release: Identifier,
    },
    Completion {
        callback: CallbackCParameter,
        context: CallbackCParameter,
        payload: Option<CallbackCompletionPayload>,
    },
}
