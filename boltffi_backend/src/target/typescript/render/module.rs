use askama::Template as AskamaTemplate;
use boltffi_binding::Wasm32;

use crate::core::{
    FileLayout, FilePath, FilePlan, GeneratedOutput, RenderedDeclaration, Result, TextChunk,
};

use super::super::{name_style::ModuleName, syntax::StringLiteral};

#[derive(AskamaTemplate)]
#[template(path = "target/typescript/browser.ts", escape = "none")]
struct BrowserPreamble<'module> {
    runtime_package: &'module StringLiteral,
}

#[derive(AskamaTemplate)]
#[template(path = "target/typescript/node.ts", escape = "none")]
struct NodePreamble<'module> {
    runtime_package: &'module StringLiteral,
    wasm_file: &'module StringLiteral,
}

#[derive(AskamaTemplate)]
#[template(path = "target/typescript/node_init.ts", escape = "none")]
struct NodeInitialization;

pub struct Module<'module> {
    name: &'module ModuleName,
    runtime_package: &'module StringLiteral,
}

impl<'module> Module<'module> {
    pub fn new(name: &'module ModuleName, runtime_package: &'module StringLiteral) -> Self {
        Self {
            name,
            runtime_package,
        }
    }

    pub fn render<'decl>(
        &self,
        declarations: Vec<RenderedDeclaration<'decl, Wasm32>>,
    ) -> Result<GeneratedOutput> {
        let browser = FileLayout::new()
            .with_file(
                FilePlan::all(FilePath::new(self.name.browser_path())?).with_preamble(
                    BrowserPreamble {
                        runtime_package: self.runtime_package,
                    }
                    .render()?,
                ),
            )
            .assemble_declarations(declarations.clone())?;
        let wasm_file = StringLiteral::new(&self.name.wasm_file());
        let node = FileLayout::new()
            .with_file(
                FilePlan::all(FilePath::new(self.name.node_path())?)
                    .with_preamble(
                        NodePreamble {
                            runtime_package: self.runtime_package,
                            wasm_file: &wasm_file,
                        }
                        .render()?,
                    )
                    .with_postamble(TextChunk::new(NodeInitialization.render()?)),
            )
            .assemble_declarations(declarations)?;
        Ok(GeneratedOutput::combine([browser, node]))
    }
}
