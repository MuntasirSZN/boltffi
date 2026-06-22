use crate::{
    bridge::{c, jni::ClosureBytesArgument},
    core::Result,
};

use super::ClosureCall;

pub fn from_group(
    call: ClosureCall<'_>,
    bytes: &c::ByteSliceParameter,
) -> Result<ClosureBytesArgument> {
    ClosureBytesArgument::from_bytes(
        call.parameter(bytes.pointer()),
        call.parameter(bytes.length()),
        bytes,
    )
}
