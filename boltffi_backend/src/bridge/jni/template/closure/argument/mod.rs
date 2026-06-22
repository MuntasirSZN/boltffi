//! Template views for closure call arguments.
//!
//! Closure arguments can be scalar values, borrowed bytes, direct vectors, or
//! nested closure handles. The templates need different setup and cleanup code
//! for each shape, while the Rust contract keeps those shapes typed.

mod bytes;
mod c_parameter;
mod direct_vector;
mod handle;

pub use bytes::ClosureBytesArgumentView;
pub use c_parameter::ClosureCParameterView;
pub use direct_vector::ClosureDirectVectorArgumentView;
pub use handle::ClosureHandleArgumentView;
