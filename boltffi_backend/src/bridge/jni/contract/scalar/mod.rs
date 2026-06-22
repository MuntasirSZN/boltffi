//! Scalar JNI contract.
//!
//! Scalar parameters and scalar returns share the same JNI type vocabulary, but
//! they have different C bridge responsibilities. Parameters may need casts when
//! passed into C, while returns may need casts when passed back to Java.

mod parameter;
mod return_value;

pub use parameter::ScalarParameter;
pub use return_value::ScalarReturn;
