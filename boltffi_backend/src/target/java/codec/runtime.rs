use askama::Template as AskamaTemplate;
use boltffi_binding::CanonicalName;

use crate::core::{AuxChunk, HelperId, Result, TextChunk};

#[derive(AskamaTemplate)]
#[template(path = "target/java/runtime/wire.java", escape = "none")]
struct RuntimeTemplate;

#[derive(AskamaTemplate)]
#[template(path = "target/java/runtime/direct_vector.java", escape = "none")]
struct DirectVectorTemplate;

pub struct Runtime;

impl Runtime {
    pub fn helper() -> Result<AuxChunk> {
        Ok(AuxChunk::Helper {
            id: HelperId::new(CanonicalName::single("java_wire_runtime")),
            text: TextChunk::new(RuntimeTemplate.render()?),
        })
    }

    pub fn direct_vector_helper() -> Result<AuxChunk> {
        Ok(AuxChunk::Helper {
            id: HelperId::new(CanonicalName::single("java_direct_vector_runtime")),
            text: TextChunk::new(DirectVectorTemplate.render()?),
        })
    }
}
