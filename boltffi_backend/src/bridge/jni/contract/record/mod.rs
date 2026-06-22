//! Direct-record JNI contract.
//!
//! Direct records travel through JNI as fixed-size byte arrays and through the C
//! bridge as record values. This module keeps the record value, parameter, and
//! mutation writeback contracts separate so direct-record handling does not leak
//! into scalar or byte-buffer paths.

mod parameter;
mod value;
mod writeback;

pub use parameter::RecordParameter;
pub use value::RecordValue;
pub use writeback::RecordWriteback;
