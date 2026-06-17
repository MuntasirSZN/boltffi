use std::collections::HashSet;
use std::env;
use std::fmt;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};

use boltffi_ast::PackageInfo;
use boltffi_binding::{
    BINDING_METADATA_BUILD_ENV, BINDING_METADATA_ROOT_ENV, BINDING_METADATA_SOURCE_ENV, LowerError,
    Native, Wasm32, lower_with_declarations,
};
use boltffi_scan::{ScanError, ScanInput};
use proc_macro2::{Span, TokenStream};
use quote::quote_spanned;

use crate::experimental::{
    error::Error as ExpansionError, expander::Expander, expansion::Expansion,
};

static EMITTED_ROOTS: OnceLock<Mutex<HashSet<PathBuf>>> = OnceLock::new();

pub enum Rendered {
    Inactive,
    OriginalOnly,
    Tokens(TokenStream),
}

pub fn render() -> Rendered {
    if env::var_os(BINDING_METADATA_BUILD_ENV).is_none() {
        return Rendered::Inactive;
    }

    match Request::from_env() {
        Ok(Some(request)) if request.mark_emitted() => request
            .render()
            .map(Rendered::Tokens)
            .unwrap_or_else(|error| Rendered::Tokens(error.into_compile_error())),
        Ok(Some(_)) => Rendered::OriginalOnly,
        Ok(None) => Rendered::Inactive,
        Err(error) => Rendered::Tokens(error.into_compile_error()),
    }
}

struct Request {
    root: PathBuf,
    source: PathBuf,
    package: PackageInfo,
}

impl Request {
    fn from_env() -> Result<Option<Self>, BuildError> {
        if !compiling_generated_crate()? {
            return Ok(None);
        }

        Ok(Some(Self {
            root: generated_root()?,
            source: PathBuf::from(required_env(BINDING_METADATA_SOURCE_ENV)?),
            package: PackageInfo::new(
                required_env("CARGO_PKG_NAME")?,
                env::var("CARGO_PKG_VERSION")
                    .ok()
                    .filter(|version| !version.is_empty()),
            ),
        }))
    }

    fn mark_emitted(&self) -> bool {
        EMITTED_ROOTS
            .get_or_init(|| Mutex::new(HashSet::new()))
            .lock()
            .map(|mut roots| roots.insert(canonical(&self.root)))
            .unwrap_or(false)
    }

    fn render(self) -> Result<TokenStream, BuildError> {
        let scan = boltffi_scan::scan_package(
            &ScanInput::new(&self.source, self.package).with_manifest_dir(&self.root),
        )?;
        let native = lower_with_declarations::<Native>(scan.complete())?;
        let wasm32 = lower_with_declarations::<Wasm32>(scan.complete())?;
        let native = Expansion::new(&native);
        let wasm32 = Expansion::new(&wasm32);
        Expander::new(scan.root())
            .all(&native, &wasm32)
            .map_err(Into::into)
    }
}

enum BuildError {
    MissingEnv(&'static str),
    Scan(ScanError),
    Lower(LowerError),
    Expansion(ExpansionError),
}

impl BuildError {
    fn into_compile_error(self) -> TokenStream {
        let message = self.to_string();
        quote_spanned! { Span::call_site() =>
            compile_error!(#message);
        }
    }
}

impl fmt::Display for BuildError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingEnv(key) => {
                write!(formatter, "BoltFFI metadata build: `{key}` is not set")
            }
            Self::Scan(error) => write!(formatter, "BoltFFI metadata build scan failed: {error}"),
            Self::Lower(error) => write!(formatter, "BoltFFI metadata build lower failed: {error}"),
            Self::Expansion(error) => {
                write!(
                    formatter,
                    "BoltFFI metadata build expansion failed: {error}"
                )
            }
        }
    }
}

impl From<ScanError> for BuildError {
    fn from(error: ScanError) -> Self {
        Self::Scan(error)
    }
}

impl From<LowerError> for BuildError {
    fn from(error: LowerError) -> Self {
        Self::Lower(error)
    }
}

impl From<ExpansionError> for BuildError {
    fn from(error: ExpansionError) -> Self {
        Self::Expansion(error)
    }
}

fn required_env(key: &'static str) -> Result<String, BuildError> {
    env::var(key).map_err(|_| BuildError::MissingEnv(key))
}

fn compiling_generated_crate() -> Result<bool, BuildError> {
    let manifest_dir = PathBuf::from(required_env("CARGO_MANIFEST_DIR")?);
    Ok(canonical(&manifest_dir) == canonical(&generated_root()?))
}

fn generated_root() -> Result<PathBuf, BuildError> {
    Ok(PathBuf::from(required_env(BINDING_METADATA_ROOT_ENV)?))
}

fn canonical(path: &Path) -> PathBuf {
    path.canonicalize().unwrap_or_else(|_| path.to_path_buf())
}
