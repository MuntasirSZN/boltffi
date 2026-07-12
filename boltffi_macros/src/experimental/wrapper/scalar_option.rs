use boltffi_binding::Primitive;
use proc_macro2::TokenStream;
use quote::quote;
use syn::Ident;

use crate::experimental::error::Error;

pub struct WasmScalar {
    primitive: Primitive,
    value: Ident,
}

impl WasmScalar {
    pub fn new(primitive: Primitive, value: Ident) -> Self {
        Self { primitive, value }
    }

    pub fn incoming(self) -> Result<TokenStream, Error> {
        let value = self.value;
        Ok(match self.primitive {
            Primitive::Bool => quote! { #value != 0.0 },
            Primitive::F64 => quote! { #value },
            Primitive::I8
            | Primitive::U8
            | Primitive::I16
            | Primitive::U16
            | Primitive::I32
            | Primitive::U32
            | Primitive::I64
            | Primitive::U64
            | Primitive::ISize
            | Primitive::USize
            | Primitive::F32 => quote! { #value as _ },
            _ => return Err(Error::UnsupportedExpansion("scalar option primitive")),
        })
    }

    pub fn outgoing(self) -> Result<TokenStream, Error> {
        let value = self.value;
        Ok(match self.primitive {
            Primitive::Bool => quote! {
                if #value { 1.0 } else { 0.0 }
            },
            Primitive::F64 => quote! { #value },
            Primitive::I8
            | Primitive::U8
            | Primitive::I16
            | Primitive::U16
            | Primitive::I32
            | Primitive::U32
            | Primitive::I64
            | Primitive::U64
            | Primitive::ISize
            | Primitive::USize
            | Primitive::F32 => quote! { #value as f64 },
            _ => return Err(Error::UnsupportedExpansion("scalar option primitive")),
        })
    }
}
