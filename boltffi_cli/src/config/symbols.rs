use std::path::PathBuf;

use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize, Clone, Default)]
pub struct DebugSymbolsConfig {
    #[serde(default)]
    pub enabled: bool,
    pub output: Option<PathBuf>,
    #[serde(default)]
    pub format: DebugSymbolsFormat,
    #[serde(default)]
    pub bundle: DebugSymbolsBundle,
}

#[derive(Debug, Deserialize, Serialize, Clone, Copy, PartialEq, Eq, Default)]
#[serde(rename_all = "lowercase")]
pub enum DebugSymbolsFormat {
    #[default]
    Zip,
}

#[derive(Debug, Deserialize, Serialize, Clone, Copy, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum DebugSymbolsBundle {
    #[default]
    Unstripped,
}
