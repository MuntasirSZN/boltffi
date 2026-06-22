use crate::{
    bridge::{
        c::{self, Identifier},
        jni::{CallbackCParameter, JniType},
    },
    core::Result,
};

use super::super::{CallbackArgument, CallbackArgumentKind};

pub fn from_parameter(parameter: &c::Parameter) -> Result<CallbackArgument> {
    if matches!(parameter.ty(), c::Type::CallbackHandle(_)) {
        return Ok(CallbackArgument {
            kind: CallbackArgumentKind::CallbackHandle {
                handle: Identifier::parse(format!("__boltffi_{}_handle", parameter.name()))?,
                parameter: CallbackCParameter::from_parameter(parameter)?,
            },
        });
    }
    if matches!(parameter.ty(), c::Type::DirectRecord(_)) {
        return Ok(CallbackArgument {
            kind: CallbackArgumentKind::Record {
                array: Identifier::parse(format!("__boltffi_{}_array", parameter.name()))?,
                parameter: CallbackCParameter::from_parameter(parameter)?,
            },
        });
    }
    Ok(CallbackArgument {
        kind: CallbackArgumentKind::Value {
            parameter: CallbackCParameter::from_parameter(parameter)?,
            jni_type: JniType::from_c_type(parameter.ty())?,
        },
    })
}
