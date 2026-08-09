use askama::Template;
use boltffi_binding::{CustomTypeDecl, Native};

use crate::core::{Emitted, RenderContext, Result};

use super::super::{
    syntax::{Identifier, TypeFragment},
    type_name,
};
use super::{Documentation, declaration_name};

#[derive(Template)]
#[template(path = "target/dart/custom_type.dart", escape = "none")]
struct CustomTypeTemplate<'a> {
    custom_type: &'a CustomType,
}

pub struct CustomType {
    documentation: Documentation,
    name: Identifier,
    representation: TypeFragment,
}

impl CustomType {
    pub fn from_declaration(
        declaration: &CustomTypeDecl,
        context: &RenderContext<Native>,
    ) -> Result<Self> {
        Ok(Self {
            documentation: Documentation::new(declaration.meta().doc(), 0),
            name: declaration_name(declaration.name())?,
            representation: type_name::type_ref(declaration.representation(), context)?,
        })
    }

    pub fn render(self) -> Emitted {
        Emitted::primary(
            CustomTypeTemplate { custom_type: &self }
                .render()
                .expect("rendering an in-memory Dart custom-type template cannot fail"),
        )
    }

    fn documentation(&self) -> &Documentation {
        &self.documentation
    }

    fn name(&self) -> &Identifier {
        &self.name
    }

    fn representation(&self) -> &TypeFragment {
        &self.representation
    }
}
