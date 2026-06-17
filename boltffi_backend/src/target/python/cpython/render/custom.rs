use boltffi_binding::{Bindings, CustomTypeDecl, CustomTypeId, DeclarationRef, Native, TypeRef};

use crate::core::{Error, RenderContext, Result};

pub struct CustomTypes<'bindings> {
    bindings: &'bindings Bindings<Native>,
    target: &'static str,
}

impl<'bindings> CustomTypes<'bindings> {
    pub fn from_context(context: &'bindings RenderContext<'bindings, Native>) -> Self {
        Self::new(context.bindings(), context.target())
    }

    pub const fn new(bindings: &'bindings Bindings<Native>, target: &'static str) -> Self {
        Self { bindings, target }
    }

    pub fn representation(&self, id: CustomTypeId) -> Result<&'bindings TypeRef> {
        self.custom_type(id).map(CustomTypeDecl::representation)
    }

    pub fn custom_type(&self, id: CustomTypeId) -> Result<&'bindings CustomTypeDecl> {
        self.bindings
            .decls()
            .iter()
            .find_map(|decl| match DeclarationRef::from(decl) {
                DeclarationRef::CustomType(custom) if custom.id() == id => Some(custom),
                DeclarationRef::Record(_)
                | DeclarationRef::Enum(_)
                | DeclarationRef::Function(_)
                | DeclarationRef::Class(_)
                | DeclarationRef::Callback(_)
                | DeclarationRef::Stream(_)
                | DeclarationRef::Constant(_)
                | DeclarationRef::CustomType(_) => None,
            })
            .ok_or(Error::UnsupportedTarget {
                target: self.target,
                shape: "custom type without declaration",
            })
    }
}
