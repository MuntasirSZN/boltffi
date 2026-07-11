mod emit;
mod lower;
mod mappings;
mod names;
mod plan;
mod templates;

pub use emit::{JavaEmitter, JavaFile, JavaOutput};
pub use lower::JavaLowerer;
pub use names::NamingConvention;
pub use plan::*;

pub use boltffi_backend::target::java::JavaVersion;

use boltffi_ffi_rules::naming::{LibraryName, Name};

#[derive(Debug, Clone)]
pub struct JavaOptions {
    pub library_name: Option<Name<LibraryName>>,
    pub min_java_version: JavaVersion,
    pub desktop_loader: bool,
}

impl Default for JavaOptions {
    fn default() -> Self {
        Self {
            library_name: None,
            min_java_version: JavaVersion::default(),
            desktop_loader: true,
        }
    }
}
