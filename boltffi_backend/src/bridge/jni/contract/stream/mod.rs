//! JNI contract for stream protocol functions.
//!
//! A stream is not one JNI method. The C bridge exposes a protocol: subscribe,
//! poll, wait, fetch a batch, unsubscribe, and free. The JVM needs that protocol
//! as a group of native methods with consistent names and handle ownership.
//!
//! This module owns the JNI view of that stream protocol. When stream items have
//! a direct layout, it also adds the direct-batch helper that lets the JVM pull a
//! byte array instead of decoding item by item.

mod direct_batch;
mod protocol;

pub use direct_batch::DirectStreamBatchMethod;
pub use protocol::StreamProtocolMethods;
