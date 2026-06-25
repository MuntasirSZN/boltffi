use crate::{
    bridge::{
        c,
        jni::{CallbackCParameter, CallbackSuccessOutWriter, JniType},
    },
    core::Result,
};

use super::super::{CallbackArgument, CallbackArgumentKind};

pub fn from_parameter(parameter: &c::Parameter) -> Result<CallbackArgument> {
    Ok(CallbackArgument {
        kind: CallbackArgumentKind::SuccessOut {
            writer: CallbackSuccessOutWriter::method_for_parameter(parameter)?,
            parameter: CallbackCParameter::from_parameter(parameter)?,
            jni_type: JniType::from_c_type(parameter.ty())?,
        },
    })
}
