mod read;
mod size;
mod value;
mod write;

pub use read::Reader;
pub use size::Sizer;
pub use value::ValueScope;
pub use write::Writer;

pub fn primitive_read_method(primitive: boltffi_binding::Primitive) -> &'static str {
    use boltffi_binding::Primitive;
    match primitive {
        Primitive::Bool => "readBool",
        Primitive::I8 => "readI8",
        Primitive::U8 => "readU8",
        Primitive::I16 => "readI16",
        Primitive::U16 => "readU16",
        Primitive::I32 => "readI32",
        Primitive::U32 => "readU32",
        Primitive::I64 | Primitive::ISize => "readI64",
        Primitive::U64 | Primitive::USize => "readU64",
        Primitive::F32 => "readF32",
        Primitive::F64 => "readF64",
        _ => unreachable!("unsupported primitive passed Dart IR validation"),
    }
}

pub fn primitive_write_method(primitive: boltffi_binding::Primitive) -> &'static str {
    use boltffi_binding::Primitive;
    match primitive {
        Primitive::Bool => "writeBool",
        Primitive::I8 => "writeI8",
        Primitive::U8 => "writeU8",
        Primitive::I16 => "writeI16",
        Primitive::U16 => "writeU16",
        Primitive::I32 => "writeI32",
        Primitive::U32 => "writeU32",
        Primitive::I64 | Primitive::ISize => "writeI64",
        Primitive::U64 | Primitive::USize => "writeU64",
        Primitive::F32 => "writeF32",
        Primitive::F64 => "writeF64",
        _ => unreachable!("unsupported primitive passed Dart IR validation"),
    }
}

pub fn primitive_size(primitive: boltffi_binding::Primitive) -> usize {
    use boltffi_binding::Primitive;
    match primitive {
        Primitive::Bool | Primitive::I8 | Primitive::U8 => 1,
        Primitive::I16 | Primitive::U16 => 2,
        Primitive::I32 | Primitive::U32 | Primitive::F32 => 4,
        Primitive::I64 | Primitive::U64 | Primitive::ISize | Primitive::USize | Primitive::F64 => 8,
        _ => unreachable!("unsupported primitive passed Dart IR validation"),
    }
}
