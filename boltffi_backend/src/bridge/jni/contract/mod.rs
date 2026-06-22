//! Typed JNI view of the lower C bridge contract.
//!
//! The C bridge has already made the ABI decisions. It knows which symbols
//! exist, which parameters are byte buffers or direct records, which returns use
//! out-pointers, how callbacks are shaped, and how streams are driven. JNI needs
//! the same facts in a different form: Java parameter types, JNI descriptors,
//! borrowed array locals, cached method ids, generated `Java_*` symbols, and
//! cleanup obligations.
//!
//! This module is the translation boundary between those two views. It does not
//! lower Rust declarations again, inspect source syntax, or decide transport
//! rules from `TypeRef`. It reads the C bridge contract, validates that each C
//! shape has a JNI representation, and gives rendering code typed values rather
//! than loose strings.

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
