//! Template data for callback vtable dispatch into the JVM.
//!
//! The callback contract describes a C vtable slot and the JVM static method it
//! calls. The template needs a source-ready view of that contract: C
//! declarations, local JNI setup, Java method arguments, return conversion,
//! cleanup, and async completion invokers.
//!
//! This module is the rendering adapter for callback glue. It keeps C syntax out
//! of the contract layer while still making templates consume typed data instead
//! of rebuilding callback behavior.

mod argument;
mod closure_return;
mod completion;
mod direct_vector;
mod method;
mod registration;

pub use argument::{
    CallbackBytesArgumentView, CallbackCParameterView, CallbackClosureArgumentView,
    CallbackCompletionArgumentView, CallbackHandleArgumentView, CallbackRecordArgumentView,
};
pub use closure_return::CallbackClosureReturnView;
pub use completion::CallbackCompletionInvokerView;
pub use direct_vector::CallbackDirectVectorArgumentView;
pub use method::CallbackMethodView;
pub use registration::CallbackRegistrationView;
