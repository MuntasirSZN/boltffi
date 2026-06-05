use boltffi_binding::{Native, Wasm32, native, wasm32};
use proc_macro2::TokenStream;
use quote::quote;

use crate::experimental::{error::Error, render::Rule as RenderRule, target::Target};

pub struct Rule;

pub struct Input<S: Target> {
    shape: S::BufferShape,
    value: syn::Ident,
}

impl<S: Target> Input<S> {
    pub fn new(shape: S::BufferShape, value: syn::Ident) -> Self {
        Self { shape, value }
    }
}

pub struct Tokens {
    value_type: TokenStream,
    return_type: TokenStream,
    value: TokenStream,
}

impl Tokens {
    pub fn return_type(&self) -> &TokenStream {
        &self.return_type
    }

    pub fn value(&self) -> &TokenStream {
        &self.value
    }

    pub fn return_type_without_arrow(&self) -> TokenStream {
        self.value_type.clone()
    }
}

pub struct Empty<S: Target> {
    shape: S::BufferShape,
}

impl<S: Target> Empty<S> {
    pub fn new(shape: S::BufferShape) -> Self {
        Self { shape }
    }
}

impl RenderRule<Native, Input<Native>> for Rule {
    type Output = Tokens;

    fn apply(self, input: Input<Native>) -> Result<Self::Output, Error> {
        let value = input.value;
        match input.shape {
            native::BufferShape::Buffer => Ok(Tokens {
                value_type: quote! { ::boltffi::__private::FfiBuf },
                return_type: quote! { -> ::boltffi::__private::FfiBuf },
                value: quote! { ::boltffi::__private::FfiBuf::wire_encode(&#value) },
            }),
            native::BufferShape::Slice | native::BufferShape::BufferPointer => {
                Err(Error::UnsupportedExpansion("native encoded return shape"))
            }
            _ => Err(Error::UnsupportedExpansion(
                "unknown native encoded return shape",
            )),
        }
    }
}

impl RenderRule<Native, Empty<Native>> for Rule {
    type Output = Tokens;

    fn apply(self, input: Empty<Native>) -> Result<Self::Output, Error> {
        match input.shape {
            native::BufferShape::Buffer => Ok(Tokens {
                value_type: quote! { ::boltffi::__private::FfiBuf },
                return_type: quote! { -> ::boltffi::__private::FfiBuf },
                value: quote! { ::boltffi::__private::FfiBuf::default() },
            }),
            native::BufferShape::Slice | native::BufferShape::BufferPointer => {
                Err(Error::UnsupportedExpansion("native encoded return shape"))
            }
            _ => Err(Error::UnsupportedExpansion(
                "unknown native encoded return shape",
            )),
        }
    }
}

impl RenderRule<Wasm32, Input<Wasm32>> for Rule {
    type Output = Tokens;

    fn apply(self, input: Input<Wasm32>) -> Result<Self::Output, Error> {
        let value = input.value;

        match input.shape {
            wasm32::BufferShape::Packed => Ok(Tokens {
                value_type: quote! { u64 },
                return_type: quote! { -> u64 },
                value: quote! { ::boltffi::__private::FfiBuf::wire_encode(&#value).into_packed() },
            }),
            wasm32::BufferShape::Slice => {
                Err(Error::UnsupportedExpansion("wasm encoded return shape"))
            }
            _ => Err(Error::UnsupportedExpansion(
                "unknown wasm encoded return shape",
            )),
        }
    }
}

impl RenderRule<Wasm32, Empty<Wasm32>> for Rule {
    type Output = Tokens;

    fn apply(self, input: Empty<Wasm32>) -> Result<Self::Output, Error> {
        match input.shape {
            wasm32::BufferShape::Packed => Ok(Tokens {
                value_type: quote! { u64 },
                return_type: quote! { -> u64 },
                value: quote! { ::boltffi::__private::FfiBuf::default().into_packed() },
            }),
            wasm32::BufferShape::Slice => {
                Err(Error::UnsupportedExpansion("wasm encoded return shape"))
            }
            _ => Err(Error::UnsupportedExpansion(
                "unknown wasm encoded return shape",
            )),
        }
    }
}
