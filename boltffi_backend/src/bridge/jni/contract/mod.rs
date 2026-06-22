//! Typed JNI contract built from the C bridge contract.
//!
//! The C bridge has already decided the ABI: functions, callbacks, streams,
//! handles, byte buffers, direct records, and async completion slots. JNI needs a
//! different view of those same facts: Java parameter types, JNI descriptors,
//! local array borrows, cached method ids, and generated `Java_*` symbols.
//!
//! This module is that translation boundary. It does not lower Rust declarations
//! again and it does not inspect the original AST. It reads the C bridge contract
//! once, validates the shapes JNI can represent, and gives templates typed values
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
