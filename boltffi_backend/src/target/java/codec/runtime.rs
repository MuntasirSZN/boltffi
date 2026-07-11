use askama::Template as AskamaTemplate;
use boltffi_binding::CanonicalName;

use crate::core::{AuxChunk, HelperId, Result, TextChunk};

#[derive(AskamaTemplate)]
#[template(path = "target/java/runtime/wire.java", escape = "none")]
struct RuntimeTemplate;

pub struct Runtime;

impl Runtime {
    pub fn helper() -> Result<AuxChunk> {
        Ok(AuxChunk::Helper {
            id: HelperId::new(CanonicalName::single("java_wire_runtime")),
            text: TextChunk::new(RuntimeTemplate.render()?),
        })
    }
}
