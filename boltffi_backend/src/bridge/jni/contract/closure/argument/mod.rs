//! Arguments passed when Rust invokes a JVM-owned closure.
//!
//! Closure arguments are built from C closure parameter groups. They describe the
//! C function-pointer parameters, the JVM values passed to the closure method,
//! and any setup needed for borrowed bytes, direct vectors, or nested closures.

mod bytes;
mod c_abi;
mod c_bridge;
mod direct_vector;
mod handle;
mod jvm;
mod parameters;
mod scalar;
mod setup;

pub use bytes::ClosureBytesArgument;
pub use c_abi::ClosureCParameter;
pub use direct_vector::ClosureDirectVectorArgument;
pub use handle::ClosureHandleArgument;
pub use scalar::ClosureScalarArgument;

/// One inline-closure argument crossing the JNI bridge.
#[derive(Clone, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub struct ClosureArgument {
    kind: ClosureArgumentKind,
}

#[derive(Clone, Debug, Eq, PartialEq)]
enum ClosureArgumentKind {
    Scalar(ClosureScalarArgument),
    Bytes(ClosureBytesArgument),
    DirectVector(ClosureDirectVectorArgument),
    Closure(ClosureHandleArgument),
}
