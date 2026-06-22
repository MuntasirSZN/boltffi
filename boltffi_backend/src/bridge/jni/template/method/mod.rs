//! Template views for `Java_*` native methods.
//!
//! Native method contracts are typed for correctness. The Askama template needs
//! a source-ready view: parameter declarations, borrowed array locals,
//! direct-record writeback fields, status checks, C bridge arguments, and the
//! final return expression.
//!
//! This module performs that projection for method rendering only. Parameter and
//! return semantics already live in the contract layer.

mod array;
mod parameter;
mod record;
mod view;

pub use array::BorrowedArrayParameterView;
pub use parameter::NativeParameterView;
pub use record::RecordParameterView;
pub use view::NativeMethodView;
