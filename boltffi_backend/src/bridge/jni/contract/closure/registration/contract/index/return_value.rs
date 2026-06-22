//! Closure registrations discovered inside return groups.
//!
//! Returned closures are represented by C out-parameter groups rather than by a
//! direct return value. The registration index still needs the closure signature
//! so the generated bridge class and helper methods exist before a return path
//! references them.
//!
//! This module extracts that signature from C return groups and inserts it into
//! the shared closure registration index.

use crate::{
    bridge::{c, jni::JvmClassPath},
    core::Result,
};

use super::super::build::ClosureRegistrationConstructor;
use super::ClosureRegistrationIndex;

impl ClosureRegistrationIndex {
    pub fn insert_closure_return(
        &mut self,
        class: &JvmClassPath,
        returned: &c::ClosureReturnParameter,
        callbacks: &[c::Callback],
    ) -> Result<()> {
        let inserted = if self.registrations.contains_key(returned.signature()) {
            false
        } else {
            self.registrations.insert(
                returned.signature().clone(),
                ClosureRegistrationConstructor::from_closure_return(class, returned, callbacks)?,
            );
            true
        };

        if inserted {
            returned
                .parameter_groups()
                .iter()
                .try_for_each(|group| -> Result<()> {
                    if let c::ParameterGroup::Closure(nested) = group {
                        self.insert_closure_parameter(
                            class,
                            returned.parameter(nested.call()).ty(),
                            nested,
                            true,
                            callbacks,
                        )?;
                    }
                    Ok(())
                })?;
        }

        Ok(())
    }
}
