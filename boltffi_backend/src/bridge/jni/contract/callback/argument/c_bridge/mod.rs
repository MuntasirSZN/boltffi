//! Conversion from C callback parameter groups into callback arguments.
//!
//! The C bridge has already grouped raw callback slot parameters by meaning.
//! This module maps each group into one typed `CallbackArgument`, keeping the
//! validation close to the C shape that produced it.

mod bytes;
mod closure;
mod completion;
mod direct_vector;
mod value;

use crate::{
    bridge::{c, jni::ClosureRegistration},
    core::{Error, Result},
};

use super::CallbackArgument;

const JNI_BRIDGE: &str = "jni";

impl CallbackArgument {
    pub(in crate::bridge::jni::contract::callback) fn from_group(
        slot: &c::CallbackSlot,
        group: &c::ParameterGroup,
        callbacks: &[c::Callback],
        closures: &[ClosureRegistration],
    ) -> Result<Self> {
        match group {
            c::ParameterGroup::Value(index) => value::from_parameter(slot.parameter(*index)),
            c::ParameterGroup::ByteSlice(slice) => bytes::from_group(slot, slice),
            c::ParameterGroup::DirectVector(vector) => direct_vector::from_group(slot, vector),
            c::ParameterGroup::DirectWriteback(_) => Err(Error::BrokenBridgeContract {
                bridge: JNI_BRIDGE,
                invariant: "callback method argument cannot be a direct-record writeback",
            }),
            c::ParameterGroup::CallbackCompletion(completion) => {
                completion::from_group(slot, completion, callbacks)
            }
            c::ParameterGroup::Closure(closure) => closure::from_group(slot, closure, closures),
            c::ParameterGroup::ClosureReturn(_) => Err(Error::BrokenBridgeContract {
                bridge: JNI_BRIDGE,
                invariant: "callback method argument cannot be a closure return out-pointer",
            }),
            c::ParameterGroup::Continuation(_) => Err(Error::UnsupportedBridge {
                bridge: JNI_BRIDGE,
                shape: "callback continuation parameter",
            }),
        }
    }
}
