use askama::Template as AskamaTemplate;

use crate::core::Result;

use super::contract::{CBridgeContract, Callback, Enum, Field, Function, Parameter, Record, Type};
use super::identifier::Identifier;

#[derive(AskamaTemplate)]
#[template(path = "bridge/c/header.h", escape = "none")]
struct HeaderTemplate {
    records: Vec<RecordView>,
    enums: Vec<EnumView>,
    callback_vtables: Vec<RecordView>,
    callback_functions: Vec<FunctionView>,
    functions: Vec<FunctionView>,
}

struct RecordView {
    name: Identifier,
    fields: Vec<FieldView>,
}

struct FieldView {
    declaration: String,
}

struct EnumView {
    name: Identifier,
    repr: String,
    variants: Vec<EnumVariantView>,
}

struct EnumVariantView {
    name: Identifier,
    ty: Identifier,
    value: i128,
}

struct FunctionView {
    declaration: String,
}

pub struct Header<'abi> {
    abi: &'abi CBridgeContract,
}

impl<'abi> Header<'abi> {
    pub fn new(abi: &'abi CBridgeContract) -> Self {
        Self { abi }
    }

    pub fn render(self) -> Result<String> {
        Ok(HeaderTemplate {
            records: self
                .abi
                .records()
                .iter()
                .map(RecordView::from_record)
                .collect::<Result<_>>()?,
            enums: self
                .abi
                .enums()
                .iter()
                .map(EnumView::from_enum)
                .collect::<Result<_>>()?,
            callback_vtables: self
                .abi
                .callbacks()
                .iter()
                .map(Callback::vtable)
                .map(RecordView::from_record)
                .collect::<Result<_>>()?,
            callback_functions: self
                .abi
                .callbacks()
                .iter()
                .flat_map(|callback| [callback.register(), callback.create_handle()])
                .map(FunctionView::from_function)
                .collect::<Result<_>>()?,
            functions: self
                .abi
                .functions()
                .iter()
                .map(FunctionView::from_function)
                .collect::<Result<_>>()?,
        }
        .render()?)
    }
}

impl RecordView {
    fn from_record(record: &Record) -> Result<Self> {
        Ok(Self {
            name: Identifier::parse(record.name())?,
            fields: record
                .fields()
                .iter()
                .map(FieldView::from_field)
                .collect::<Result<_>>()?,
        })
    }
}

impl FieldView {
    fn from_field(field: &Field) -> Result<Self> {
        Ok(Self {
            declaration: CType(field.ty()).declaration(field.name())?,
        })
    }
}

impl EnumView {
    fn from_enum(enumeration: &Enum) -> Result<Self> {
        Ok(Self {
            name: Identifier::parse(enumeration.name())?,
            repr: CType(enumeration.repr()).anonymous()?,
            variants: enumeration
                .variants()
                .iter()
                .map(|variant| {
                    Ok(EnumVariantView {
                        name: Identifier::parse(variant.name())?,
                        ty: Identifier::parse(enumeration.name())?,
                        value: variant.value(),
                    })
                })
                .collect::<Result<_>>()?,
        })
    }
}

impl FunctionView {
    fn from_function(function: &Function) -> Result<Self> {
        let name = Identifier::parse(function.name())?;
        let params = match function.params().is_empty() {
            true => "void".to_owned(),
            false => function
                .params()
                .iter()
                .map(ParameterView::from_parameter)
                .collect::<Result<Vec<_>>>()?
                .into_iter()
                .map(|parameter| parameter.declaration)
                .collect::<Vec<_>>()
                .join(", "),
        };
        Ok(Self {
            declaration: CType(function.returns()).function(name.as_str(), &params)?,
        })
    }
}

struct ParameterView {
    declaration: String,
}

impl ParameterView {
    fn from_parameter(parameter: &Parameter) -> Result<Self> {
        Ok(Self {
            declaration: CType(parameter.ty()).declaration(parameter.name())?,
        })
    }
}

struct CType<'ty>(&'ty Type);

impl CType<'_> {
    fn anonymous(&self) -> Result<String> {
        Ok(match self.0 {
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
            Type::ConstPointer(inner) => format!("const {} *", CType(inner).anonymous()?),
            Type::MutPointer(inner) => format!("{} *", CType(inner).anonymous()?),
            Type::FunctionPointer { returns, params } => self
                .function_pointer("", returns, params)?
                .trim()
                .to_owned(),
        })
    }

    fn declaration(&self, name: &str) -> Result<String> {
        let name = Identifier::escape(name)?;
        Ok(match self.0 {
            Type::FunctionPointer { returns, params } => {
                self.function_pointer(name.as_str(), returns, params)?
            }
            Type::ConstPointer(inner) => {
                format!("const {} *{}", CType(inner).anonymous()?, name)
            }
            Type::MutPointer(inner) => format!("{} *{}", CType(inner).anonymous()?, name),
            _ => format!("{} {}", self.anonymous()?, name),
        })
    }

    fn function(&self, name: &str, params: &str) -> Result<String> {
        Ok(format!("{} {name}({params})", self.anonymous()?))
    }

    fn function_pointer(&self, name: &str, returns: &Type, params: &[Type]) -> Result<String> {
        let params = match params.is_empty() {
            true => "void".to_owned(),
            false => params
                .iter()
                .map(CType)
                .map(|ty| ty.anonymous())
                .collect::<Result<Vec<_>>>()?
                .join(", "),
        };
        Ok(format!(
            "{} (*{name})({params})",
            CType(returns).anonymous()?
        ))
    }
}
