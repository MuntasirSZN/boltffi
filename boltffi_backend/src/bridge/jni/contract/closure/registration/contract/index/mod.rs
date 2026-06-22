//! Closure registration index.
//!
//! The JNI bridge needs one registration per closure signature, even when the
//! signature appears in several functions, callback methods, or nested closure
//! arguments. This index walks those C bridge groups and keeps the deduplicated
//! registrations in signature order.

mod callback_method;
mod function;
mod parameter;
mod return_value;

use std::collections::BTreeMap;

use boltffi_binding::ClosureSignature;

use crate::{
    bridge::{c, jni::JvmClassPath},
    core::Result,
};

use super::ClosureRegistration;

#[derive(Default)]
pub struct ClosureRegistrationIndex {
    registrations: BTreeMap<ClosureSignature, ClosureRegistration>,
}

impl ClosureRegistrationIndex {
    pub fn from_c_bridge(
        class: &JvmClassPath,
        functions: &[c::Function],
        callbacks: &[c::Callback],
    ) -> Result<Self> {
        functions
            .iter()
            .try_fold(Self::default(), |index, function| {
                index.collect_function(class, function, callbacks)
            })
            .and_then(|index| {
                callbacks
                    .iter()
                    .flat_map(|callback| callback.methods().iter())
                    .try_fold(index, |index, slot| {
                        index.collect_callback_method(class, slot, callbacks)
                    })
            })
    }

    pub fn into_registrations(self) -> Vec<ClosureRegistration> {
        self.registrations.into_values().collect()
    }
}
