use crate::{bridge::c, core::Result};

use super::super::ClosureScalarArgument;
use super::ClosureCall;

pub fn from_index(
    call: ClosureCall<'_>,
    index: c::ParameterIndex,
) -> Result<ClosureScalarArgument> {
    ClosureScalarArgument::from_parameter(call.parameter(index))
}
