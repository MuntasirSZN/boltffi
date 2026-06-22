//! Template data for callback vtable glue.
//!
//! Callback methods are called by Rust through C vtables, then forwarded into
//! static JVM methods. These views collect the JNI arguments, setup arrays,
//! callback handle conversions, async completion invokers, and returned closure
//! storage needed by the callback templates.

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
