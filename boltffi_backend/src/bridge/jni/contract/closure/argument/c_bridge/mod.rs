//! Conversion from C closure parameter groups into closure arguments.
//!
//! The C bridge represents a closure call as a function pointer plus grouped
//! parameters. This module turns each group into one typed closure argument while
//! preserving the difference between normal closure parameters and returned
//! closure storage.

mod bytes;
mod direct_vector;
mod nested_closure;
mod scalar;

use crate::{
    bridge::{c, jni::JvmClassPath},
    core::{Error, Result},
};

use super::{ClosureArgument, ClosureArgumentKind};

const JNI_BRIDGE: &str = "jni";

enum ClosureCall<'source> {
    Parameter(&'source c::ClosureParameter),
    Return(&'source c::ClosureReturnParameter),
}

impl ClosureArgument {
    /// Builds a closure-call argument from a closure parameter group.
    pub fn from_closure_group(
        class: &JvmClassPath,
        closure: &c::ClosureParameter,
        group: &c::ParameterGroup,
    ) -> Result<Self> {
        Self::from_group(class, ClosureCall::Parameter(closure), group)
    }

    /// Builds a closure-call argument from a closure return storage group.
    pub fn from_return_group(
        class: &JvmClassPath,
        returned: &c::ClosureReturnParameter,
        group: &c::ParameterGroup,
    ) -> Result<Self> {
        Self::from_group(class, ClosureCall::Return(returned), group)
    }

    fn from_group(
        class: &JvmClassPath,
        call: ClosureCall<'_>,
        group: &c::ParameterGroup,
    ) -> Result<Self> {
        match group {
            c::ParameterGroup::Value(index) => Ok(Self {
                kind: ClosureArgumentKind::Scalar(scalar::from_index(call, *index)?),
            }),
            c::ParameterGroup::ByteSlice(bytes) => Ok(Self {
                kind: ClosureArgumentKind::Bytes(bytes::from_group(call, bytes)?),
            }),
            c::ParameterGroup::DirectVector(vector) => Ok(Self {
                kind: ClosureArgumentKind::DirectVector(direct_vector::from_group(call, vector)?),
            }),
            c::ParameterGroup::DirectWriteback(_) => Err(Error::BrokenBridgeContract {
                bridge: JNI_BRIDGE,
                invariant: "closure call argument cannot be a direct-record writeback",
            }),
            c::ParameterGroup::CallbackCompletion(_) => Err(Error::BrokenBridgeContract {
                bridge: JNI_BRIDGE,
                invariant: "closure call argument cannot be an async callback completion",
            }),
            c::ParameterGroup::Continuation(_) => Err(Error::BrokenBridgeContract {
                bridge: JNI_BRIDGE,
                invariant: "closure call argument cannot be a poll continuation",
            }),
            c::ParameterGroup::Closure(nested) => Ok(Self {
                kind: ClosureArgumentKind::Closure(nested_closure::from_group(
                    class, call, nested,
                )?),
            }),
            c::ParameterGroup::ClosureReturn(_) => Err(Error::BrokenBridgeContract {
                bridge: JNI_BRIDGE,
                invariant: "closure call argument cannot be a closure return out-pointer",
            }),
        }
    }
}

impl<'source> ClosureCall<'source> {
    pub fn parameter(&self, index: c::ParameterIndex) -> &'source c::Parameter {
        match self {
            Self::Parameter(closure) => closure.parameter(index),
            Self::Return(returned) => returned.parameter(index),
        }
    }
}
