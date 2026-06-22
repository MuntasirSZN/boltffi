//! JNI bridge for JVM targets.
//!
//! The C bridge owns the Rust ABI. A JVM cannot call that ABI directly because
//! it speaks in JNI symbols, Java arrays, method descriptors, cached class
//! references, and `JNIEnv` lifetimes. This bridge is the layer that makes those
//! two worlds meet.
//!
//! The bridge reads the C bridge contract and produces one JNI source file with
//! `Java_*` entry points, callback vtables, closure trampolines, stream helpers,
//! async continuation hooks, and lifecycle code. Java and Kotlin targets compose
//! with this contract instead of rediscovering JNI naming, parameter grouping, or
//! callback ownership locally.

mod bridge;
mod contract;
mod name;
mod template;

pub use bridge::JniBridge;
pub use contract::{
    BytesParameter, CallbackArgument, CallbackBytesArgument, CallbackCParameter,
    CallbackClosureArgument, CallbackClosureHandle, CallbackClosureReturn,
    CallbackCompletionArgument, CallbackCompletionInvoker, CallbackCompletionPayload,
    CallbackDirectVectorArgument, CallbackHandleArgument, CallbackMethod, CallbackParameter,
    CallbackRecordArgument, CallbackRegistration, CallbackReturn, ClosureArgument,
    ClosureBytesArgument, ClosureCParameter, ClosureDirectVectorArgument, ClosureHandleArgument,
    ClosureParameter, ClosureRegistration, ContinuationParameter, DirectStreamBatchMethod,
    DirectVectorParameter, JniBridgeContract, JniType, JvmMethodReturn, NativeMethod,
    NativeParameter, NativeParameterKind, NativeReturn, RecordParameter, RecordValue,
    ScalarParameter, ScalarReturn,
};
pub use name::{JniSymbolName, JvmClassPath, JvmNameSegment};

#[cfg(test)]
mod tests;
