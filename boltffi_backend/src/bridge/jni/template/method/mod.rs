//! Template views for native methods exported to the JVM.
//!
//! Native methods are the JNI entry points that Java and Kotlin call. These views
//! prepare parameter declarations, borrowed array setup, direct-record writeback,
//! result conversion, and the argument list passed to the C bridge function.

mod array;
mod parameter;
mod record;
mod view;

pub use array::BorrowedArrayParameterView;
pub use parameter::NativeParameterView;
pub use record::RecordParameterView;
pub use view::NativeMethodView;
