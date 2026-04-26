use crate::ir::types::PrimitiveType;

use super::{PythonCallable, PythonNativeCallable, PythonParameter, PythonType};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PythonDirectRecordField {
    pub native_name: String,
    pub primitive: PrimitiveType,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PythonDirectRecordLayout {
    pub size_bytes: usize,
    pub fields: Vec<PythonDirectRecordField>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PythonRecordTransport {
    Direct(PythonDirectRecordLayout),
    Encoded,
}

impl PythonRecordTransport {
    pub fn direct_layout(&self) -> Option<&PythonDirectRecordLayout> {
        match self {
            Self::Direct(layout) => Some(layout),
            Self::Encoded => None,
        }
    }

    pub fn is_direct(&self) -> bool {
        matches!(self, Self::Direct(_))
    }

    pub fn is_encoded(&self) -> bool {
        matches!(self, Self::Encoded)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PythonRecordType {
    pub native_name_stem: String,
    pub class_name: String,
    pub c_type_name: String,
    pub transport: PythonRecordTransport,
}

impl PythonRecordType {
    pub fn type_object_name(&self) -> String {
        format!("boltffi_python_{}_type", self.native_name_stem)
    }

    pub fn parser_name(&self) -> String {
        format!("boltffi_python_parse_{}", self.native_name_stem)
    }

    pub fn boxer_name(&self) -> String {
        format!("boltffi_python_box_{}", self.native_name_stem)
    }

    pub fn type_literal(&self) -> String {
        self.class_name.clone()
    }

    pub fn registration_function_name(&self) -> String {
        format!("_register_{}", self.native_name_stem)
    }

    pub fn registration_wrapper_name(&self) -> String {
        format!("boltffi_python_wrapper_register_{}", self.native_name_stem)
    }

    pub fn wire_encoder_name(&self) -> String {
        format!("boltffi_python_encode_{}_wire", self.native_name_stem)
    }

    pub fn wire_decoder_name(&self) -> String {
        format!("boltffi_python_decode_{}_wire", self.native_name_stem)
    }

    pub fn vector_parser_name(&self) -> String {
        format!("boltffi_python_parse_vec_{}", self.native_name_stem)
    }

    pub fn vector_decoder_name(&self) -> String {
        format!("boltffi_python_decode_owned_vec_{}", self.native_name_stem)
    }

    pub fn wire_vector_encoder_name(&self) -> String {
        format!("boltffi_python_wire_encode_vec_{}", self.native_name_stem)
    }

    pub fn wire_vector_decoder_name(&self) -> String {
        format!("boltffi_python_wire_decode_vec_{}", self.native_name_stem)
    }

    pub fn direct_layout(&self) -> Option<&PythonDirectRecordLayout> {
        self.transport.direct_layout()
    }

    pub fn direct_layout_unwrap(&self) -> &PythonDirectRecordLayout {
        self.direct_layout()
            .expect("direct python record layout should exist")
    }

    pub fn is_direct(&self) -> bool {
        self.transport.is_direct()
    }

    pub fn is_encoded(&self) -> bool {
        self.transport.is_encoded()
    }

    pub fn native_primitive_types(&self) -> Vec<PrimitiveType> {
        self.direct_layout()
            .map(|layout| layout.fields.iter().map(|field| field.primitive).collect())
            .unwrap_or_default()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PythonRecordField {
    pub python_name: String,
    pub native_name: String,
    pub type_ref: PythonType,
}

impl PythonRecordField {
    pub fn annotation(&self) -> String {
        self.type_ref.return_annotation()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PythonRecordFields {
    fields: Vec<PythonRecordField>,
}

impl PythonRecordFields {
    pub fn from_vec(fields: Vec<PythonRecordField>) -> Self {
        Self { fields }
    }

    pub fn iter(&self) -> impl Iterator<Item = &PythonRecordField> {
        self.fields.iter()
    }

    pub fn first(&self) -> Option<&PythonRecordField> {
        self.fields.first()
    }

    pub fn len(&self) -> usize {
        self.fields.len()
    }

    pub fn is_empty(&self) -> bool {
        self.fields.is_empty()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PythonRecordConstructor {
    pub python_name: String,
    pub callable: PythonCallable,
}

impl PythonRecordConstructor {
    pub fn callable(&self) -> &PythonCallable {
        &self.callable
    }

    pub fn native_callable(&self) -> PythonNativeCallable<'_> {
        PythonNativeCallable {
            module_attribute_name: self.callable.native_name.as_str(),
            callable: &self.callable,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PythonRecordMethod {
    pub python_name: String,
    pub callable: PythonCallable,
    pub is_static: bool,
}

impl PythonRecordMethod {
    pub fn callable(&self) -> &PythonCallable {
        &self.callable
    }

    pub fn native_callable(&self) -> PythonNativeCallable<'_> {
        PythonNativeCallable {
            module_attribute_name: self.callable.native_name.as_str(),
            callable: &self.callable,
        }
    }

    pub fn public_parameters(&self) -> &[PythonParameter] {
        if self.is_static {
            &self.callable.parameters
        } else {
            &self.callable.parameters[1..]
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PythonRecord {
    pub type_ref: PythonRecordType,
    pub fields: PythonRecordFields,
    pub constructors: Vec<PythonRecordConstructor>,
    pub methods: Vec<PythonRecordMethod>,
}

impl PythonRecord {
    pub fn new(
        type_ref: PythonRecordType,
        fields: Vec<PythonRecordField>,
        constructors: Vec<PythonRecordConstructor>,
        methods: Vec<PythonRecordMethod>,
    ) -> Self {
        Self {
            type_ref,
            fields: PythonRecordFields::from_vec(fields),
            constructors,
            methods,
        }
    }

    pub fn class_name(&self) -> &str {
        &self.type_ref.class_name
    }

    pub fn field_count(&self) -> usize {
        self.fields.len()
    }

    pub fn has_empty_body(&self) -> bool {
        self.fields.is_empty() && self.constructors.is_empty() && self.methods.is_empty()
    }

    pub fn is_direct(&self) -> bool {
        self.type_ref.is_direct()
    }

    pub fn is_encoded(&self) -> bool {
        self.type_ref.is_encoded()
    }

    pub fn callables(&self) -> impl Iterator<Item = &PythonCallable> {
        self.constructors
            .iter()
            .map(PythonRecordConstructor::callable)
            .chain(self.methods.iter().map(PythonRecordMethod::callable))
    }

    pub fn native_callables(&self) -> impl Iterator<Item = PythonNativeCallable<'_>> {
        self.constructors
            .iter()
            .map(PythonRecordConstructor::native_callable)
            .chain(self.methods.iter().map(PythonRecordMethod::native_callable))
    }

    pub fn has_native_callables(&self) -> bool {
        !self.constructors.is_empty() || !self.methods.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use crate::ir::types::PrimitiveType;

    use super::{
        PythonDirectRecordField, PythonDirectRecordLayout, PythonRecord, PythonRecordField,
        PythonRecordTransport, PythonRecordType,
    };
    use crate::render::python::PythonType;

    #[test]
    fn direct_python_records_keep_first_field_accessible() {
        let record = PythonRecord::new(
            PythonRecordType {
                native_name_stem: "point".to_string(),
                class_name: "Point".to_string(),
                c_type_name: "___Point".to_string(),
                transport: PythonRecordTransport::Direct(PythonDirectRecordLayout {
                    size_bytes: 8,
                    fields: vec![PythonDirectRecordField {
                        native_name: "x".to_string(),
                        primitive: PrimitiveType::F64,
                    }],
                }),
            },
            vec![PythonRecordField {
                python_name: "x".to_string(),
                native_name: "x".to_string(),
                type_ref: PythonType::Primitive(PrimitiveType::F64),
            }],
            vec![],
            vec![],
        );

        assert_eq!(record.fields.first().unwrap().python_name, "x");
        assert_eq!(record.field_count(), 1);
    }

    #[test]
    fn python_records_allow_empty_class_bodies() {
        let record = PythonRecord::new(
            PythonRecordType {
                native_name_stem: "empty".to_string(),
                class_name: "Empty".to_string(),
                c_type_name: "___Empty".to_string(),
                transport: PythonRecordTransport::Encoded,
            },
            vec![],
            vec![],
            vec![],
        );

        assert!(record.has_empty_body());
    }
}
