//! Registered JNI closure signatures.
//!
//! Multiple functions and callback methods can use the same inline closure
//! signature. The registration contract deduplicates those signatures, records
//! the JVM bridge class for each one, and keeps the generated call/release
//! symbols together.

mod contract;

pub use contract::ClosureRegistration;
