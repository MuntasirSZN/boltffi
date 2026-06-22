//! Async callback completion contract.
//!
//! Async JVM callback methods return later through a C completion function
//! pointer. This module describes the completion callback argument, the payload
//! carried by successful completion, and the generated JNI invokers that Rust can
//! call when it needs to complete the callback.

mod argument;
mod invoker;
mod payload;

pub use argument::CallbackCompletionArgument;
pub use invoker::CallbackCompletionInvoker;
pub use payload::CallbackCompletionPayload;
