//! Parameters accepted by generated `Java_*` native methods.
//!
//! A JNI entry point receives Java values. The lower C bridge expects grouped C
//! ABI arguments. One Java parameter can therefore expand into several C
//! arguments: a byte array becomes pointer and length, a direct vector becomes
//! pointer and element count, and a closure handle becomes call, context, and
//! release values.
//!
//! This module owns that native-method grouping. It models the Java-facing
//! parameter together with the C arguments produced from it, so method rendering
//! can forward values without reclassifying the C bridge parameter groups.

mod build;
mod native;

pub use native::{NativeParameter, NativeParameterKind};
