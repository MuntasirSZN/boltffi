//! Callback vtable method contract.
//!
//! Each callback trait method becomes one C vtable slot. The JNI bridge stores
//! the slot function, cached JVM method id, C parameters, JVM argument list, and
//! any special return handling needed by that slot.

mod closure_return;
mod contract;

pub use closure_return::CallbackClosureReturn;
pub use contract::CallbackMethod;
