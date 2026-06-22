use crate::{
    bridge::c,
    core::{Error, Result},
};

use super::{
    ClosureArgument, ClosureArgumentKind, ClosureBytesArgument, ClosureDirectVectorArgument,
    ClosureScalarArgument,
};

const JNI_BRIDGE: &str = "jni";

enum ClosureCall<'source> {
    Parameter(&'source c::ClosureParameter),
    Return(&'source c::ClosureReturnParameter),
}

impl ClosureArgument {
    /// Builds a closure-call argument from a closure parameter group.
    pub fn from_closure_group(
        closure: &c::ClosureParameter,
        group: &c::ParameterGroup,
    ) -> Result<Self> {
        Self::from_group(ClosureCall::Parameter(closure), group)
    }

    /// Builds a closure-call argument from a closure return storage group.
    pub fn from_return_group(
        returned: &c::ClosureReturnParameter,
        group: &c::ParameterGroup,
    ) -> Result<Self> {
        Self::from_group(ClosureCall::Return(returned), group)
    }

    fn from_group(call: ClosureCall<'_>, group: &c::ParameterGroup) -> Result<Self> {
        match group {
            c::ParameterGroup::Value(index) => Ok(Self {
                kind: ClosureArgumentKind::Scalar(ClosureScalarArgument::from_parameter(
                    call.parameter(*index),
                )?),
            }),
            c::ParameterGroup::ByteSlice(bytes) => Ok(Self {
                kind: ClosureArgumentKind::Bytes(ClosureBytesArgument::from_bytes(
                    call.parameter(bytes.pointer()),
                    call.parameter(bytes.length()),
                    bytes,
                )?),
            }),
            c::ParameterGroup::DirectVector(vector) => Ok(Self {
                kind: ClosureArgumentKind::DirectVector(ClosureDirectVectorArgument::from_vector(
                    call.parameter(vector.pointer()),
                    call.parameter(vector.length()),
                    vector,
                )?),
            }),
            c::ParameterGroup::CallbackCompletion(_) => Err(Error::BrokenBridgeContract {
                bridge: JNI_BRIDGE,
                invariant: "closure call argument cannot be an async callback completion",
            }),
            c::ParameterGroup::Continuation(_) => Err(Error::BrokenBridgeContract {
                bridge: JNI_BRIDGE,
                invariant: "closure call argument cannot be a poll continuation",
            }),
            c::ParameterGroup::Closure(_) => Err(Error::UnsupportedBridge {
                bridge: JNI_BRIDGE,
                shape: "nested closure argument",
            }),
            c::ParameterGroup::ClosureReturn(_) => Err(Error::BrokenBridgeContract {
                bridge: JNI_BRIDGE,
                invariant: "closure call argument cannot be a closure return out-pointer",
            }),
        }
    }
}

impl ClosureCall<'_> {
    fn parameter(&self, index: c::ParameterIndex) -> &c::Parameter {
        match self {
            Self::Parameter(closure) => closure.parameter(index),
            Self::Return(returned) => returned.parameter(index),
        }
    }
}
