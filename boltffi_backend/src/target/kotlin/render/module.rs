use askama::Template as AskamaTemplate;
use boltffi_binding::{DeclarationRef, Native};

use crate::{
    bridge::jni::JniBridgeContract,
    core::{Diagnostic, FilePath, GeneratedFile, GeneratedOutput, RenderedDeclaration, Result},
    target::kotlin::{
        KotlinHost, KotlinPackage,
        render::native::{NativeFunction, NativeMethods},
    },
};

#[derive(AskamaTemplate)]
#[template(path = "target/kotlin/module.kt", escape = "none")]
struct ModuleTemplate {
    package: KotlinPackage,
    native_functions: Vec<NativeFunction>,
    functions: Vec<String>,
}

pub struct Module<'host, 'bridge, 'decl> {
    host: &'host KotlinHost,
    bridge: &'bridge JniBridgeContract,
    declarations: Vec<RenderedDeclaration<'decl, Native>>,
}

impl<'host, 'bridge, 'decl> Module<'host, 'bridge, 'decl> {
    pub fn new(
        host: &'host KotlinHost,
        bridge: &'bridge JniBridgeContract,
        declarations: Vec<RenderedDeclaration<'decl, Native>>,
    ) -> Self {
        Self {
            host,
            bridge,
            declarations,
        }
    }

    pub fn render(self) -> Result<GeneratedOutput> {
        let diagnostics = self.diagnostics();
        let native_functions = self.native_functions()?;
        let functions = self.functions();
        let contents = ModuleTemplate {
            package: self.host.package().clone(),
            native_functions,
            functions,
        }
        .render()?;
        Ok(GeneratedOutput::new(
            vec![GeneratedFile::new(
                FilePath::new(self.host.file().path(self.host.package()))?,
                contents,
            )],
            diagnostics,
        ))
    }

    fn native_functions(&self) -> Result<Vec<NativeFunction>> {
        let methods = NativeMethods::new(self.bridge);
        self.declarations
            .iter()
            .filter_map(|declaration| {
                let DeclarationRef::Function(function) = declaration.declaration() else {
                    return None;
                };
                (!declaration.emitted().primary_chunk().is_empty())
                    .then(|| methods.function(function))
            })
            .collect()
    }

    fn functions(&self) -> Vec<String> {
        self.declarations
            .iter()
            .filter_map(|declaration| {
                let DeclarationRef::Function(_) = declaration.declaration() else {
                    return None;
                };
                let chunk = declaration.emitted().primary_chunk();
                (!chunk.is_empty()).then(|| chunk.as_str().to_owned())
            })
            .collect()
    }

    fn diagnostics(&self) -> Vec<Diagnostic> {
        self.declarations
            .iter()
            .flat_map(|declaration| declaration.emitted().diagnostics().iter().cloned())
            .collect()
    }
}
