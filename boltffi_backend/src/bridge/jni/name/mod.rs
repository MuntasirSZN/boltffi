//! Names that must obey JVM and JNI spelling rules.
//!
//! The same logical class appears in several spellings. Java source uses dotted
//! package names, JNI lookup uses slash-separated class names, generated files
//! need stable paths, and native exports use the `Java_*` escaping rules.
//! Treating those as unrelated raw strings is how bridge code drifts.
//!
//! This module validates JVM name pieces once and exposes the spellings needed by
//! the contract builder and templates. Code outside this module asks for a class
//! path or symbol. It does not split package names or hand-roll JNI escaping.

mod class_path;
mod segment;
mod symbol;

pub use class_path::JvmClassPath;
pub use segment::JvmNameSegment;
pub use symbol::JniSymbolName;
