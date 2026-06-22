//! Root template input for the generated JNI C source file.
//!
//! The JNI bridge emits one C file. The file is assembled from a root Askama
//! template plus focused fragments for lifecycle hooks, native methods,
//! callbacks, closures, streams, continuations, byte arrays, direct records, and
//! handle storage.
//!
//! This module builds the root template input from the finished JNI contract. It
//! decides which source fragments are needed, but it does not decide ABI
//! behavior. By this point, every method, callback, closure, stream, and return
//! shape has already been selected by the contract layer.

mod features;
mod file;

pub use file::SourceFile;
