use askama::Template;
use boltffi_binding::{
    CStyleEnumDecl, CStyleVariantDecl, EnumDecl, ExportedMethodDecl, Native, NativeSymbol,
};

use crate::{
    bridge::c::CBridgeContract,
    core::{Emitted, RenderContext, Result},
    target::swift::{
        SwiftHost,
        name_style::Name,
        render::{
            Documentation, SwiftType,
            function::{AssociatedFunction, Receiver},
        },
        syntax::{Identifier, TypeName},
    },
};

#[derive(Template)]
#[template(path = "target/swift/enumeration.swift", escape = "none")]
struct EnumerationTemplate<'a> {
    enumeration: &'a Enumeration,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Enumeration {
    documentation: Documentation,
    name: TypeName,
    raw_type: TypeName,
    variants: Vec<Variant>,
    initializers: Vec<AssociatedFunction>,
    static_methods: Vec<AssociatedFunction>,
    instance_methods: Vec<AssociatedFunction>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct Variant {
    documentation: Documentation,
    name: Identifier,
    discriminant: i128,
}

impl Enumeration {
    pub fn from_declaration(
        declaration: &EnumDecl<Native>,
        bridge: &CBridgeContract,
        context: &RenderContext<Native>,
    ) -> Result<Self> {
        match declaration {
            EnumDecl::CStyle(enumeration) => Self::from_c_style(enumeration, bridge, context),
            EnumDecl::Data(_) => Err(SwiftHost::unsupported("data enum declaration")),
            _ => Err(SwiftHost::unsupported("unknown enum declaration")),
        }
    }

    pub fn render(&self) -> Result<Emitted> {
        let mut source = EnumerationTemplate { enumeration: self }.render()?;
        source.push_str("\n\n");
        Ok(Emitted::primary(source))
    }

    fn name(&self) -> &TypeName {
        &self.name
    }

    fn documentation(&self) -> &Documentation {
        &self.documentation
    }

    fn raw_type(&self) -> &TypeName {
        &self.raw_type
    }

    fn variants(&self) -> &[Variant] {
        &self.variants
    }

    fn initializers(&self) -> &[AssociatedFunction] {
        &self.initializers
    }

    fn static_methods(&self) -> &[AssociatedFunction] {
        &self.static_methods
    }

    fn instance_methods(&self) -> &[AssociatedFunction] {
        &self.instance_methods
    }

    fn from_c_style(
        enumeration: &CStyleEnumDecl<Native>,
        bridge: &CBridgeContract,
        context: &RenderContext<Native>,
    ) -> Result<Self> {
        Ok(Self {
            documentation: Documentation::new(enumeration.meta().doc(), ""),
            name: Name::new(enumeration.name()).type_name(),
            raw_type: SwiftType::primitive(enumeration.repr().primitive())?,
            variants: enumeration
                .variants()
                .iter()
                .map(Variant::from_declaration)
                .collect::<Result<Vec<_>>>()?,
            initializers: enumeration
                .initializers()
                .iter()
                .map(|initializer| {
                    AssociatedFunction::from_initializer(initializer, bridge, context)
                })
                .collect::<Result<Vec<_>>>()?,
            static_methods: Self::methods(enumeration.methods(), None, bridge, context)?,
            instance_methods: Self::methods(
                enumeration.methods(),
                Some(Receiver::direct()),
                bridge,
                context,
            )?,
        })
    }

    fn methods(
        methods: &[ExportedMethodDecl<Native, NativeSymbol>],
        receiver: Option<Receiver>,
        bridge: &CBridgeContract,
        context: &RenderContext<Native>,
    ) -> Result<Vec<AssociatedFunction>> {
        methods
            .iter()
            .filter(|method| method.callable().receiver().is_some() == receiver.is_some())
            .map(|method| {
                AssociatedFunction::from_method(method, receiver.clone(), bridge, context)
            })
            .collect()
    }
}

impl Variant {
    fn from_declaration(variant: &CStyleVariantDecl) -> Result<Self> {
        Ok(Self {
            documentation: Documentation::new(variant.meta().doc(), "    "),
            name: Name::new(variant.name()).variant()?,
            discriminant: variant.discriminant().get(),
        })
    }

    fn name(&self) -> &Identifier {
        &self.name
    }

    fn documentation(&self) -> &Documentation {
        &self.documentation
    }

    const fn discriminant(&self) -> i128 {
        self.discriminant
    }
}
