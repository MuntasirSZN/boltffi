use crate::bridge::c::ArgumentList;

use super::ClosureArgument;

impl ClosureArgument {
    /// Returns the argument list passed to the static JVM closure method.
    pub fn jvm_argument_list(arguments: &[Self]) -> ArgumentList {
        ArgumentList::from_iter(arguments.iter().flat_map(ClosureArgument::jvm_arguments))
    }

    /// Returns the argument list passed to the Rust closure call function.
    pub fn rust_argument_list(arguments: &[Self]) -> ArgumentList {
        ArgumentList::from_iter(arguments.iter().flat_map(ClosureArgument::rust_arguments))
    }
}
