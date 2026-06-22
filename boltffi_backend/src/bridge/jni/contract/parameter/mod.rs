//! Native method parameters for JNI exports.
//!
//! JNI entry points receive Java values. The lower C bridge expects C ABI
//! arguments. One Java parameter can therefore become several C arguments, such
//! as a byte array becoming pointer and length, or a closure handle becoming
//! call, context, and release values.
//!
//! This module owns that grouping for native methods. It models the Java-facing
//! parameter and the C arguments produced from it so method rendering can forward
//! values without reclassifying the C bridge parameter groups.

mod build;
mod native;

pub use native::{NativeParameter, NativeParameterKind};
