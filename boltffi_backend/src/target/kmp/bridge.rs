//! Shared JNI bridge stack for Kotlin Multiplatform JVM-family targets.

use std::path::PathBuf;

use boltffi_binding::{Bindings, Native};

use crate::{
    bridge::{
        c::CBridge,
        jni::{JniBridge, JniBridgeContract, JniHeaderStyle},
    },
    core::{
        GeneratedOutput, Result, bridge,
        bridge::{BridgeBackend, BridgeOutput},
        contract::sealed,
    },
};

use super::{emit::KMP_GENERATED_C_HEADER_DIR, library::cargo_crate_name};

const JVM_SOURCE_SETS: [&str; 2] = ["jvmMain", "androidMain"];

/// JNI bridge stack used by the KMP JVM and Android source sets.
///
/// Both source sets expose the same generated `Native` JVM class. Each source
/// set owns its JNI translation unit, while the typed contract returned to the
/// KMP host comes from the first source set. The C headers themselves are
/// staged by the KMP packager beside these generated translation units.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
#[non_exhaustive]
pub struct KmpBridge {
    internal_package: String,
}

impl KmpBridge {
    /// Creates a bridge for the internal JVM implementation package.
    pub fn new(internal_package: impl Into<String>) -> Self {
        Self {
            internal_package: internal_package.into(),
        }
    }

    fn source_set_layout(
        &self,
        source_set: &'static str,
        bindings: &Bindings<Native>,
    ) -> KmpJniSourceSet {
        let c_directory = PathBuf::from(format!("src/{source_set}/c"));
        let header_name = cargo_crate_name(bindings);
        KmpJniSourceSet {
            header: c_directory
                .join(KMP_GENERATED_C_HEADER_DIR)
                .join(format!("{header_name}.h")),
            source: c_directory.join("jni_glue.c"),
        }
    }

    fn build_source_set(
        &self,
        source_set: &'static str,
        bindings: &Bindings<Native>,
    ) -> Result<(JniBridgeContract, GeneratedOutput)> {
        let layout = self.source_set_layout(source_set, bindings);
        let c_bridge = CBridge::new(layout.header)?;
        let c_contract = c_bridge.build_contract(bindings)?;
        let jni_bridge = JniBridge::new(&self.internal_package, "Native", layout.source)?
            .header_style(JniHeaderStyle::SearchPath);
        let jni_contract = jni_bridge.build_contract(&c_contract)?;
        let output = jni_bridge.render_bridge(&c_contract, &jni_contract)?;
        Ok((jni_contract, output))
    }
}

impl sealed::BridgeStack for KmpBridge {}

impl bridge::BridgeStack for KmpBridge {
    type Surface = Native;
    type Contract = JniBridgeContract;

    fn build(&self, bindings: &Bindings<Self::Surface>) -> Result<BridgeOutput<Self::Contract>> {
        let mut source_sets = JVM_SOURCE_SETS.into_iter();
        let first = source_sets
            .next()
            .expect("KMP JVM source-set list must not be empty");
        let (contract, mut output) = self.build_source_set(first, bindings)?;
        for source_set in source_sets {
            let (_, source_set_output) = self.build_source_set(source_set, bindings)?;
            output.append(source_set_output);
        }
        Ok(BridgeOutput::new(contract, output))
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct KmpJniSourceSet {
    header: PathBuf,
    source: PathBuf,
}
