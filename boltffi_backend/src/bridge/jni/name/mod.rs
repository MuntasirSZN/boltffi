//! JVM names and JNI symbol names.
//!
//! JNI names are not plain identifiers. Package segments, class names, callback
//! bridge classes, closure bridge classes, and exported `Java_*` symbols all have
//! different escaping rules.
//!
//! This module keeps that spelling logic in one place so contract building and
//! templates do not split paths or hand-roll JNI escaping.

mod class_path;
mod segment;
mod symbol;

pub use class_path::JvmClassPath;
pub use segment::JvmNameSegment;
pub use symbol::JniSymbolName;
