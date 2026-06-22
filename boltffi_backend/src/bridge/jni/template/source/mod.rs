//! Top-level JNI C source file rendering.
//!
//! The JNI bridge emits one C file. That file is assembled from a root template
//! plus feature fragments for lifecycle hooks, native methods, callbacks,
//! closures, streams, continuations, byte arrays, direct records, and handle
//! storage.
//!
//! This module builds the root template input and decides which fragments are
//! needed from the typed contract. It does not decide ABI behavior; it only turns
//! an already-built contract into source-file data.

mod features;
mod file;

pub use file::SourceFile;
