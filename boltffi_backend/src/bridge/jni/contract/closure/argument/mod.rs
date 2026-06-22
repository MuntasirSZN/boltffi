mod bytes;
mod c_abi;
mod c_bridge;
mod jvm;
mod scalar;

pub use bytes::ClosureBytesArgument;
pub use c_abi::ClosureCParameter;
pub use scalar::ClosureScalarArgument;

use crate::bridge::c::Expression;

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
}

impl ClosureArgument {
    /// Returns the C parameters accepted by the closure call trampoline.
    pub fn c_parameters(&self) -> Vec<ClosureCParameter> {
        match &self.kind {
            ClosureArgumentKind::Scalar(argument) => argument.c_parameters(),
            ClosureArgumentKind::Bytes(argument) => argument.c_parameters(),
        }
    }

    /// Returns the C parameters accepted by the Rust-owned closure handle entrypoint.
    pub fn handle_parameters(&self) -> Vec<ClosureCParameter> {
        match &self.kind {
            ClosureArgumentKind::Scalar(argument) => argument.handle_parameters(),
            ClosureArgumentKind::Bytes(argument) => argument.handle_parameters(),
        }
    }

    /// Returns the byte-array argument when the JVM receives encoded bytes.
    pub fn call_bytes(&self) -> Option<&ClosureBytesArgument> {
        match &self.kind {
            ClosureArgumentKind::Scalar(_) => None,
            ClosureArgumentKind::Bytes(argument) => Some(argument),
        }
    }

    /// Returns the byte-array argument when the JVM sends encoded bytes.
    pub fn handle_bytes(&self) -> Option<&ClosureBytesArgument> {
        self.call_bytes()
    }

    /// Returns the expressions passed to the static JVM closure method.
    pub fn jvm_arguments(&self) -> Vec<Expression> {
        match &self.kind {
            ClosureArgumentKind::Scalar(argument) => argument.jvm_arguments(),
            ClosureArgumentKind::Bytes(argument) => argument.jvm_arguments(),
        }
    }

    /// Returns the expressions passed into the Rust closure call function.
    pub fn rust_arguments(&self) -> Vec<Expression> {
        match &self.kind {
            ClosureArgumentKind::Scalar(argument) => argument.rust_arguments(),
            ClosureArgumentKind::Bytes(argument) => argument.rust_arguments(),
        }
    }

    /// Returns the JNI method descriptor segment for this argument.
    pub fn jni_signature(&self) -> &'static str {
        match &self.kind {
            ClosureArgumentKind::Scalar(argument) => argument.jni_signature(),
            ClosureArgumentKind::Bytes(argument) => argument.jni_signature(),
        }
    }
}
