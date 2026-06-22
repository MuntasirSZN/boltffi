//! JVM bridge layered on top of the C ABI bridge.
//!
//! The lower C bridge already describes the Rust-facing ABI: exported symbols,
//! record layouts, callback vtables, stream protocols, async continuations, and
//! owned buffers. A JVM target cannot call that contract directly. It needs
//! `Java_*` entry points, JNI descriptors, Java arrays, cached classes and
//! method ids, global references, and `JNIEnv`-scoped cleanup.
//!
//! This module owns that second bridge. It reads the C bridge contract once,
//! builds a typed JNI contract, and renders one C source file that the Java or
//! Kotlin target can compile beside the C bridge output. Host targets compose
//! with this bridge instead of rediscovering JNI naming, callback ownership, or
//! parameter grouping in their own renderers.

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
