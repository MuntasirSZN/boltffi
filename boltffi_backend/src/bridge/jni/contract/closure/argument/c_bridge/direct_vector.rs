use crate::{
    bridge::{c, jni::ClosureDirectVectorArgument},
    core::Result,
};

use super::ClosureCall;

pub fn from_group(
    call: ClosureCall<'_>,
    vector: &c::DirectVectorParameter,
) -> Result<ClosureDirectVectorArgument> {
    ClosureDirectVectorArgument::from_vector(
        call.parameter(vector.pointer()),
        call.parameter(vector.length()),
        vector,
    )
}
