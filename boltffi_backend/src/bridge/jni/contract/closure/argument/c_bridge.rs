use crate::{
    bridge::c,
    core::{Error, Result},
};

use super::{ClosureArgument, ClosureArgumentKind, ClosureBytesArgument, ClosureScalarArgument};

const JNI_BRIDGE: &str = "jni";

impl ClosureArgument {
    pub(in crate::bridge::jni::contract::closure) fn from_group(
        closure: &c::ClosureParameter,
        group: &c::ParameterGroup,
    ) -> Result<Self> {
        match group {
            c::ParameterGroup::Value(index) => Ok(Self {
                kind: ClosureArgumentKind::Scalar(ClosureScalarArgument::from_parameter(
                    closure.parameter(*index),
                )?),
            }),
            c::ParameterGroup::ByteSlice(bytes) => Ok(Self {
                kind: ClosureArgumentKind::Bytes(ClosureBytesArgument::from_bytes(closure, bytes)?),
            }),
            c::ParameterGroup::DirectVector(_) => Err(Error::UnsupportedBridge {
                bridge: JNI_BRIDGE,
                shape: "closure direct-vector argument",
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
        }
    }
}
