use std::path::{Path, PathBuf};

use crate::cli::Result;
use crate::config::Config;
use crate::pack::java::plan::PreparedJvmPackaging;
use crate::pack::java::{
    selected_jvm_package_artifact_name, selected_jvm_package_source_directory,
};

use super::layout::KmpPackageLayout;

/// Inputs and derived paths needed to package a generated KMP module.
pub(crate) struct KmpPackagingPlan {
    source_directory: PathBuf,
    source_crate_name: String,
    artifact_name: String,
    layout: KmpPackageLayout,
    jvm_packaging: PreparedJvmPackaging,
}

impl KmpPackagingPlan {
    /// Builds a KMP packaging plan from the configured crate and prepared JVM matrix.
    pub(crate) fn new(config: &Config, jvm_packaging: PreparedJvmPackaging) -> Result<Self> {
        let source_directory =
            selected_jvm_package_source_directory(&jvm_packaging.packaging_targets)?;
        let artifact_name = selected_jvm_package_artifact_name(&jvm_packaging.packaging_targets)?;
        Ok(Self {
            source_directory,
            source_crate_name: config.library_name().to_string(),
            artifact_name: artifact_name.to_string(),
            layout: KmpPackageLayout::from_config(config),
            jvm_packaging,
        })
    }

    /// Returns the source directory selected for generation and native builds.
    pub(crate) fn source_directory(&self) -> &Path {
        &self.source_directory
    }

    /// Returns the Rust source crate name.
    pub(crate) fn source_crate_name(&self) -> &str {
        &self.source_crate_name
    }

    /// Returns the native artifact name selected from the JVM packaging matrix.
    pub(crate) fn artifact_name(&self) -> &str {
        &self.artifact_name
    }

    /// Returns the generated KMP project layout.
    pub(crate) fn layout(&self) -> &KmpPackageLayout {
        &self.layout
    }

    /// Returns the prepared JVM packaging matrix reused by KMP desktop packaging.
    pub(crate) fn jvm_packaging(&self) -> &PreparedJvmPackaging {
        &self.jvm_packaging
    }
}
