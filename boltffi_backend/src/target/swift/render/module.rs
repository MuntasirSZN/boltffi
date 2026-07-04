use askama::Template;

use boltffi_binding::{DeclarationRef, ErrorPayloadTypes, Native};

use crate::{
    bridge::c::CBridgeContract,
    core::{
        FileLayout, FilePath, FilePlan, GeneratedOutput, RenderContext, RenderedDeclaration, Result,
    },
    target::swift::{
        SwiftHost,
        render::{Enumeration, Record},
    },
};

#[derive(Template)]
#[template(path = "target/swift/module.swift", escape = "none")]
struct ModuleTemplate<'a> {
    module: &'a str,
}

pub struct Module<'host, 'bridge, 'context, 'decl> {
    host: &'host SwiftHost,
    bridge: &'bridge CBridgeContract,
    context: &'context RenderContext<'decl, Native>,
    error_payloads: ErrorPayloadTypes,
    declarations: Vec<RenderedDeclaration<'decl, Native>>,
}

impl<'host, 'bridge, 'context, 'decl> Module<'host, 'bridge, 'context, 'decl> {
    pub fn new(
        host: &'host SwiftHost,
        bridge: &'bridge CBridgeContract,
        context: &'context RenderContext<'decl, Native>,
        error_payloads: ErrorPayloadTypes,
        declarations: Vec<RenderedDeclaration<'decl, Native>>,
    ) -> Self {
        Self {
            host,
            bridge,
            context,
            error_payloads,
            declarations,
        }
    }

    pub fn render(self) -> Result<GeneratedOutput> {
        let declarations = self.declarations(&self.error_payloads)?;
        let mut preamble = ModuleTemplate {
            module: self.host.module().as_str(),
        }
        .render()?;
        preamble.push_str("\n\n");
        let file =
            FilePlan::all(FilePath::new(self.host.file_name().path())?).with_preamble(preamble);
        FileLayout::new()
            .with_file(file)
            .assemble_declarations(declarations)
    }

    fn declarations(
        &self,
        error_payloads: &ErrorPayloadTypes,
    ) -> Result<Vec<RenderedDeclaration<'decl, Native>>> {
        self.declarations
            .iter()
            .map(|declaration| self.declaration(declaration, error_payloads))
            .collect()
    }

    fn declaration(
        &self,
        declaration: &RenderedDeclaration<'decl, Native>,
        error_payloads: &ErrorPayloadTypes,
    ) -> Result<RenderedDeclaration<'decl, Native>> {
        match declaration.declaration() {
            DeclarationRef::Record(record) if error_payloads.contains_record(record.id()) => {
                Ok(RenderedDeclaration::new(
                    DeclarationRef::Record(record),
                    Record::from_declaration_as_error(record, self.bridge, self.context)?
                        .render()?,
                ))
            }
            DeclarationRef::Enum(enumeration) if error_payloads.contains_enum(enumeration.id()) => {
                Ok(RenderedDeclaration::new(
                    DeclarationRef::Enum(enumeration),
                    Enumeration::from_declaration_as_error(enumeration, self.bridge, self.context)?
                        .render()?,
                ))
            }
            _ => Ok(declaration.clone()),
        }
    }
}
