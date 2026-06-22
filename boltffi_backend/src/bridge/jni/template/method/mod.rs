//! Template views for native methods exported to the JVM.
//!
//! Native method contracts are typed for correctness. The template needs a
//! flatter view: declarations, borrowed array locals, direct-record writeback
//! fields, status checks, C bridge arguments, and the final return expression.
//!
//! This module performs that projection for method rendering only. It does not
//! choose parameter or return semantics; those decisions already live in the
//! contract layer.

mod array;
mod parameter;
mod record;
mod view;

pub use array::BorrowedArrayParameterView;
pub use parameter::NativeParameterView;
pub use record::RecordParameterView;
pub use view::NativeMethodView;
