use askama::Template as AskamaTemplate;

use crate::{
    core::Result,
    target::{
        jvm::{
            DesktopLoader, NativeLibraries,
            resource::{PLATFORMS, Platform},
        },
        kotlin::syntax::Literal,
    },
};

#[derive(AskamaTemplate)]
#[template(path = "target/kotlin/native_library_loader.kt", escape = "none")]
struct NativeLibraryLoaderTemplate {
    native_libraries: LibraryLiterals,
    resource_platforms: &'static [Platform],
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct LibraryLiterals {
    android: Literal,
    desktop_jni: Literal,
    desktop_fallback: Literal,
    desktop_loader: DesktopLoader,
}

/// Kotlin members that load the Android or desktop native library.
pub(crate) struct NativeLibraryLoader<'libraries> {
    libraries: &'libraries NativeLibraries,
}

impl<'libraries> NativeLibraryLoader<'libraries> {
    pub(crate) fn new(libraries: &'libraries NativeLibraries) -> Self {
        Self { libraries }
    }

    pub(crate) fn render(&self) -> Result<String> {
        Ok(NativeLibraryLoaderTemplate {
            native_libraries: LibraryLiterals::new(self.libraries),
            resource_platforms: PLATFORMS,
        }
        .render()?)
    }
}

impl LibraryLiterals {
    fn new(libraries: &NativeLibraries) -> Self {
        Self {
            android: Literal::string(libraries.android().as_str()),
            desktop_jni: Literal::string(libraries.desktop_jni().as_str()),
            desktop_fallback: Literal::string(libraries.desktop_fallback().as_str()),
            desktop_loader: libraries.desktop_loader(),
        }
    }

    fn android(&self) -> &Literal {
        &self.android
    }

    fn desktop_jni(&self) -> &Literal {
        &self.desktop_jni
    }

    fn desktop_fallback(&self) -> &Literal {
        &self.desktop_fallback
    }

    fn bundled_desktop_loader(&self) -> bool {
        self.desktop_loader.loads_bundled()
    }

    fn system_desktop_loader(&self) -> bool {
        self.desktop_loader.loads_system()
    }
}
