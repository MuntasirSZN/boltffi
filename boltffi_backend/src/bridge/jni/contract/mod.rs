//! Typed JNI contract built from the C bridge contract.
//!
//! The C bridge has already decided ABI shape: functions, callbacks, streams,
//! handles, byte buffers, direct records, and async completion slots. This module
//! translates those facts into the JNI surface that a JVM host needs to call.
//!
//! The important boundary is that this layer does not lower Rust declarations
//! again. It reads the C bridge contract and gives templates typed JNI concepts
//! instead of raw strings.

mod bridge;
mod bytes;
mod callback;
mod closure;
mod continuation;
mod direct_vector;
mod jni_type;
mod jvm;
mod method;
mod parameter;
mod record;
mod return_value;
mod scalar;
mod stream;

pub use bridge::JniBridgeContract;
pub use bytes::BytesParameter;
pub use callback::{
    CallbackArgument, CallbackBytesArgument, CallbackCParameter, CallbackClosureArgument,
    CallbackClosureReturn, CallbackCompletionArgument, CallbackCompletionInvoker,
    CallbackCompletionPayload, CallbackDirectVectorArgument, CallbackHandleArgument,
    CallbackMethod, CallbackParameter, CallbackRecordArgument, CallbackRegistration,
    CallbackReturn,
};
pub use closure::{
    CallbackClosureHandle, ClosureArgument, ClosureBytesArgument, ClosureCParameter,
    ClosureDirectVectorArgument, ClosureHandleArgument, ClosureParameter, ClosureRegistration,
};
pub use continuation::ContinuationParameter;
pub use direct_vector::DirectVectorParameter;
pub use jni_type::JniType;
pub use jvm::JvmMethodReturn;
pub use method::NativeMethod;
pub use parameter::{NativeParameter, NativeParameterKind};
pub use record::{RecordParameter, RecordValue};
pub use return_value::NativeReturn;
pub use scalar::{ScalarParameter, ScalarReturn};
pub use stream::{DirectStreamBatchMethod, StreamProtocolMethods};
