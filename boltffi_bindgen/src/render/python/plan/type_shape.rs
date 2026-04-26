use crate::ir::types::PrimitiveType;
use crate::render::python::primitives::PythonScalarTypeExt as _;

use super::{PythonEnumType, PythonRecordType};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PythonSequenceType {
    Bytes,
    PrimitiveVec(PrimitiveType),
    StringVec,
    CStyleEnumVec(PythonEnumType),
    RecordVec(PythonRecordType),
}

impl PythonSequenceType {
    pub fn parameter_annotation(&self) -> String {
        match self {
            Self::Bytes => "bytes".to_string(),
            Self::PrimitiveVec(PrimitiveType::U8) => "bytes | Sequence[int]".to_string(),
            Self::PrimitiveVec(primitive) => {
                format!("Sequence[{}]", primitive.python_annotation())
            }
            Self::StringVec => "Sequence[str]".to_string(),
            Self::CStyleEnumVec(enum_type) => format!("Sequence[{}]", enum_type.type_literal()),
            Self::RecordVec(record_type) => format!("Sequence[{}]", record_type.type_literal()),
        }
    }

    pub fn return_annotation(&self) -> String {
        match self {
            Self::Bytes | Self::PrimitiveVec(PrimitiveType::U8) => "bytes".to_string(),
            Self::PrimitiveVec(primitive) => {
                format!("list[{}]", primitive.python_annotation())
            }
            Self::StringVec => "list[str]".to_string(),
            Self::CStyleEnumVec(enum_type) => format!("list[{}]", enum_type.type_literal()),
            Self::RecordVec(record_type) => format!("list[{}]", record_type.type_literal()),
        }
    }

    pub fn primitive_element(&self) -> Option<PrimitiveType> {
        match self {
            Self::Bytes => None,
            Self::PrimitiveVec(primitive) => Some(*primitive),
            Self::StringVec => None,
            Self::CStyleEnumVec(_) => None,
            Self::RecordVec(_) => None,
        }
    }

    pub fn enum_element(&self) -> Option<&PythonEnumType> {
        match self {
            Self::CStyleEnumVec(enum_type) => Some(enum_type),
            _ => None,
        }
    }

    pub fn record_element(&self) -> Option<&PythonRecordType> {
        match self {
            Self::RecordVec(record_type) => Some(record_type),
            _ => None,
        }
    }

    pub fn is_bytes(&self) -> bool {
        matches!(self, Self::Bytes)
    }

    pub fn is_byte_like(&self) -> bool {
        matches!(self, Self::Bytes | Self::PrimitiveVec(PrimitiveType::U8))
    }

    pub fn is_primitive_vector(&self) -> bool {
        matches!(self, Self::PrimitiveVec(_))
    }

    pub fn is_c_style_enum_vector(&self) -> bool {
        matches!(self, Self::CStyleEnumVec(_))
    }

    pub fn is_string_vector(&self) -> bool {
        matches!(self, Self::StringVec)
    }

    pub fn is_record_vector(&self) -> bool {
        matches!(self, Self::RecordVec(_))
    }

    pub fn uses_buffer_input(&self) -> bool {
        matches!(
            self,
            Self::Bytes
                | Self::PrimitiveVec(_)
                | Self::StringVec
                | Self::CStyleEnumVec(_)
                | Self::RecordVec(_)
        )
    }

    pub fn is_encoded_buffer(&self) -> bool {
        match self {
            Self::Bytes | Self::PrimitiveVec(_) => false,
            Self::StringVec | Self::CStyleEnumVec(_) => true,
            Self::RecordVec(record_type) => record_type.is_encoded(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PythonType {
    Void,
    Primitive(PrimitiveType),
    Record(PythonRecordType),
    CStyleEnum(PythonEnumType),
    String,
    Sequence(PythonSequenceType),
}

impl PythonType {
    pub fn parameter_annotation(&self) -> String {
        match self {
            Self::Void => "None".to_string(),
            Self::Primitive(primitive) => primitive.python_annotation().to_string(),
            Self::Record(record_type) => record_type.type_literal(),
            Self::CStyleEnum(enum_type) => enum_type.type_literal(),
            Self::String => "str".to_string(),
            Self::Sequence(sequence) => sequence.parameter_annotation(),
        }
    }

    pub fn return_annotation(&self) -> String {
        match self {
            Self::Void => "None".to_string(),
            Self::Primitive(primitive) => primitive.python_annotation().to_string(),
            Self::Record(record_type) => record_type.type_literal(),
            Self::CStyleEnum(enum_type) => enum_type.type_literal(),
            Self::String => "str".to_string(),
            Self::Sequence(sequence) => sequence.return_annotation(),
        }
    }

    pub fn native_primitive(&self) -> Option<PrimitiveType> {
        match self {
            Self::Void => None,
            Self::Primitive(primitive) => Some(*primitive),
            Self::Record(_) => None,
            Self::CStyleEnum(enum_type) => Some(enum_type.tag_type),
            Self::String => None,
            Self::Sequence(sequence) => sequence.primitive_element(),
        }
    }

    pub fn native_primitive_types(&self) -> Vec<PrimitiveType> {
        match self {
            Self::Void | Self::String => Vec::new(),
            Self::Primitive(primitive) => vec![*primitive],
            Self::Record(record_type) => record_type.native_primitive_types(),
            Self::CStyleEnum(enum_type) => vec![enum_type.tag_type],
            Self::Sequence(PythonSequenceType::Bytes) => vec![PrimitiveType::U8],
            Self::Sequence(PythonSequenceType::PrimitiveVec(primitive)) => vec![*primitive],
            Self::Sequence(PythonSequenceType::StringVec) => Vec::new(),
            Self::Sequence(PythonSequenceType::CStyleEnumVec(enum_type)) => {
                vec![enum_type.tag_type]
            }
            Self::Sequence(PythonSequenceType::RecordVec(record_type)) => {
                record_type.native_primitive_types()
            }
        }
    }

    pub fn primitive(&self) -> Option<PrimitiveType> {
        match self {
            Self::Primitive(primitive) => Some(*primitive),
            _ => None,
        }
    }

    pub fn record(&self) -> Option<&PythonRecordType> {
        match self {
            Self::Record(record_type) => Some(record_type),
            _ => None,
        }
    }

    pub fn c_style_enum(&self) -> Option<&PythonEnumType> {
        match self {
            Self::CStyleEnum(enum_type) => Some(enum_type),
            _ => None,
        }
    }

    pub fn sequence_c_style_enum(&self) -> Option<&PythonEnumType> {
        match self {
            Self::Sequence(sequence) => sequence.enum_element(),
            _ => None,
        }
    }

    pub fn sequence_primitive(&self) -> Option<PrimitiveType> {
        match self {
            Self::Sequence(PythonSequenceType::PrimitiveVec(primitive)) => Some(*primitive),
            _ => None,
        }
    }

    pub fn sequence_record(&self) -> Option<&PythonRecordType> {
        match self {
            Self::Sequence(sequence) => sequence.record_element(),
            _ => None,
        }
    }

    pub fn is_void(&self) -> bool {
        matches!(self, Self::Void)
    }

    pub fn is_string(&self) -> bool {
        matches!(self, Self::String)
    }

    pub fn is_record(&self) -> bool {
        matches!(self, Self::Record(_))
    }

    pub fn is_c_style_enum(&self) -> bool {
        matches!(self, Self::CStyleEnum(_))
    }

    pub fn is_bytes(&self) -> bool {
        matches!(self, Self::Sequence(PythonSequenceType::Bytes))
    }

    pub fn is_byte_like(&self) -> bool {
        matches!(self, Self::Sequence(sequence) if sequence.is_byte_like())
    }

    pub fn is_primitive_vector(&self) -> bool {
        matches!(self, Self::Sequence(PythonSequenceType::PrimitiveVec(_)))
    }

    pub fn is_c_style_enum_vector(&self) -> bool {
        matches!(self, Self::Sequence(PythonSequenceType::CStyleEnumVec(_)))
    }

    pub fn is_string_vector(&self) -> bool {
        matches!(self, Self::Sequence(PythonSequenceType::StringVec))
    }

    pub fn is_record_vector(&self) -> bool {
        matches!(self, Self::Sequence(PythonSequenceType::RecordVec(_)))
    }

    pub fn uses_buffer_input(&self) -> bool {
        matches!(
            self,
            Self::Record(record_type) if record_type.is_encoded()
        ) || matches!(self, Self::Sequence(sequence) if sequence.uses_buffer_input())
    }

    pub fn is_owned_buffer(&self) -> bool {
        matches!(self, Self::String | Self::Sequence(_))
            || matches!(self, Self::Record(record_type) if record_type.is_encoded())
    }

    pub fn is_encoded_buffer(&self) -> bool {
        matches!(self, Self::Record(record_type) if record_type.is_encoded())
            || matches!(self, Self::Sequence(sequence) if sequence.is_encoded_buffer())
    }
}
