//! JNI contract for stream protocol functions.
//!
//! Streams are exposed through the C bridge as subscription, polling, batch,
//! wait, unsubscribe, and free functions. The JNI bridge turns those functions
//! into native methods and adds direct-batch helpers when stream items can be
//! copied as primitive or record arrays.

mod direct_batch;
mod protocol;

pub use direct_batch::DirectStreamBatchMethod;
pub use protocol::StreamProtocolMethods;
