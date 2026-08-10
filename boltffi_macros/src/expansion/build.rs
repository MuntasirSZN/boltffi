use std::env;
use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};

use boltffi_ast::PackageInfo;
use boltffi_binding::{
    BINDING_EXPANSION_BUILD_ENV, BINDING_EXPANSION_ROOT_ENV, BINDING_EXPANSION_SOURCE_ENV,
    BINDING_EXPANSION_SURFACE_ENV, BINDING_METADATA_BUILD_ENV, BINDING_METADATA_FEATURES_ENV,
    BINDING_METADATA_ROOT_ENV, BINDING_METADATA_SOURCE_ENV, BINDING_METADATA_SURFACE_ENV,
    BindingMetadataSurface, LowerError, Native, SerializedBindings, Wasm32,
    lower_with_declarations,
};
use boltffi_scan::{ActiveCfg, ScanError, ScanInput};
use proc_macro2::{Span, TokenStream};
use quote::quote_spanned;
use serde::Deserialize;

use crate::expansion::{
    error::Error as ExpansionError, expander::Expander, expansion::Expansion, metadata,
    rust_api::RootModuleTypes,
};

pub enum Item {
    Preserve,
    Tokens(TokenStream),
    Error(TokenStream),
}

#[derive(Clone, Copy)]
enum Emission {
    Root,
    TypeSupport,
    Metadata,
}

struct Request {
    manifest_dir: PathBuf,
    source: PathBuf,
    package: PackageInfo,
    surface: BindingMetadataSurface,
    emission: Emission,
}

#[derive(Deserialize)]
struct CargoManifest {
    lib: Option<LibraryTarget>,
}

#[derive(Deserialize)]
struct LibraryTarget {
    path: Option<PathBuf>,
}

static EMITTED: AtomicBool = AtomicBool::new(false);

pub fn item() -> Item {
    if EMITTED.swap(true, Ordering::AcqRel) {
        return Item::Preserve;
    }
    Request::from_environment()
        .and_then(Request::render)
        .map(Item::Tokens)
        .unwrap_or_else(|error| Item::Error(error.into_compile_error()))
}

impl Request {
    fn from_environment() -> Result<Self, BuildError> {
        if env::var_os(BINDING_EXPANSION_BUILD_ENV).is_some() {
            return Self::expansion_build();
        }
        if env::var_os(BINDING_METADATA_BUILD_ENV).is_some() {
            return Self::metadata_build();
        }
        Self::cargo_build()
    }

    fn expansion_build() -> Result<Self, BuildError> {
        let requested_root = PathBuf::from(required_env(BINDING_EXPANSION_ROOT_ENV)?);
        let manifest_dir = current_manifest_dir()?;
        let surface = parsed_surface(BINDING_EXPANSION_SURFACE_ENV)?;
        if canonical(&manifest_dir) == canonical(&requested_root) {
            return Self::new(
                manifest_dir,
                PathBuf::from(required_env(BINDING_EXPANSION_SOURCE_ENV)?),
                surface,
                Emission::Root,
            );
        }
        Self::local(manifest_dir, surface, Emission::TypeSupport)
    }

    fn metadata_build() -> Result<Self, BuildError> {
        let requested_root = PathBuf::from(required_env(BINDING_METADATA_ROOT_ENV)?);
        let manifest_dir = current_manifest_dir()?;
        let surface = parsed_surface(BINDING_METADATA_SURFACE_ENV)?;
        if canonical(&manifest_dir) == canonical(&requested_root) {
            return Self::new(
                manifest_dir,
                PathBuf::from(required_env(BINDING_METADATA_SOURCE_ENV)?),
                surface,
                Emission::Metadata,
            );
        }
        Self::local(manifest_dir, surface, Emission::TypeSupport)
    }

    fn cargo_build() -> Result<Self, BuildError> {
        let manifest_dir = current_manifest_dir()?;
        let surface = match env::var("CARGO_CFG_TARGET_ARCH").as_deref() {
            Ok("wasm32") => BindingMetadataSurface::Wasm32,
            _ => BindingMetadataSurface::Native,
        };
        let emission = match env::var_os("CARGO_PRIMARY_PACKAGE") {
            Some(_) => Emission::Root,
            None => Emission::TypeSupport,
        };
        Self::local(manifest_dir, surface, emission)
    }

    fn local(
        manifest_dir: PathBuf,
        surface: BindingMetadataSurface,
        emission: Emission,
    ) -> Result<Self, BuildError> {
        let manifest_path = manifest_dir.join("Cargo.toml");
        let manifest_source =
            fs::read_to_string(&manifest_path).map_err(|source| BuildError::ReadManifest {
                path: manifest_path.clone(),
                source,
            })?;
        let manifest = toml::from_str::<CargoManifest>(&manifest_source).map_err(|source| {
            BuildError::ParseManifest {
                path: manifest_path,
                source,
            }
        })?;
        let source = manifest.lib.and_then(|library| library.path).map_or_else(
            || manifest_dir.join("src/lib.rs"),
            |path| manifest_dir.join(path),
        );
        Self::new(manifest_dir, source, surface, emission)
    }

    fn new(
        manifest_dir: PathBuf,
        source: PathBuf,
        surface: BindingMetadataSurface,
        emission: Emission,
    ) -> Result<Self, BuildError> {
        Ok(Self {
            manifest_dir,
            source,
            package: PackageInfo::new(
                required_env("CARGO_PKG_NAME")?,
                env::var("CARGO_PKG_VERSION")
                    .ok()
                    .filter(|version| !version.is_empty()),
            ),
            surface,
            emission,
        })
    }

    fn render(self) -> Result<TokenStream, BuildError> {
        let scan = boltffi_scan::scan_package(
            &ScanInput::new(&self.source, self.package.clone())
                .with_manifest_dir(&self.manifest_dir)
                .with_cfg(self.active_cfg()),
        )?;
        match self.emission {
            Emission::Root => self.render_root(&scan),
            Emission::TypeSupport => self.render_type_support(&scan),
            Emission::Metadata => self.render_metadata(&scan),
        }
    }

    fn render_root(&self, scan: &boltffi_scan::PackageScan) -> Result<TokenStream, BuildError> {
        let source = scan.root_with_support();
        let visible_paths = scan
            .root_visible_paths()
            .map(|(id, path)| (id.to_owned(), path.clone()))
            .collect::<Vec<_>>();
        let root_types =
            RootModuleTypes::with_visible_paths(&scan.complete().package, visible_paths.clone());
        let source = root_types.contract(&source);
        let root = root_types.contract(scan.root());
        let expander = Expander::with_support(&root, &source, visible_paths);
        match self.surface {
            BindingMetadataSurface::Native => {
                let lowered = lower_with_declarations::<Native>(&source)?;
                expander
                    .native(&Expansion::new(&lowered))
                    .map_err(Into::into)
            }
            BindingMetadataSurface::Wasm32 => {
                let lowered = lower_with_declarations::<Wasm32>(&source)?;
                expander
                    .wasm32(&Expansion::new(&lowered))
                    .map_err(Into::into)
            }
        }
    }

    fn render_type_support(
        &self,
        scan: &boltffi_scan::PackageScan,
    ) -> Result<TokenStream, BuildError> {
        let visible_paths = scan
            .root_visible_paths()
            .map(|(id, path)| (id.to_owned(), path.clone()))
            .collect::<Vec<_>>();
        let root_types =
            RootModuleTypes::with_visible_paths(&scan.complete().package, visible_paths.clone());
        let source = root_types.contract(&scan.root_with_support());
        let root = root_types.contract(scan.root());
        let expander = Expander::with_support(&root, &root, visible_paths);
        match self.surface {
            BindingMetadataSurface::Native => {
                let lowered = lower_with_declarations::<Native>(&source)?;
                expander
                    .native_type_support(&Expansion::new(&lowered))
                    .map_err(Into::into)
            }
            BindingMetadataSurface::Wasm32 => {
                let lowered = lower_with_declarations::<Wasm32>(&source)?;
                expander
                    .wasm32_type_support(&Expansion::new(&lowered))
                    .map_err(Into::into)
            }
        }
    }

    fn render_metadata(&self, scan: &boltffi_scan::PackageScan) -> Result<TokenStream, BuildError> {
        let source = scan.root_with_support();
        match self.surface {
            BindingMetadataSurface::Native => {
                let lowered = lower_with_declarations::<Native>(&source)?;
                metadata::render(SerializedBindings::native(lowered.into_bindings()))
                    .map_err(Into::into)
            }
            BindingMetadataSurface::Wasm32 => {
                let lowered = lower_with_declarations::<Wasm32>(&source)?;
                metadata::render(SerializedBindings::wasm32(lowered.into_bindings()))
                    .map_err(Into::into)
            }
        }
    }

    fn active_cfg(&self) -> ActiveCfg {
        let features = matches!(self.emission, Emission::Root | Emission::Metadata)
            .then(|| env::var(BINDING_METADATA_FEATURES_ENV).ok())
            .flatten()
            .into_iter()
            .flat_map(|features| {
                features
                    .split(',')
                    .filter(|feature| !feature.is_empty())
                    .map(str::to_owned)
                    .collect::<Vec<_>>()
            });
        ActiveCfg::from_cargo_env().with_features(features)
    }
}

enum BuildError {
    MissingEnv(&'static str),
    InvalidSurface {
        key: &'static str,
        value: String,
    },
    ReadManifest {
        path: PathBuf,
        source: std::io::Error,
    },
    ParseManifest {
        path: PathBuf,
        source: toml::de::Error,
    },
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
            Self::MissingEnv(key) => write!(formatter, "BoltFFI macro build: `{key}` is not set"),
            Self::InvalidSurface { key, value } => {
                write!(
                    formatter,
                    "BoltFFI macro build: `{key}` has invalid value `{value}"
                )
            }
            Self::ReadManifest { path, source } => {
                write!(
                    formatter,
                    "read Cargo manifest `{}`: {source}",
                    path.display()
                )
            }
            Self::ParseManifest { path, source } => {
                write!(
                    formatter,
                    "parse Cargo manifest `{}`: {source}",
                    path.display()
                )
            }
            Self::Scan(error) => write!(formatter, "BoltFFI macro scan failed: {error}"),
            Self::Lower(error) => write!(formatter, "BoltFFI macro lowering failed: {error}"),
            Self::Expansion(error) => write!(formatter, "BoltFFI macro expansion failed: {error}"),
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

fn current_manifest_dir() -> Result<PathBuf, BuildError> {
    required_env("CARGO_MANIFEST_DIR").map(PathBuf::from)
}

fn parsed_surface(key: &'static str) -> Result<BindingMetadataSurface, BuildError> {
    let value = required_env(key)?;
    BindingMetadataSurface::parse(&value).ok_or(BuildError::InvalidSurface { key, value })
}

fn canonical(path: &Path) -> PathBuf {
    path.canonicalize().unwrap_or_else(|_| path.to_path_buf())
}
