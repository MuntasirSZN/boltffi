use std::path::{Path, PathBuf};

use boltffi_ast::PackageInfo;

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct ScanInput {
    root: PathBuf,
    package: PackageInfo,
    manifest_dir: Option<PathBuf>,
}

impl ScanInput {
    pub fn new(root: impl Into<PathBuf>, package: PackageInfo) -> Self {
        Self {
            root: root.into(),
            package,
            manifest_dir: None,
        }
    }

    pub fn with_manifest_dir(mut self, manifest_dir: impl Into<PathBuf>) -> Self {
        self.manifest_dir = Some(manifest_dir.into());
        self
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn package(&self) -> &PackageInfo {
        &self.package
    }

    pub fn manifest_dir(&self) -> Option<&Path> {
        self.manifest_dir.as_deref()
    }
}
