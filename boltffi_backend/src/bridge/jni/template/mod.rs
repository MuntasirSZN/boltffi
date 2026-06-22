//! Askama-backed rendering for the generated JNI C source.
//!
//! The contract layer decides what exists and what each piece means. This layer
//! prepares the small template views needed to print that contract as C source.
//! Rust code keeps the data typed, while generated C syntax stays in Askama
//! templates.
//!
//! The split matters because JNI glue is mostly syntax once the contract is
//! built. Adding a callback shape, stream helper, or closure return should mean
//! extending the typed contract and a focused template view, not rebuilding ABI
//! decisions in the renderer or burying generated C inside Rust string
//! concatenation.

mod callback;
mod closure;
mod method;
mod source;
mod stream;

pub use self::source::SourceFile;
