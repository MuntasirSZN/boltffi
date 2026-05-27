mod callback;
mod custom_type;
mod enumeration;
mod function;
mod record;
mod r#type;

pub use callback::*;
pub use custom_type::*;
pub use enumeration::*;
pub use function::*;
pub use record::*;
pub use r#type::*;

#[derive(Debug, Clone)]
pub enum DartConstructorKind {
    Default,
    Named { name: String },
}

#[derive(Debug, Clone)]
pub struct DartConstructor {
    pub ffi_name: String,
    pub kind: DartConstructorKind,
    pub params: Vec<DartFunctionParam>,
    pub is_fallible: bool,
}

#[derive(Debug, Clone)]
pub struct DartLibrary {
    pub custom_types: Vec<DartCustomType>,
    pub native: DartNative,
    pub records: Vec<DartRecord>,
    pub enums: Vec<DartEnum>,
    pub callbacks: Vec<DartCallback>,
}
