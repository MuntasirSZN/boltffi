use std::marker::PhantomData;

use boltffi_binding::{
    BuiltinType, CallbackId, ClassId, CodecNode, CodecRead, CustomConverterPath,
    CustomConverterPathRoot, CustomTypeConverter, CustomTypeId, ElementCount, EnumId, MapKind, Op,
    Primitive, RecordId,
};
use proc_macro2::TokenStream;
use quote::quote;
use syn::{Expr, ExprPath, Index, parse_str};

use crate::experimental::{error::Error, expansion::Expansion, surface::RenderSurface, wrapper};

pub struct Incoming<'expansion, 'lowered, S: RenderSurface> {
    codec: &'lowered CodecNode,
    expansion: &'expansion Expansion<'lowered, S>,
}

pub struct Outgoing<'expansion, 'lowered, S: RenderSurface> {
    codec: &'lowered CodecNode,
    expansion: &'expansion Expansion<'lowered, S>,
}

pub struct BorrowedOutgoing<'expansion, 'lowered, S: RenderSurface> {
    codec: &'lowered CodecNode,
    expansion: &'expansion Expansion<'lowered, S>,
}

pub struct IncomingConversion {
    tokens: TokenStream,
    fallible: bool,
    changed: bool,
}

struct IncomingConverter<'expansion, 'lowered, S: RenderSurface> {
    expansion: &'expansion Expansion<'lowered, S>,
}

struct OutgoingConverter<'expansion, 'lowered, S: RenderSurface, M> {
    expansion: &'expansion Expansion<'lowered, S>,
    mode: PhantomData<M>,
}

struct IncomingTransform {
    tokens: TokenStream,
    fallible: bool,
    changed: bool,
}

enum OutgoingTransform {
    Identity,
    ReferenceConverter(TokenStream),
    ValueConverter(TokenStream),
    Optional(Box<OutgoingTransform>),
    Sequence(Box<OutgoingTransform>),
    Tuple(Vec<OutgoingTransform>),
    Result {
        ok: Box<OutgoingTransform>,
        err: Box<OutgoingTransform>,
    },
    Map {
        key: Box<OutgoingTransform>,
        value: Box<OutgoingTransform>,
    },
    CustomRepresentation {
        custom: Box<OutgoingTransform>,
        representation: Box<OutgoingTransform>,
    },
}

struct OwnedValue;

struct BorrowedValue;

struct ConverterRenderer<'lowered> {
    converter: &'lowered CustomTypeConverter,
}

struct PathRenderer<'lowered> {
    path: &'lowered CustomConverterPath,
}

struct RepresentationType;

trait OutgoingMode {
    fn custom(converter: TokenStream) -> OutgoingTransform;

    fn optional(value: TokenStream, inner: TokenStream) -> TokenStream;

    fn sequence(value: TokenStream, element: TokenStream) -> TokenStream;

    fn tuple_field(value: TokenStream, index: Index) -> TokenStream;

    fn map(value: TokenStream, key: TokenStream, item: TokenStream) -> TokenStream;
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

    fn value_or_return_error(&self) -> TokenStream {
        let tokens = self.tokens();
        match self.fallible {
            true => quote! {
                match #tokens {
                    Ok(value) => value,
                    Err(error) => return Err(error),
                }
            },
            false => quote! { #tokens },
        }
    }
}

impl IncomingTransform {
    fn identity() -> Self {
        Self {
            tokens: TokenStream::new(),
            fallible: false,
            changed: false,
        }
    }

    fn new(tokens: TokenStream, fallible: bool, changed: bool) -> Self {
        Self {
            tokens,
            fallible,
            changed,
        }
    }

    fn apply(&self, value: TokenStream) -> IncomingConversion {
        match self.changed {
            true => IncomingConversion {
                tokens: {
                    let tokens = &self.tokens;
                    quote! { (#tokens)(#value) }
                },
                fallible: self.fallible,
                changed: true,
            },
            false => IncomingConversion {
                tokens: value,
                fallible: false,
                changed: false,
            },
        }
    }

    fn value_or_return_error(&self, value: TokenStream) -> TokenStream {
        self.apply(value).value_or_return_error()
    }
}

impl OutgoingTransform {
    fn changed(&self) -> bool {
        !matches!(self, Self::Identity)
    }

    fn apply<M>(&self, value: TokenStream) -> TokenStream
    where
        M: OutgoingMode,
    {
        match self {
            Self::Identity => value,
            Self::ReferenceConverter(converter) => quote! { (#converter)(&#value) },
            Self::ValueConverter(converter) => quote! { (#converter)(#value) },
            Self::Optional(inner) => {
                let inner = inner.apply::<M>(quote! { value });
                M::optional(value, inner)
            }
            Self::Sequence(element) => {
                let element = element.apply::<M>(quote! { value });
                M::sequence(value, element)
            }
            Self::Tuple(elements) => {
                let values = elements
                    .iter()
                    .enumerate()
                    .map(|(index, element)| {
                        let field = M::tuple_field(value.clone(), Index::from(index));
                        element.apply::<M>(field)
                    })
                    .collect::<Vec<_>>();
                quote! { (#(#values,)*) }
            }
            Self::Result { ok, err } => {
                let ok = ok.apply::<M>(quote! { value });
                let err = err.apply::<M>(quote! { error });
                quote! {
                    match #value {
                        Ok(value) => Ok(#ok),
                        Err(error) => Err(#err),
                    }
                }
            }
            Self::Map { key, value: item } => {
                let key = key.apply::<M>(quote! { key });
                let item = item.apply::<M>(quote! { value });
                M::map(value, key, item)
            }
            Self::CustomRepresentation {
                custom,
                representation,
            } => {
                let custom = custom.apply::<M>(value);
                let representation = representation.apply::<M>(quote! { __boltffi_representation });
                quote! {
                    {
                        let __boltffi_representation = #custom;
                        #representation
                    }
                }
            }
        }
    }
}

impl<'expansion, 'lowered, S: RenderSurface> IncomingConverter<'expansion, 'lowered, S> {
    fn new(expansion: &'expansion Expansion<'lowered, S>) -> Self {
        Self { expansion }
    }
}

impl<'expansion, 'lowered, S: RenderSurface, M> OutgoingConverter<'expansion, 'lowered, S, M> {
    fn new(expansion: &'expansion Expansion<'lowered, S>) -> Self {
        Self {
            expansion,
            mode: PhantomData,
        }
    }
}

impl OutgoingMode for OwnedValue {
    fn custom(converter: TokenStream) -> OutgoingTransform {
        OutgoingTransform::ReferenceConverter(converter)
    }

    fn optional(value: TokenStream, inner: TokenStream) -> TokenStream {
        quote! { #value.map(|value| #inner) }
    }

    fn sequence(value: TokenStream, element: TokenStream) -> TokenStream {
        quote! {
            #value
                .into_iter()
                .map(|value| #element)
                .collect::<Vec<_>>()
        }
    }

    fn tuple_field(value: TokenStream, index: Index) -> TokenStream {
        quote! { (#value).#index }
    }

    fn map(value: TokenStream, key: TokenStream, item: TokenStream) -> TokenStream {
        quote! {
            #value
                .into_iter()
                .map(|(key, value)| (#key, #item))
                .collect::<Vec<_>>()
        }
    }
}

impl OutgoingMode for BorrowedValue {
    fn custom(converter: TokenStream) -> OutgoingTransform {
        OutgoingTransform::ValueConverter(converter)
    }

    fn optional(value: TokenStream, inner: TokenStream) -> TokenStream {
        quote! { #value.as_ref().map(|value| #inner) }
    }

    fn sequence(value: TokenStream, element: TokenStream) -> TokenStream {
        quote! {
            #value
                .iter()
                .map(|value| #element)
                .collect::<Vec<_>>()
        }
    }

    fn tuple_field(value: TokenStream, index: Index) -> TokenStream {
        quote! { &(#value).#index }
    }

    fn map(value: TokenStream, key: TokenStream, item: TokenStream) -> TokenStream {
        quote! {
            #value
                .iter()
                .map(|(key, value)| (#key, #item))
                .collect::<Vec<_>>()
        }
    }
}

impl<'expansion, 'lowered, S: RenderSurface> Incoming<'expansion, 'lowered, S> {
    pub const fn new(
        codec: &'lowered CodecNode,
        expansion: &'expansion Expansion<'lowered, S>,
    ) -> Self {
        Self { codec, expansion }
    }

    pub fn convert(&self, value: TokenStream) -> Result<IncomingConversion, Error> {
        let transform = self
            .codec
            .render_read_with(&mut IncomingConverter::new(self.expansion))?;
        Ok(transform.apply(value))
    }

    pub fn decoded_type(&self) -> Result<Option<TokenStream>, Error> {
        match self.codec.contains_custom() {
            true => self
                .codec
                .render_read_with(&mut RepresentationType)
                .map(Some),
            false => Ok(None),
        }
    }
}

impl<S: RenderSurface> CodecRead for IncomingConverter<'_, '_, S> {
    type Expr = Result<IncomingTransform, Error>;

    fn primitive(&mut self, _: Primitive) -> Self::Expr {
        Ok(IncomingTransform::identity())
    }

    fn string(&mut self) -> Self::Expr {
        Ok(IncomingTransform::identity())
    }

    fn bytes(&mut self) -> Self::Expr {
        Ok(IncomingTransform::identity())
    }

    fn direct_record(&mut self, _: RecordId) -> Self::Expr {
        Ok(IncomingTransform::identity())
    }

    fn encoded_record(&mut self, _: RecordId) -> Self::Expr {
        Ok(IncomingTransform::identity())
    }

    fn c_style_enum(&mut self, _: EnumId) -> Self::Expr {
        Ok(IncomingTransform::identity())
    }

    fn data_enum(&mut self, _: EnumId) -> Self::Expr {
        Ok(IncomingTransform::identity())
    }

    fn class_handle(&mut self, _: ClassId) -> Self::Expr {
        Ok(IncomingTransform::identity())
    }

    fn callback_handle(&mut self, _: CallbackId) -> Self::Expr {
        Ok(IncomingTransform::identity())
    }

    fn custom(&mut self, id: CustomTypeId, representation: Self::Expr) -> Self::Expr {
        let representation = representation?;
        let custom = self.expansion.custom_type(id)?;
        let converter = ConverterRenderer::new(custom.converters().try_from_ffi()).render()?;
        if !representation.changed {
            return Ok(IncomingTransform::new(converter, true, true));
        }
        let representation_value = representation.value_or_return_error(quote! { __boltffi_value });
        Ok(IncomingTransform::new(
            quote! {
                |__boltffi_value| {
                    let __boltffi_representation = #representation_value;
                    (#converter)(__boltffi_representation)
                }
            },
            true,
            true,
        ))
    }

    fn builtin(&mut self, _: BuiltinType) -> Self::Expr {
        Ok(IncomingTransform::identity())
    }

    fn optional(&mut self, inner: Self::Expr) -> Self::Expr {
        let inner = inner?;
        match inner.changed {
            true => {
                let inner = inner.apply(quote! { value });
                let inner_tokens = inner.tokens();
                Ok(match inner.fallible {
                    true => IncomingTransform::new(
                        quote! {
                            |__boltffi_value| {
                                match __boltffi_value {
                                    Some(value) => #inner_tokens.map(Some),
                                    None => Ok(None),
                                }
                            }
                        },
                        true,
                        true,
                    ),
                    false => IncomingTransform::new(
                        quote! {
                            |__boltffi_value| {
                                __boltffi_value.map(|value| #inner_tokens)
                            }
                        },
                        false,
                        true,
                    ),
                })
            }
            false => Ok(IncomingTransform::identity()),
        }
    }

    fn sequence(&mut self, _: &Op<ElementCount>, element: Self::Expr) -> Self::Expr {
        let element = element?;
        match element.changed {
            true => {
                let element = element.apply(quote! { value });
                let element_tokens = element.tokens();
                Ok(match element.fallible {
                    true => IncomingTransform::new(
                        quote! {
                            |__boltffi_value| {
                                __boltffi_value
                                    .into_iter()
                                    .map(|value| #element_tokens)
                                    .collect::<Result<Vec<_>, _>>()
                            }
                        },
                        true,
                        true,
                    ),
                    false => IncomingTransform::new(
                        quote! {
                            |__boltffi_value| {
                                __boltffi_value
                                    .into_iter()
                                    .map(|value| #element_tokens)
                                    .collect::<Vec<_>>()
                            }
                        },
                        false,
                        true,
                    ),
                })
            }
            false => Ok(IncomingTransform::identity()),
        }
    }

    fn tuple(&mut self, elements: Vec<Self::Expr>) -> Self::Expr {
        let elements = elements.into_iter().collect::<Result<Vec<_>, _>>()?;
        let changed = elements.iter().any(|element| element.changed);
        let fallible = elements.iter().any(|element| element.fallible);
        match changed {
            true => {
                let values = elements
                    .iter()
                    .enumerate()
                    .map(|(index, element)| {
                        let index = Index::from(index);
                        element.value_or_return_error(quote! { (__boltffi_value).#index })
                    })
                    .collect::<Vec<_>>();
                Ok(match fallible {
                    true => IncomingTransform::new(
                        quote! {
                            |__boltffi_value| {
                                Ok((#(#values,)*))
                            }
                        },
                        true,
                        true,
                    ),
                    false => IncomingTransform::new(
                        quote! {
                            |__boltffi_value| {
                                (#(#values,)*)
                            }
                        },
                        false,
                        true,
                    ),
                })
            }
            false => Ok(IncomingTransform::identity()),
        }
    }

    fn result(&mut self, ok: Self::Expr, err: Self::Expr) -> Self::Expr {
        let ok = ok?;
        let err = err?;
        let changed = ok.changed || err.changed;
        let fallible = ok.fallible || err.fallible;
        match changed {
            true => {
                let ok = ok.apply(quote! { value });
                let err = err.apply(quote! { error });
                let ok_tokens = ok.tokens();
                let err_tokens = err.tokens();
                let ok_arm = match ok.fallible {
                    true => quote! { #ok_tokens.map(Ok) },
                    false => quote! { Ok(#ok_tokens) },
                };
                let err_arm = match err.fallible {
                    true => quote! { #err_tokens.map(Err) },
                    false => quote! { Err(#err_tokens) },
                };
                let tokens = match fallible {
                    true => {
                        let ok_arm = match ok.fallible {
                            true => ok_arm,
                            false => quote! { Ok(#ok_arm) },
                        };
                        let err_arm = match err.fallible {
                            true => err_arm,
                            false => quote! { Ok(#err_arm) },
                        };
                        quote! {
                            |__boltffi_value| {
                                match __boltffi_value {
                                    Ok(value) => #ok_arm,
                                    Err(error) => #err_arm,
                                }
                            }
                        }
                    }
                    false => quote! {
                        |__boltffi_value| {
                            match __boltffi_value {
                                Ok(value) => #ok_arm,
                                Err(error) => #err_arm,
                            }
                        }
                    },
                };
                Ok(IncomingTransform::new(tokens, fallible, true))
            }
            false => Ok(IncomingTransform::identity()),
        }
    }

    fn map(&mut self, _: MapKind, key: Self::Expr, value: Self::Expr) -> Self::Expr {
        let key = key?;
        let value = value?;
        let changed = key.changed || value.changed;
        let fallible = key.fallible || value.fallible;
        match changed {
            true => {
                let key = key.value_or_return_error(quote! { key });
                let value = value.value_or_return_error(quote! { value });
                Ok(match fallible {
                    true => IncomingTransform::new(
                        quote! {
                            |__boltffi_value| {
                                __boltffi_value
                                    .into_iter()
                                    .map(|(key, value)| {
                                        Ok((#key, #value))
                                    })
                                    .collect::<Result<_, _>>()
                            }
                        },
                        true,
                        true,
                    ),
                    false => IncomingTransform::new(
                        quote! {
                            |__boltffi_value| {
                                __boltffi_value
                                    .into_iter()
                                    .map(|(key, value)| (#key, #value))
                                    .collect::<_>()
                            }
                        },
                        false,
                        true,
                    ),
                })
            }
            false => Ok(IncomingTransform::identity()),
        }
    }
}

impl<S, M> CodecRead for OutgoingConverter<'_, '_, S, M>
where
    S: RenderSurface,
    M: OutgoingMode,
{
    type Expr = Result<OutgoingTransform, Error>;

    fn primitive(&mut self, _: Primitive) -> Self::Expr {
        Ok(OutgoingTransform::Identity)
    }

    fn string(&mut self) -> Self::Expr {
        Ok(OutgoingTransform::Identity)
    }

    fn bytes(&mut self) -> Self::Expr {
        Ok(OutgoingTransform::Identity)
    }

    fn direct_record(&mut self, _: RecordId) -> Self::Expr {
        Ok(OutgoingTransform::Identity)
    }

    fn encoded_record(&mut self, _: RecordId) -> Self::Expr {
        Ok(OutgoingTransform::Identity)
    }

    fn c_style_enum(&mut self, _: EnumId) -> Self::Expr {
        Ok(OutgoingTransform::Identity)
    }

    fn data_enum(&mut self, _: EnumId) -> Self::Expr {
        Ok(OutgoingTransform::Identity)
    }

    fn class_handle(&mut self, _: ClassId) -> Self::Expr {
        Ok(OutgoingTransform::Identity)
    }

    fn callback_handle(&mut self, _: CallbackId) -> Self::Expr {
        Ok(OutgoingTransform::Identity)
    }

    fn custom(&mut self, id: CustomTypeId, representation: Self::Expr) -> Self::Expr {
        let representation = representation?;
        let custom = self.expansion.custom_type(id)?;
        let converter = ConverterRenderer::new(custom.converters().into_ffi()).render()?;
        let custom = M::custom(converter);
        match representation.changed() {
            true => Ok(OutgoingTransform::CustomRepresentation {
                custom: Box::new(custom),
                representation: Box::new(representation),
            }),
            false => Ok(custom),
        }
    }

    fn builtin(&mut self, _: BuiltinType) -> Self::Expr {
        Ok(OutgoingTransform::Identity)
    }

    fn optional(&mut self, inner: Self::Expr) -> Self::Expr {
        let inner = inner?;
        match inner.changed() {
            true => Ok(OutgoingTransform::Optional(Box::new(inner))),
            false => Ok(OutgoingTransform::Identity),
        }
    }

    fn sequence(&mut self, _: &Op<ElementCount>, element: Self::Expr) -> Self::Expr {
        let element = element?;
        match element.changed() {
            true => Ok(OutgoingTransform::Sequence(Box::new(element))),
            false => Ok(OutgoingTransform::Identity),
        }
    }

    fn tuple(&mut self, elements: Vec<Self::Expr>) -> Self::Expr {
        let elements = elements.into_iter().collect::<Result<Vec<_>, _>>()?;
        match elements.iter().any(OutgoingTransform::changed) {
            true => Ok(OutgoingTransform::Tuple(elements)),
            false => Ok(OutgoingTransform::Identity),
        }
    }

    fn result(&mut self, ok: Self::Expr, err: Self::Expr) -> Self::Expr {
        let ok = ok?;
        let err = err?;
        match ok.changed() || err.changed() {
            true => Ok(OutgoingTransform::Result {
                ok: Box::new(ok),
                err: Box::new(err),
            }),
            false => Ok(OutgoingTransform::Identity),
        }
    }

    fn map(&mut self, _: MapKind, key: Self::Expr, value: Self::Expr) -> Self::Expr {
        let key = key?;
        let value = value?;
        match key.changed() || value.changed() {
            true => Ok(OutgoingTransform::Map {
                key: Box::new(key),
                value: Box::new(value),
            }),
            false => Ok(OutgoingTransform::Identity),
        }
    }
}

impl<'expansion, 'lowered, S: RenderSurface> Outgoing<'expansion, 'lowered, S> {
    pub const fn new(
        codec: &'lowered CodecNode,
        expansion: &'expansion Expansion<'lowered, S>,
    ) -> Self {
        Self { codec, expansion }
    }

    pub fn has_custom_conversion(&self) -> bool {
        self.codec.contains_custom()
    }

    pub fn convert(&self, value: TokenStream) -> Result<TokenStream, Error> {
        let transform = self
            .codec
            .render_read_with(&mut OutgoingConverter::<S, OwnedValue>::new(self.expansion))?;
        Ok(transform.apply::<OwnedValue>(value))
    }
}

impl<'expansion, 'lowered, S: RenderSurface> BorrowedOutgoing<'expansion, 'lowered, S> {
    pub const fn new(
        codec: &'lowered CodecNode,
        expansion: &'expansion Expansion<'lowered, S>,
    ) -> Self {
        Self { codec, expansion }
    }

    pub fn has_custom_conversion(&self) -> bool {
        self.codec.contains_custom()
    }

    pub fn convert(&self, value: TokenStream) -> Result<TokenStream, Error> {
        let transform =
            self.codec
                .render_read_with(&mut OutgoingConverter::<S, BorrowedValue>::new(
                    self.expansion,
                ))?;
        Ok(transform.apply::<BorrowedValue>(value))
    }
}

impl<'lowered> ConverterRenderer<'lowered> {
    const fn new(converter: &'lowered CustomTypeConverter) -> Self {
        Self { converter }
    }

    fn render(self) -> Result<TokenStream, Error> {
        match self.converter {
            CustomTypeConverter::Path(path) => PathRenderer::new(path).render(),
            CustomTypeConverter::TraitMethod(converter) => {
                let receiver = PathRenderer::new(converter.receiver()).render_type()?;
                let method =
                    parse_str::<syn::Ident>(converter.method().as_str()).map_err(|_| {
                        Error::SourceSyntaxMismatch("custom converter method is not Rust syntax")
                    })?;
                Ok(quote! {
                    <#receiver as ::boltffi::CustomFfiConvertible>::#method
                })
            }
            CustomTypeConverter::Expression(expression) => parse_str::<Expr>(expression.source())
                .map(|expression| quote! { #expression })
                .map_err(|_| Error::SourceSyntaxMismatch("custom converter is not Rust syntax")),
            _ => Err(Error::UnsupportedExpansion("unknown custom converter")),
        }
    }
}

impl<'lowered> PathRenderer<'lowered> {
    const fn new(path: &'lowered CustomConverterPath) -> Self {
        Self { path }
    }

    fn render(self) -> Result<TokenStream, Error> {
        parse_str::<ExprPath>(&self.source()?)
            .map(|path| quote! { #path })
            .map_err(|_| Error::SourceSyntaxMismatch("custom converter path is not Rust syntax"))
    }

    fn render_type(self) -> Result<syn::Type, Error> {
        parse_str::<syn::Type>(&self.source()?).map_err(|_| {
            Error::SourceSyntaxMismatch("custom converter receiver is not Rust syntax")
        })
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

impl CodecRead for RepresentationType {
    type Expr = Result<TokenStream, Error>;

    fn primitive(&mut self, primitive: Primitive) -> Self::Expr {
        wrapper::type_ref::Renderer.primitive(primitive)
    }

    fn string(&mut self) -> Self::Expr {
        Ok(quote! { String })
    }

    fn bytes(&mut self) -> Self::Expr {
        Ok(quote! { Vec<u8> })
    }

    fn direct_record(&mut self, _: RecordId) -> Self::Expr {
        Err(Error::UnsupportedExpansion(
            "direct record representation type",
        ))
    }

    fn encoded_record(&mut self, _: RecordId) -> Self::Expr {
        Err(Error::UnsupportedExpansion(
            "encoded record representation type",
        ))
    }

    fn c_style_enum(&mut self, _: EnumId) -> Self::Expr {
        Err(Error::UnsupportedExpansion(
            "c-style enum representation type",
        ))
    }

    fn data_enum(&mut self, _: EnumId) -> Self::Expr {
        Err(Error::UnsupportedExpansion("data enum representation type"))
    }

    fn class_handle(&mut self, _: ClassId) -> Self::Expr {
        Err(Error::UnsupportedExpansion(
            "class handle representation type",
        ))
    }

    fn callback_handle(&mut self, _: CallbackId) -> Self::Expr {
        Err(Error::UnsupportedExpansion(
            "callback handle representation type",
        ))
    }

    fn custom(&mut self, _: CustomTypeId, representation: Self::Expr) -> Self::Expr {
        representation
    }

    fn builtin(&mut self, kind: BuiltinType) -> Self::Expr {
        wrapper::type_ref::Renderer.builtin(kind)
    }

    fn optional(&mut self, inner: Self::Expr) -> Self::Expr {
        let inner = inner?;
        Ok(quote! { Option<#inner> })
    }

    fn sequence(&mut self, _: &Op<ElementCount>, element: Self::Expr) -> Self::Expr {
        let element = element?;
        Ok(quote! { Vec<#element> })
    }

    fn tuple(&mut self, elements: Vec<Self::Expr>) -> Self::Expr {
        let elements = elements.into_iter().collect::<Result<Vec<_>, _>>()?;
        Ok(quote! { (#(#elements,)*) })
    }

    fn result(&mut self, ok: Self::Expr, err: Self::Expr) -> Self::Expr {
        let ok = ok?;
        let err = err?;
        Ok(quote! { Result<#ok, #err> })
    }

    fn map(&mut self, kind: MapKind, key: Self::Expr, value: Self::Expr) -> Self::Expr {
        let key = key?;
        let value = value?;
        match kind {
            MapKind::Hash => Ok(quote! { ::std::collections::HashMap<#key, #value> }),
            MapKind::BTree => Ok(quote! { ::std::collections::BTreeMap<#key, #value> }),
        }
    }
}
