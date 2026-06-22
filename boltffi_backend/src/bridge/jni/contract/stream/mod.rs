//! JNI contract for stream protocols.
//!
//! A stream is not one JNI method. The C bridge exposes a small protocol:
//! subscribe, poll, wait, fetch a batch, unsubscribe, and free. The JVM needs
//! those functions as a grouped native API with consistent names and handle
//! ownership.
//!
//! This module owns the JNI view of that protocol. When stream items have a
//! direct layout, it also adds the direct-batch helper that lets the JVM pull one
//! byte array instead of decoding item by item.

mod direct_batch;
mod protocol;

pub use direct_batch::DirectStreamBatchMethod;
pub use protocol::StreamProtocolMethods;
