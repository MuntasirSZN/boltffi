//! JVM names and JNI symbol names.
//!
//! JNI has several spellings for the same logical class or method. Java source
//! uses dotted package names, class lookup uses slash-separated names, and native
//! exports use the `Java_*` escaping rules. Treating those as raw strings is how
//! bridges drift.
//!
//! This module validates JVM name pieces once and exposes the spellings the
//! contract builder and templates need. Code outside this module should ask for a
//! class path or symbol, not split package names or hand-roll JNI escaping.

mod class_path;
mod segment;
mod symbol;

pub use class_path::JvmClassPath;
pub use segment::JvmNameSegment;
pub use symbol::JniSymbolName;
