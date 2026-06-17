use std::path::PathBuf;

use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct KotlinMultiplatformConfig {
    #[serde(default = "default_kotlin_multiplatform_output")]
    pub output: PathBuf,
    #[serde(default)]
    pub enabled: bool,
    pub package: Option<String>,
    pub module_name: Option<String>,
}

impl Default for KotlinMultiplatformConfig {
    fn default() -> Self {
        Self {
            output: default_kotlin_multiplatform_output(),
            enabled: false,
            package: None,
            module_name: None,
        }
    }
}

fn default_kotlin_multiplatform_output() -> PathBuf {
    PathBuf::from("dist/kotlin-multiplatform")
}
