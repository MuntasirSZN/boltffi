//! Builds a Rust crate and reads embedded BoltFFI binding metadata.

use std::ffi::{OsStr, OsString};
use std::path::{Path, PathBuf};
use std::process::{Command, ExitStatus};

use boltffi_binding::BindingMetadataEnvelope;
use serde::Deserialize;
use thiserror::Error;

use crate::artifact::{BindingMetadataReadError, BindingMetadataReader};

/// A Cargo library build that extracts embedded BoltFFI binding metadata.
///
/// The build enables the `boltffi_metadata` cfg and reads Cargo's JSON
/// artifact stream. Artifact decoding is delegated to
/// [`BindingMetadataReader`], so section framing and contract validation
/// stay on the same path used by direct artifact reads.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BindingMetadataBuild {
    manifest_path: PathBuf,
    target: Option<String>,
}

impl BindingMetadataBuild {
    /// Creates a metadata build for a Cargo manifest.
    pub fn new(manifest_path: impl Into<PathBuf>) -> Self {
        Self {
            manifest_path: manifest_path.into(),
            target: None,
        }
    }

    /// Builds for a Cargo target triple.
    pub fn target(mut self, target: impl Into<String>) -> Self {
        self.target = Some(target.into());
        self
    }

    /// Runs Cargo and returns the validated metadata envelopes.
    pub fn read(&self) -> Result<Vec<BindingMetadataEnvelope>, BindingMetadataBuildError> {
        let output = CargoBuild::new(self).output()?;
        let artifacts = output.artifacts(&self.manifest_path)?;
        BindingMetadataReader::new(artifacts.into_paths())
            .read_required()
            .map_err(BindingMetadataBuildError::Metadata)
    }
}

/// Failure while building a crate for embedded binding metadata.
#[derive(Debug, Error)]
pub enum BindingMetadataBuildError {
    /// Cargo could not be started.
    #[error("run cargo build for binding metadata: {source}")]
    CargoSpawn {
        /// Process spawn error.
        source: std::io::Error,
    },
    /// Cargo returned a non-zero exit status.
    #[error("cargo build for binding metadata failed with status {status}: {stderr}")]
    CargoFailed {
        /// Process exit status.
        status: CargoStatus,
        /// Cargo standard error.
        stderr: String,
    },
    /// Cargo emitted a malformed JSON message.
    #[error("parse cargo JSON message `{line}`: {source}")]
    CargoJson {
        /// Raw Cargo output line.
        line: String,
        /// JSON parse error.
        source: serde_json::Error,
    },
    /// Cargo did not report a readable compiled artifact.
    #[error("cargo build for `{manifest_path}` did not report compiled library artifacts")]
    NoArtifacts {
        /// Manifest path passed to Cargo.
        manifest_path: PathBuf,
    },
    /// Embedded metadata could not be read from the produced artifacts.
    #[error(transparent)]
    Metadata(#[from] BindingMetadataReadError),
}

/// Exit status reported by Cargo.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CargoStatus {
    code: Option<i32>,
}

impl CargoStatus {
    fn from_status(status: ExitStatus) -> Self {
        Self {
            code: status.code(),
        }
    }

    /// Returns Cargo's process exit code.
    pub const fn code(self) -> Option<i32> {
        self.code
    }
}

impl std::fmt::Display for CargoStatus {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.code
            .map(|code| write!(formatter, "{code}"))
            .unwrap_or_else(|| formatter.write_str("terminated by signal"))
    }
}

#[derive(Clone, Debug)]
struct CargoBuild<'build> {
    build: &'build BindingMetadataBuild,
}

impl<'build> CargoBuild<'build> {
    const fn new(build: &'build BindingMetadataBuild) -> Self {
        Self { build }
    }

    fn output(self) -> Result<CargoOutput, BindingMetadataBuildError> {
        let mut command = Command::new(CargoProgram::from_env().into_os_string());
        command
            .arg("build")
            .arg("--lib")
            .arg("--message-format=json-render-diagnostics")
            .arg("--manifest-path")
            .arg(&self.build.manifest_path);
        if let Some(target) = &self.build.target {
            command.arg("--target").arg(target);
        }
        MetadataRustflags::from_env().apply(&mut command);
        command
            .output()
            .map_err(|source| BindingMetadataBuildError::CargoSpawn { source })
            .and_then(CargoOutput::from_output)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct CargoProgram {
    program: OsString,
}

impl CargoProgram {
    fn from_env() -> Self {
        Self {
            program: std::env::var_os("CARGO").unwrap_or_else(|| OsString::from("cargo")),
        }
    }

    fn into_os_string(self) -> OsString {
        self.program
    }
}

#[derive(Clone, Debug)]
struct CargoOutput {
    stdout: String,
}

impl CargoOutput {
    fn from_output(output: std::process::Output) -> Result<Self, BindingMetadataBuildError> {
        if output.status.success() {
            Ok(Self {
                stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
            })
        } else {
            Err(BindingMetadataBuildError::CargoFailed {
                status: CargoStatus::from_status(output.status),
                stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
            })
        }
    }

    fn artifacts(
        &self,
        manifest_path: &Path,
    ) -> Result<MetadataArtifacts, BindingMetadataBuildError> {
        let artifacts = self
            .stdout
            .lines()
            .filter(|line| !line.trim().is_empty())
            .map(CargoMessage::parse)
            .collect::<Result<Vec<_>, _>>()?
            .into_iter()
            .flat_map(CargoMessage::filenames)
            .filter_map(MetadataArtifact::from_cargo_filename)
            .collect::<Vec<_>>();

        MetadataArtifacts::new(manifest_path, artifacts)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct MetadataArtifacts {
    artifacts: Vec<MetadataArtifact>,
}

impl MetadataArtifacts {
    fn new(
        manifest_path: &Path,
        artifacts: Vec<MetadataArtifact>,
    ) -> Result<Self, BindingMetadataBuildError> {
        if artifacts.is_empty() {
            Err(BindingMetadataBuildError::NoArtifacts {
                manifest_path: manifest_path.to_path_buf(),
            })
        } else {
            Ok(Self { artifacts })
        }
    }

    fn into_paths(self) -> Vec<PathBuf> {
        self.artifacts
            .into_iter()
            .map(MetadataArtifact::into_path)
            .collect()
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct MetadataArtifact {
    path: PathBuf,
}

impl MetadataArtifact {
    fn from_cargo_filename(path: PathBuf) -> Option<Self> {
        path.extension()
            .and_then(OsStr::to_str)
            .is_some_and(Self::metadata_extension)
            .then_some(Self { path })
    }

    fn metadata_extension(extension: &str) -> bool {
        matches!(
            extension,
            "a" | "dll" | "dylib" | "lib" | "o" | "obj" | "rlib" | "so" | "wasm"
        )
    }

    fn into_path(self) -> PathBuf {
        self.path
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
#[serde(tag = "reason", rename_all = "kebab-case")]
enum CargoMessage {
    CompilerArtifact {
        filenames: Vec<PathBuf>,
    },
    #[serde(other)]
    Other,
}

impl CargoMessage {
    fn parse(line: &str) -> Result<Self, BindingMetadataBuildError> {
        serde_json::from_str(line).map_err(|source| BindingMetadataBuildError::CargoJson {
            line: line.to_owned(),
            source,
        })
    }

    fn filenames(self) -> Vec<PathBuf> {
        match self {
            Self::CompilerArtifact { filenames } => filenames,
            Self::Other => Vec::new(),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
enum MetadataRustflags {
    Encoded(OsString),
    Plain(OsString),
}

impl MetadataRustflags {
    fn from_env() -> Self {
        std::env::var_os("CARGO_ENCODED_RUSTFLAGS")
            .map(Self::encoded)
            .unwrap_or_else(|| Self::plain(std::env::var_os("RUSTFLAGS")))
    }

    fn encoded(existing: OsString) -> Self {
        Self::Encoded(Self::append_encoded(existing))
    }

    fn plain(existing: Option<OsString>) -> Self {
        Self::Plain(match existing.filter(|value| !value.is_empty()) {
            Some(mut value) => {
                value.push(" --cfg boltffi_metadata");
                value
            }
            None => OsString::from("--cfg boltffi_metadata"),
        })
    }

    fn append_encoded(mut existing: OsString) -> OsString {
        if !existing.is_empty() {
            existing.push(OsStr::new("\u{1f}"));
        }
        existing.push(OsStr::new("--cfg"));
        existing.push(OsStr::new("\u{1f}"));
        existing.push(OsStr::new("boltffi_metadata"));
        existing
    }

    fn apply(self, command: &mut Command) {
        match self {
            Self::Encoded(value) => {
                command.env("CARGO_ENCODED_RUSTFLAGS", value);
            }
            Self::Plain(value) => {
                command.env("RUSTFLAGS", value);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    use boltffi_ast::{PackageInfo, SourceContract};
    use boltffi_binding::{
        BindingMetadataEnvelope, BindingMetadataSection, Native, SerializedBindings,
        lower_with_declarations,
    };

    use super::{BindingMetadataBuild, BindingMetadataBuildError};
    use crate::artifact::BindingMetadataReadError;

    #[test]
    fn cargo_build_reads_metadata_from_reported_artifacts() {
        if cfg!(miri) {
            return;
        }

        let expected = metadata_envelope();
        let fixture = FixtureCrate::with_metadata(&expected);

        let envelopes = BindingMetadataBuild::new(fixture.manifest())
            .read()
            .expect("cargo metadata build reads");

        assert_eq!(envelopes, vec![expected]);
    }

    #[test]
    fn cargo_build_rejects_crate_without_metadata() {
        if cfg!(miri) {
            return;
        }

        let fixture = FixtureCrate::without_metadata();

        let error = BindingMetadataBuild::new(fixture.manifest())
            .read()
            .expect_err("metadata is required");

        assert!(matches!(
            error,
            BindingMetadataBuildError::Metadata(BindingMetadataReadError::NoMetadata { .. })
        ));
    }

    struct FixtureCrate {
        root: PathBuf,
        manifest: PathBuf,
    }

    impl FixtureCrate {
        fn with_metadata(envelope: &BindingMetadataEnvelope) -> Self {
            Self::write(Source::with_metadata(envelope))
        }

        fn without_metadata() -> Self {
            Self::write(Source::without_metadata())
        }

        fn write(source: Source) -> Self {
            let root = temp_root("boltffi-bindgen-cargo-metadata");
            let source_dir = root.join("src");
            let manifest = root.join("Cargo.toml");
            fs::create_dir_all(&source_dir).expect("create metadata fixture source dir");
            fs::write(
                &manifest,
                "[package]\nname = \"metadata_fixture\"\nversion = \"0.0.0\"\nedition = \"2024\"\n\n[lib]\npath = \"src/lib.rs\"\n",
            )
            .expect("write metadata fixture manifest");
            fs::write(source_dir.join("lib.rs"), source.into_string())
                .expect("write metadata fixture lib");
            Self { root, manifest }
        }

        fn manifest(&self) -> PathBuf {
            self.manifest.clone()
        }
    }

    impl Drop for FixtureCrate {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.root);
        }
    }

    struct Source {
        code: String,
    }

    impl Source {
        fn with_metadata(envelope: &BindingMetadataEnvelope) -> Self {
            let section_bytes = envelope.to_section_bytes().expect("metadata section bytes");
            let length = section_bytes.len();
            let bytes = section_bytes
                .iter()
                .map(u8::to_string)
                .collect::<Vec<_>>()
                .join(", ");
            let mach_o_section = BindingMetadataSection::MachO.link_section();
            let object_section = BindingMetadataSection::Object.link_section();
            Self {
                code: format!(
                    "#![allow(unexpected_cfgs)]\n#[cfg(boltffi_metadata)]\n#[cfg_attr(target_vendor = \"apple\", unsafe(link_section = \"{mach_o_section}\"))]\n#[cfg_attr(not(target_vendor = \"apple\"), unsafe(link_section = \"{object_section}\"))]\n#[used]\nstatic BOLTFFI_METADATA: [u8; {length}] = [{bytes}];\npub fn exported() -> u32 {{ 1 }}\n"
                ),
            }
        }

        fn without_metadata() -> Self {
            Self {
                code: "pub fn exported() -> u32 { 1 }\n".to_owned(),
            }
        }

        fn into_string(self) -> String {
            self.code
        }
    }

    fn metadata_envelope() -> BindingMetadataEnvelope {
        let source = SourceContract::new(PackageInfo::new("demo", None));
        let lowered = lower_with_declarations::<Native>(&source).expect("empty source lowers");
        BindingMetadataEnvelope::new(SerializedBindings::native(lowered.into_bindings()))
            .expect("metadata envelope")
    }

    fn temp_root(prefix: &str) -> PathBuf {
        std::env::temp_dir().join(format!(
            "{prefix}-{}",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("system time after unix epoch")
                .as_nanos()
        ))
    }
}
