use boltffi_binding::{
    CodecNode, CustomConverterPath, CustomConverterPathRoot, CustomTypeConverter,
};
use proc_macro2::TokenStream;
use quote::quote;
use syn::{Expr, ExprPath, parse_str};

use crate::experimental::{error::Error, expansion::CustomTypeDeclarations, target::Target};

pub struct Incoming<'context, 'a, S: Target> {
    codec: &'a CodecNode,
    custom_declarations: CustomTypeDeclarations<'context, 'a, S>,
}

pub struct Outgoing<'context, 'a, S: Target> {
    codec: &'a CodecNode,
    custom_declarations: CustomTypeDeclarations<'context, 'a, S>,
}

pub struct IncomingConversion {
    tokens: TokenStream,
    fallible: bool,
    changed: bool,
}

struct ConverterRenderer<'a> {
    converter: &'a CustomTypeConverter,
}

struct PathRenderer<'a> {
    path: &'a CustomConverterPath,
}

impl IncomingConversion {
    pub fn tokens(&self) -> &TokenStream {
        &self.tokens
    }

    pub const fn fallible(&self) -> bool {
        self.fallible
    }

    pub const fn changed(&self) -> bool {
        self.changed
    }
}

impl<'context, 'a, S: Target> Incoming<'context, 'a, S> {
    pub const fn new(
        codec: &'a CodecNode,
        custom_declarations: CustomTypeDeclarations<'context, 'a, S>,
    ) -> Self {
        Self {
            codec,
            custom_declarations,
        }
    }

    pub fn convert(&self, value: TokenStream) -> Result<IncomingConversion, Error> {
        self.convert_node(self.codec, value)
    }

    fn convert_node(
        &self,
        codec: &CodecNode,
        value: TokenStream,
    ) -> Result<IncomingConversion, Error> {
        match codec {
            CodecNode::Custom(id) => {
                let custom = self.custom_declarations.get(*id)?;
                let converter =
                    ConverterRenderer::new(custom.converters().try_from_ffi()).render()?;
                Ok(IncomingConversion {
                    tokens: quote! { (#converter)(#value) },
                    fallible: true,
                    changed: true,
                })
            }
            CodecNode::Optional(inner) => self.convert_optional(inner, value),
            CodecNode::Sequence { element, .. } => self.convert_sequence(element, value),
            CodecNode::Result { ok, err } => self.convert_result(ok, err, value),
            CodecNode::Tuple(elements) => Err(match elements.iter().any(contains_custom) {
                true => Error::UnsupportedExpansion("custom conversion inside tuple codec"),
                false => Error::UnsupportedExpansion("tuple codec conversion"),
            }),
            CodecNode::Map { key, value } => {
                Err(match contains_custom(key) || contains_custom(value) {
                    true => Error::UnsupportedExpansion("custom conversion inside map codec"),
                    false => Error::UnsupportedExpansion("map codec conversion"),
                })
            }
            _ => Ok(IncomingConversion {
                tokens: value,
                fallible: false,
                changed: false,
            }),
        }
    }

    fn convert_optional(
        &self,
        inner: &CodecNode,
        value: TokenStream,
    ) -> Result<IncomingConversion, Error> {
        let inner = self.convert_node(inner, quote! { value })?;
        let inner_tokens = inner.tokens();
        Ok(match inner.fallible {
            true => IncomingConversion {
                tokens: quote! {
                    match #value {
                        Some(value) => #inner_tokens.map(Some),
                        None => Ok(None),
                    }
                },
                fallible: true,
                changed: true,
            },
            false => IncomingConversion {
                tokens: quote! { #value.map(|value| #inner_tokens) },
                fallible: false,
                changed: inner.changed,
            },
        })
    }

    fn convert_sequence(
        &self,
        element: &CodecNode,
        value: TokenStream,
    ) -> Result<IncomingConversion, Error> {
        let element = self.convert_node(element, quote! { value })?;
        let element_tokens = element.tokens();
        Ok(match element.fallible {
            true => IncomingConversion {
                tokens: quote! {
                    #value
                        .into_iter()
                        .map(|value| #element_tokens)
                        .collect::<Result<Vec<_>, _>>()
                },
                fallible: true,
                changed: true,
            },
            false => IncomingConversion {
                tokens: quote! {
                    #value
                        .into_iter()
                        .map(|value| #element_tokens)
                        .collect::<Vec<_>>()
                },
                fallible: false,
                changed: element.changed,
            },
        })
    }

    fn convert_result(
        &self,
        ok: &CodecNode,
        err: &CodecNode,
        value: TokenStream,
    ) -> Result<IncomingConversion, Error> {
        let ok = self.convert_node(ok, quote! { value })?;
        let err = self.convert_node(err, quote! { error })?;
        let ok_tokens = ok.tokens();
        let err_tokens = err.tokens();
        let ok_arm = match ok.fallible {
            true => quote! { #ok_tokens.map(Ok) },
            false => quote! { Ok(Ok(#ok_tokens)) },
        };
        let err_arm = match err.fallible {
            true => quote! { #err_tokens.map(Err) },
            false => quote! { Ok(Err(#err_tokens)) },
        };
        Ok(IncomingConversion {
            tokens: quote! {
                match #value {
                    Ok(value) => #ok_arm,
                    Err(error) => #err_arm,
                }
            },
            fallible: ok.fallible || err.fallible,
            changed: ok.changed || err.changed,
        })
    }
}

impl<'context, 'a, S: Target> Outgoing<'context, 'a, S> {
    pub const fn new(
        codec: &'a CodecNode,
        custom_declarations: CustomTypeDeclarations<'context, 'a, S>,
    ) -> Self {
        Self {
            codec,
            custom_declarations,
        }
    }

    pub fn has_custom_conversion(&self) -> bool {
        contains_custom(self.codec)
    }

    pub fn convert(&self, value: TokenStream) -> Result<TokenStream, Error> {
        self.convert_node(self.codec, value)
    }

    fn convert_node(&self, codec: &CodecNode, value: TokenStream) -> Result<TokenStream, Error> {
        match codec {
            CodecNode::Custom(id) => {
                let custom = self.custom_declarations.get(*id)?;
                let converter = ConverterRenderer::new(custom.converters().into_ffi()).render()?;
                Ok(quote! { (#converter)(&#value) })
            }
            CodecNode::Optional(inner) => {
                let inner = self.convert_node(inner, quote! { value })?;
                Ok(quote! { #value.map(|value| #inner) })
            }
            CodecNode::Sequence { element, .. } => {
                let element = self.convert_node(element, quote! { value })?;
                Ok(quote! {
                    #value
                        .into_iter()
                        .map(|value| #element)
                        .collect::<Vec<_>>()
                })
            }
            CodecNode::Result { ok, err } => {
                let ok = self.convert_node(ok, quote! { value })?;
                let err = self.convert_node(err, quote! { error })?;
                Ok(quote! {
                    match #value {
                        Ok(value) => Ok(#ok),
                        Err(error) => Err(#err),
                    }
                })
            }
            CodecNode::Tuple(elements) => Err(match elements.iter().any(contains_custom) {
                true => Error::UnsupportedExpansion("custom conversion inside tuple codec"),
                false => Error::UnsupportedExpansion("tuple codec conversion"),
            }),
            CodecNode::Map { key, value } => {
                Err(match contains_custom(key) || contains_custom(value) {
                    true => Error::UnsupportedExpansion("custom conversion inside map codec"),
                    false => Error::UnsupportedExpansion("map codec conversion"),
                })
            }
            _ => Ok(value),
        }
    }
}

impl<'a> ConverterRenderer<'a> {
    const fn new(converter: &'a CustomTypeConverter) -> Self {
        Self { converter }
    }

    fn render(self) -> Result<TokenStream, Error> {
        match self.converter {
            CustomTypeConverter::Path(path) => PathRenderer::new(path).render(),
            CustomTypeConverter::Expression(expression) => parse_str::<Expr>(expression.source())
                .map(|expression| quote! { #expression })
                .map_err(|_| Error::SourceSyntaxMismatch("custom converter is not Rust syntax")),
            _ => Err(Error::UnsupportedExpansion("unknown custom converter")),
        }
    }
}

impl<'a> PathRenderer<'a> {
    const fn new(path: &'a CustomConverterPath) -> Self {
        Self { path }
    }

    fn render(self) -> Result<TokenStream, Error> {
        parse_str::<ExprPath>(&self.source()?)
            .map(|path| quote! { #path })
            .map_err(|_| Error::SourceSyntaxMismatch("custom converter path is not Rust syntax"))
    }

    fn source(&self) -> Result<String, Error> {
        Ok(self.prefix()? + &self.segments())
    }

    fn prefix(&self) -> Result<String, Error> {
        Ok(match self.path.root() {
            CustomConverterPathRoot::Relative => String::new(),
            CustomConverterPathRoot::Crate => "crate::".to_owned(),
            CustomConverterPathRoot::Self_ => "self::".to_owned(),
            CustomConverterPathRoot::Super(levels) => {
                std::iter::repeat_n("super", levels.get())
                    .collect::<Vec<_>>()
                    .join("::")
                    + "::"
            }
            CustomConverterPathRoot::Absolute => "::".to_owned(),
            _ => {
                return Err(Error::UnsupportedExpansion(
                    "unknown custom converter path root",
                ));
            }
        })
    }

    fn segments(&self) -> String {
        self.path
            .segments()
            .iter()
            .map(|segment| segment.as_str())
            .collect::<Vec<_>>()
            .join("::")
    }
}

fn contains_custom(codec: &CodecNode) -> bool {
    match codec {
        CodecNode::Custom(_) => true,
        CodecNode::Optional(inner) | CodecNode::Sequence { element: inner, .. } => {
            contains_custom(inner)
        }
        CodecNode::Result { ok, err } => contains_custom(ok) || contains_custom(err),
        CodecNode::Tuple(elements) => elements.iter().any(contains_custom),
        CodecNode::Map { key, value } => contains_custom(key) || contains_custom(value),
        _ => false,
    }
}
