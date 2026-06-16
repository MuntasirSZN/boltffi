use crate::core::Result;

use super::{
    contract::{Function, Parameter, Type},
    identifier::Identifier,
};

pub struct TypeSyntax<'ty> {
    ty: &'ty Type,
}

pub struct FunctionSyntax<'function> {
    function: &'function Function,
}

struct ParameterSyntax<'parameter> {
    parameter: &'parameter Parameter,
}

impl<'ty> TypeSyntax<'ty> {
    pub fn new(ty: &'ty Type) -> Self {
        Self { ty }
    }

    pub fn anonymous(&self) -> Result<String> {
        Ok(match self.ty {
            Type::Void => "void".to_owned(),
            Type::Bool => "bool".to_owned(),
            Type::Int8 => "int8_t".to_owned(),
            Type::Uint8 => "uint8_t".to_owned(),
            Type::Int16 => "int16_t".to_owned(),
            Type::Uint16 => "uint16_t".to_owned(),
            Type::Int32 => "int32_t".to_owned(),
            Type::Uint32 => "uint32_t".to_owned(),
            Type::Int64 => "int64_t".to_owned(),
            Type::Uint64 => "uint64_t".to_owned(),
            Type::Float32 => "float".to_owned(),
            Type::Float64 => "double".to_owned(),
            Type::SignedPointerWidth => "intptr_t".to_owned(),
            Type::PointerWidth => "uintptr_t".to_owned(),
            Type::Status => "FfiStatus".to_owned(),
            Type::Buffer => "FfiBuf_u8".to_owned(),
            Type::String => "FfiString".to_owned(),
            Type::Span => "FfiSpan".to_owned(),
            Type::FutureHandle => "RustFutureHandle".to_owned(),
            Type::StreamPollResult => "StreamPollResult".to_owned(),
            Type::WaitResult => "WaitResult".to_owned(),
            Type::CallbackHandle => "BoltFFICallbackHandle".to_owned(),
            Type::Named(name) => Identifier::parse(name)?.to_string(),
            Type::ConstPointer(inner) => format!("const {} *", Self::new(inner).anonymous()?),
            Type::MutPointer(inner) => format!("{} *", Self::new(inner).anonymous()?),
            Type::FunctionPointer { returns, params } => {
                Self::function_pointer_declaration("", returns, params.iter())?
                    .trim()
                    .to_owned()
            }
        })
    }

    pub fn declaration(&self, name: &str) -> Result<String> {
        let name = Identifier::escape(name)?;
        Ok(match self.ty {
            Type::FunctionPointer { returns, params } => {
                Self::function_pointer_declaration(name.as_str(), returns, params.iter())?
            }
            Type::ConstPointer(inner) => {
                format!("const {} *{}", Self::new(inner).anonymous()?, name)
            }
            Type::MutPointer(inner) => format!("{} *{}", Self::new(inner).anonymous()?, name),
            _ => format!("{} {}", self.anonymous()?, name),
        })
    }

    pub fn function(&self, name: &str, params: &str) -> Result<String> {
        Ok(format!("{} {name}({params})", self.anonymous()?))
    }
}

impl TypeSyntax<'_> {
    pub fn function_pointer_declaration<'params>(
        name: &str,
        returns: &Type,
        params: impl IntoIterator<Item = &'params Type>,
    ) -> Result<String> {
        let params = params
            .into_iter()
            .map(|ty| TypeSyntax { ty }.anonymous())
            .collect::<Result<Vec<_>>>()?;
        let params = match params.is_empty() {
            true => "void".to_owned(),
            false => params.join(", "),
        };
        Ok(format!(
            "{} (*{name})({params})",
            TypeSyntax { ty: returns }.anonymous()?
        ))
    }
}

impl<'function> FunctionSyntax<'function> {
    pub fn new(function: &'function Function) -> Self {
        Self { function }
    }

    pub fn declaration(&self) -> Result<String> {
        let name = Identifier::parse(self.function.name())?;
        TypeSyntax::new(self.function.returns()).function(name.as_str(), &self.named_params()?)
    }

    pub fn pointer_typedef(&self, name: &str) -> Result<String> {
        let name = Identifier::parse(name)?;
        Ok(format!(
            "typedef {}",
            TypeSyntax::function_pointer_declaration(
                name.as_str(),
                self.function.returns(),
                self.function.params().iter().map(Parameter::ty)
            )?
        ))
    }

    fn named_params(&self) -> Result<String> {
        match self.function.params().is_empty() {
            true => Ok("void".to_owned()),
            false => self
                .function
                .params()
                .iter()
                .map(ParameterSyntax::new)
                .map(|parameter| parameter.declaration())
                .collect::<Result<Vec<_>>>()
                .map(|params| params.join(", ")),
        }
    }
}

impl<'parameter> ParameterSyntax<'parameter> {
    fn new(parameter: &'parameter Parameter) -> Self {
        Self { parameter }
    }

    fn declaration(&self) -> Result<String> {
        TypeSyntax::new(self.parameter.ty()).declaration(self.parameter.name())
    }
}
