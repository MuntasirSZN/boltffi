use boltffi_ast::{EnumDef, MethodDef, Path as SourcePath, TypeExpr};
use boltffi_binding::{
    ExportedCallable, ExportedMethodDecl, InitializerDecl, NativeSymbol, Receive, TypeRef,
    WritePlan,
};
use proc_macro2::TokenStream;
use quote::quote;
use syn::{Ident, Type, parse_str};

use crate::experimental::{
    error::Error,
    expansion::Expansion,
    rust_api,
    target::Target,
    wrapper::{self, Render, export, names},
};

pub struct Renderer<'expansion, 'lowered, S: Target> {
    source: &'lowered EnumDef,
    enumeration: Ident,
    receiver: Receiver<'lowered>,
    initializers: &'lowered [InitializerDecl<S>],
    methods: &'lowered [ExportedMethodDecl<S, NativeSymbol>],
    expansion: &'expansion Expansion<'lowered, S>,
}

struct EnumExport<'expansion, 'lowered, S: Target> {
    source: &'lowered EnumDef,
    enumeration: Ident,
    receiver: Receiver<'lowered>,
    source_method: &'lowered MethodDef,
    symbol: &'lowered NativeSymbol,
    callable: &'lowered ExportedCallable<S>,
    expansion: &'expansion Expansion<'lowered, S>,
}

#[derive(Clone)]
pub enum Receiver<'lowered> {
    Direct { ty: TypeRef },
    Encoded { codec: &'lowered WritePlan },
}

impl<'expansion, 'lowered, S: Target> Renderer<'expansion, 'lowered, S> {
    pub fn new(
        source: &'lowered EnumDef,
        enumeration: Ident,
        receiver: Receiver<'lowered>,
        initializers: &'lowered [InitializerDecl<S>],
        methods: &'lowered [ExportedMethodDecl<S, NativeSymbol>],
        expansion: &'expansion Expansion<'lowered, S>,
    ) -> Self {
        Self {
            source,
            enumeration,
            receiver,
            initializers,
            methods,
            expansion,
        }
    }

    pub fn render(self) -> Result<TokenStream, Error>
    where
        S: Target,
        wrapper::arguments::SyncRenderer: Render<
                S,
                wrapper::arguments::Input<'expansion, 'lowered, S>,
                Output = wrapper::arguments::Tokens,
            >,
        wrapper::returns::Failure: Render<S, wrapper::returns::FailureInput<'expansion, 'lowered, S>, Output = TokenStream>,
        wrapper::returns::Renderer: Render<
                S,
                wrapper::returns::Input<'expansion, 'lowered, S>,
                Output = wrapper::returns::Tokens,
            >,
        wrapper::async_call::Renderer:
            Render<S, wrapper::async_call::Input<'expansion, 'lowered, S>, Output = TokenStream>,
        for<'ty> wrapper::param::direct::Renderer:
            Render<S, wrapper::param::direct::Input<'ty>, Output = wrapper::param::Tokens>,
        wrapper::param::encoded::Renderer: Render<
                S,
                wrapper::param::encoded::Input<'expansion, 'lowered, S>,
                Output = wrapper::param::Tokens,
            >,
    {
        let declarations = rust_api::MethodDeclarations::enumeration(self.source);
        let initializers = self
            .initializers
            .iter()
            .map(|initializer| {
                EnumExport {
                    source: self.source,
                    enumeration: self.enumeration.clone(),
                    receiver: self.receiver.clone(),
                    source_method: declarations.resolve(initializer.name())?,
                    symbol: initializer.symbol(),
                    callable: initializer.callable(),
                    expansion: self.expansion,
                }
                .render()
            })
            .collect::<Result<Vec<_>, _>>()?;
        let methods = self
            .methods
            .iter()
            .map(|method| {
                EnumExport {
                    source: self.source,
                    enumeration: self.enumeration.clone(),
                    receiver: self.receiver.clone(),
                    source_method: declarations.resolve(method.name())?,
                    symbol: method.target(),
                    callable: method.callable(),
                    expansion: self.expansion,
                }
                .render()
            })
            .collect::<Result<Vec<_>, _>>()?;
        Ok(quote! {
            #(#initializers)*
            #(#methods)*
        })
    }
}

impl<'expansion, 'lowered, S> EnumExport<'expansion, 'lowered, S>
where
    S: Target,
    wrapper::arguments::SyncRenderer: Render<
            S,
            wrapper::arguments::Input<'expansion, 'lowered, S>,
            Output = wrapper::arguments::Tokens,
        >,
    wrapper::returns::Failure:
        Render<S, wrapper::returns::FailureInput<'expansion, 'lowered, S>, Output = TokenStream>,
    wrapper::returns::Renderer: Render<
            S,
            wrapper::returns::Input<'expansion, 'lowered, S>,
            Output = wrapper::returns::Tokens,
        >,
    wrapper::async_call::Renderer:
        Render<S, wrapper::async_call::Input<'expansion, 'lowered, S>, Output = TokenStream>,
    for<'ty> wrapper::param::direct::Renderer:
        Render<S, wrapper::param::direct::Input<'ty>, Output = wrapper::param::Tokens>,
    wrapper::param::encoded::Renderer: Render<
            S,
            wrapper::param::encoded::Input<'expansion, 'lowered, S>,
            Output = wrapper::param::Tokens,
        >,
{
    fn render(self) -> Result<TokenStream, Error> {
        let method = method_ident(self.source_method)?;
        let (receiver, rust_call) = self.receiver(method.clone())?;
        let source_signature = rust_api::Callable::enum_method(self.source_method, self.source);
        if matches!(
            self.callable.execution(),
            boltffi_binding::ExecutionDecl::Asynchronous(_)
        ) {
            return <wrapper::async_call::Renderer as Render<S, _>>::render(
                wrapper::async_call::Renderer,
                wrapper::async_call::Input::exported(
                    self.symbol,
                    self.callable,
                    source_signature,
                    rust_call,
                    receiver,
                    rust_api::VisibilityTokens::new(&self.source_method.source.visibility)
                        .into_tokens()?,
                    self.expansion,
                ),
            );
        }
        export::Renderer::new(
            self.symbol,
            self.callable,
            source_signature,
            rust_call,
            receiver,
            rust_api::VisibilityTokens::new(&self.source_method.source.visibility).into_tokens()?,
            self.expansion,
        )
        .render()
    }

    fn receiver(&self, method: Ident) -> Result<(export::ReceiverTokens, export::RustCall), Error> {
        match self.callable.receiver() {
            None => {
                let enumeration = &self.enumeration;
                Ok((
                    export::ReceiverTokens::none(),
                    export::RustCall::associated(quote! { #enumeration }, method),
                ))
            }
            Some(receive) => self.receiver.clone().render(
                self.source,
                self.callable,
                receive,
                method,
                self.expansion,
            ),
        }
    }
}

impl<'lowered> Receiver<'lowered> {
    fn render<'expansion, S>(
        self,
        source: &'lowered EnumDef,
        callable: &'lowered ExportedCallable<S>,
        receive: Receive,
        method: Ident,
        expansion: &'expansion Expansion<'lowered, S>,
    ) -> Result<(export::ReceiverTokens, export::RustCall), Error>
    where
        S: Target,
        wrapper::returns::Failure: Render<S, wrapper::returns::FailureInput<'expansion, 'lowered, S>, Output = TokenStream>,
        for<'ty> wrapper::param::direct::Renderer:
            Render<S, wrapper::param::direct::Input<'ty>, Output = wrapper::param::Tokens>,
        wrapper::param::encoded::Renderer: Render<
                S,
                wrapper::param::encoded::Input<'expansion, 'lowered, S>,
                Output = wrapper::param::Tokens,
            >,
    {
        match self {
            Self::Direct { ty } => Self::render_direct::<S>(source, &ty, receive, method),
            Self::Encoded { codec } => {
                Self::render_encoded::<S>(source, callable, codec, receive, method, expansion)
            }
        }
    }

    fn render_direct<S>(
        source: &EnumDef,
        ty: &TypeRef,
        receive: Receive,
        method: Ident,
    ) -> Result<(export::ReceiverTokens, export::RustCall), Error>
    where
        S: Target,
        for<'ty> wrapper::param::direct::Renderer:
            Render<S, wrapper::param::direct::Input<'ty>, Output = wrapper::param::Tokens>,
    {
        if receive == Receive::ByMutRef {
            return Err(Error::UnsupportedExpansion(
                "mutable enum receiver without writeback",
            ));
        }
        let rust_type = enum_type(source)?;
        let receiver = names::Wrapper::new(method.span()).receiver();
        let tokens = <wrapper::param::direct::Renderer as Render<S, _>>::render(
            wrapper::param::direct::Renderer,
            wrapper::param::direct::Input::new(
                ty,
                receive,
                rust_type,
                receiver.clone(),
                TokenStream::new(),
            ),
        )?;
        Ok((
            export::ReceiverTokens::new(
                tokens.ffi_parameters().to_vec(),
                tokens.conversions().to_vec(),
                tokens.writebacks().to_vec(),
                false,
            ),
            export::RustCall::method(receiver, method),
        ))
    }

    fn render_encoded<'expansion, S>(
        source: &'lowered EnumDef,
        callable: &'lowered ExportedCallable<S>,
        codec: &'lowered WritePlan,
        receive: Receive,
        method: Ident,
        expansion: &'expansion Expansion<'lowered, S>,
    ) -> Result<(export::ReceiverTokens, export::RustCall), Error>
    where
        S: Target,
        wrapper::returns::Failure: Render<S, wrapper::returns::FailureInput<'expansion, 'lowered, S>, Output = TokenStream>,
        wrapper::param::encoded::Renderer: Render<
                S,
                wrapper::param::encoded::Input<'expansion, 'lowered, S>,
                Output = wrapper::param::Tokens,
            >,
    {
        if receive == Receive::ByMutRef {
            return Err(Error::UnsupportedExpansion(
                "mutable encoded enum receiver without writeback",
            ));
        }
        let receiver = names::Wrapper::new(method.span()).receiver();
        let source_type = TypeExpr::enumeration(
            source.id.clone(),
            SourcePath::single(source.name.spelling()),
        );
        let failure = <wrapper::returns::Failure as Render<S, _>>::render(
            wrapper::returns::Failure,
            wrapper::returns::FailureInput::new(callable.returns(), callable.error(), expansion),
        )?;
        let tokens = <wrapper::param::encoded::Renderer as Render<S, _>>::render(
            wrapper::param::encoded::Renderer,
            wrapper::param::encoded::Input::new(
                codec,
                <S as boltffi_binding::SurfaceLower>::encoded_param_shape(),
                rust_api::DecodeTarget::received(receive, &source_type)?,
                receiver.clone(),
                failure,
                expansion,
            ),
        )?;
        Ok((
            export::ReceiverTokens::new(
                tokens.ffi_parameters().to_vec(),
                tokens.conversions().to_vec(),
                tokens.writebacks().to_vec(),
                true,
            ),
            export::RustCall::method(receiver, method),
        ))
    }
}

fn enum_type(source: &EnumDef) -> Result<Type, Error> {
    parse_str(source.name.spelling())
        .map_err(|_| Error::SourceSyntaxMismatch("source enum name is not a Rust type"))
}

fn method_ident(source: &MethodDef) -> Result<Ident, Error> {
    parse_str(source.name.spelling())
        .map_err(|_| Error::SourceSyntaxMismatch("source method name is not a Rust identifier"))
}
