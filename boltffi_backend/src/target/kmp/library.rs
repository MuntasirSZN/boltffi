//! Native-library configuration for Kotlin Multiplatform JVM-family targets.

use boltffi_binding::{Bindings, Native};

use crate::{
    core::Result,
    target::jvm::{DesktopLoader, LibraryName, NativeLibraries},
};

/// Explicit KMP native-library overrides awaiting binding-package defaults.
#[derive(Clone, Debug, Default, Eq, Hash, PartialEq)]
pub(crate) struct KmpNativeLibraryConfiguration {
    android_override: Option<LibraryName>,
    desktop_jni_override: Option<LibraryName>,
    desktop_fallback_override: Option<LibraryName>,
    desktop_loader: DesktopLoader,
}

impl KmpNativeLibraryConfiguration {
    pub(crate) fn android_library(mut self, library: LibraryName) -> Self {
        self.android_override = Some(library);
        self
    }

    pub(crate) fn desktop_jni_library(mut self, library: LibraryName) -> Self {
        self.desktop_jni_override = Some(library);
        self
    }

    pub(crate) fn desktop_fallback_library(mut self, library: LibraryName) -> Self {
        self.desktop_fallback_override = Some(library);
        self
    }

    pub(crate) fn desktop_loader(mut self, loader: DesktopLoader) -> Self {
        self.desktop_loader = loader;
        self
    }

    /// Resolves omitted names from the Cargo package represented by the bindings.
    pub(crate) fn resolve(&self, bindings: &Bindings<Native>) -> Result<NativeLibraries> {
        let android_default = LibraryName::parse(binding_package_name(bindings))?;
        let mut libraries = NativeLibraries::from_artifact(cargo_crate_name(bindings))?
            .with_android(android_default)
            .with_desktop_loader(self.desktop_loader);

        if let Some(library) = &self.android_override {
            libraries = libraries.with_android(library.clone());
        }
        if let Some(library) = &self.desktop_jni_override {
            libraries = libraries.with_desktop_jni(library.clone());
        }
        if let Some(library) = &self.desktop_fallback_override {
            libraries = libraries.with_desktop_fallback(library.clone());
        }

        Ok(libraries)
    }
}

/// Returns the Cargo crate artifact spelling for the binding package.
pub(crate) fn cargo_crate_name(bindings: &Bindings<Native>) -> String {
    binding_package_name(bindings).replace('-', "_")
}

fn binding_package_name(bindings: &Bindings<Native>) -> String {
    bindings
        .package()
        .name()
        .parts()
        .iter()
        .map(|part| part.as_str())
        .collect::<Vec<_>>()
        .join("_")
}
