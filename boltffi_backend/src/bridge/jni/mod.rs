//! JNI bridge for JVM targets.
//!
//! The C bridge owns the Rust ABI. This bridge adds the JVM-facing layer above it:
//! exported `Java_*` symbols, JNI type spellings, callback entry points, closure
//! trampolines, stream helpers, and the lifecycle hooks needed to cache JVM
//! method ids safely.
//!
//! Java and Kotlin targets should not rediscover those rules. They compose with
//! this bridge contract and render host code against the typed native-method
//! surface it exposes.

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
