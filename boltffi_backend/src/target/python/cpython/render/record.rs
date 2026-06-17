use askama::Template as AskamaTemplate;
use boltffi_binding::{
    DeclarationRef, DirectFieldDecl, DirectRecordDecl, FieldKey, Native, RecordDecl, RecordId,
    TypeRef,
};

use crate::{
    bridge::{
        c::{self, identifier::Identifier, syntax::TypeSyntax},
        python_cext::{ExtensionMethod, MethodFlags, PythonCExtBridgeContract},
    },
    core::{Emitted, Error, RenderContext, Result},
    target::python::{cpython::render::primitive, name_style::Name},
};

#[derive(AskamaTemplate)]
#[template(path = "target/python/record.c", escape = "none")]
struct Template {
    class_name: String,
    c_type: String,
    type_object: String,
    register_method: String,
    register_wrapper: String,
    parser: String,
    boxer: String,
    fields: Vec<Field>,
}

pub struct Wrapper {
    symbols: Symbols,
    fields: Vec<Field>,
    method: ExtensionMethod,
    primitives: Vec<primitive::Runtime>,
}

impl Wrapper {
    pub fn from_declaration(
        declaration: &RecordDecl<Native>,
        bridge: &PythonCExtBridgeContract,
    ) -> Result<Self> {
        match declaration {
            RecordDecl::Direct(record) => Self::from_direct(record, bridge),
            RecordDecl::Encoded(_) => Err(Error::UnsupportedTarget {
                target: "python",
                shape: "encoded record",
            }),
            _ => Err(Error::UnsupportedTarget {
                target: "python",
                shape: "unknown record",
            }),
        }
    }

    pub fn render(self) -> Result<Emitted> {
        let source = Template {
            class_name: self.symbols.class_name,
            c_type: self.symbols.c_type,
            type_object: self.symbols.type_object,
            register_method: self.symbols.register_method,
            register_wrapper: self.symbols.register_wrapper,
            parser: self.symbols.parser,
            boxer: self.symbols.boxer,
            fields: self.fields,
        }
        .render()?;
        Ok(Emitted::primary(source))
    }

    pub fn method(&self) -> &ExtensionMethod {
        &self.method
    }

    pub fn primitives(&self) -> impl Iterator<Item = primitive::Runtime> + '_ {
        self.primitives.iter().copied()
    }

    pub fn cleanup(&self) -> String {
        format!("Py_CLEAR({})", self.symbols.type_object)
    }

    fn from_direct(
        record: &DirectRecordDecl<Native>,
        bridge: &PythonCExtBridgeContract,
    ) -> Result<Self> {
        let c_record =
            bridge
                .source_direct_record(record.id())
                .ok_or(Error::UnsupportedTarget {
                    target: "python",
                    shape: "direct record without C typedef",
                })?;
        if record.fields().len() != c_record.fields().len() {
            return Err(Error::UnsupportedTarget {
                target: "python",
                shape: "record field mismatch",
            });
        }
        let symbols = Symbols::from_direct(record, c_record)?;
        let fields = record
            .fields()
            .iter()
            .zip(c_record.fields())
            .map(|(source, c_field)| Field::new(source, c_field))
            .collect::<Result<Vec<_>>>()?;
        let primitives = fields.iter().map(Field::primitive).collect();
        let method = ExtensionMethod::new(
            symbols.register_method.clone(),
            symbols.register_wrapper.clone(),
            MethodFlags::FastCall,
        )?;
        Ok(Self {
            symbols,
            fields,
            method,
            primitives,
        })
    }
}

pub struct Symbols {
    class_name: String,
    c_type: String,
    type_object: String,
    register_method: String,
    register_wrapper: String,
    parser: String,
    boxer: String,
}

impl Symbols {
    pub fn from_record_id(
        record_id: RecordId,
        bridge: &PythonCExtBridgeContract,
        context: &RenderContext<Native>,
    ) -> Result<Self> {
        let record = context
            .bindings()
            .decls()
            .iter()
            .find_map(|decl| match DeclarationRef::from(decl) {
                DeclarationRef::Record(RecordDecl::Direct(record)) if record.id() == record_id => {
                    Some(record)
                }
                DeclarationRef::Record(_)
                | DeclarationRef::Enum(_)
                | DeclarationRef::Function(_)
                | DeclarationRef::Class(_)
                | DeclarationRef::Callback(_)
                | DeclarationRef::Stream(_)
                | DeclarationRef::Constant(_)
                | DeclarationRef::CustomType(_) => None,
            })
            .ok_or(Error::UnsupportedTarget {
                target: "python",
                shape: "record id without direct declaration",
            })?;
        let c_record = bridge
            .source_direct_record(record_id)
            .ok_or(Error::UnsupportedTarget {
                target: "python",
                shape: "direct record without C typedef",
            })?;
        Self::from_direct(record, c_record)
    }

    pub fn c_type(&self) -> &str {
        &self.c_type
    }

    pub fn parser(&self) -> &str {
        &self.parser
    }

    pub fn boxer(&self) -> &str {
        &self.boxer
    }

    pub fn class_name(&self) -> &str {
        &self.class_name
    }

    pub fn register_method(&self) -> &str {
        &self.register_method
    }

    pub fn from_direct(record: &DirectRecordDecl<Native>, c_record: &c::Record) -> Result<Self> {
        let stem = Identifier::escape(Name::new(record.name()).function())?.to_string();
        Ok(Self {
            class_name: Name::new(record.name()).class(),
            c_type: TypeSyntax::new(&c::Type::Named(c_record.name().to_owned())).anonymous()?,
            type_object: format!("boltffi_python_{stem}_type"),
            register_method: format!("_register_{stem}"),
            register_wrapper: format!("boltffi_python_wrapper_register_{stem}"),
            parser: format!("boltffi_python_parse_{stem}"),
            boxer: format!("boltffi_python_box_{stem}"),
        })
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct Field {
    python_name: String,
    c_name: String,
    value_name: String,
    parser: &'static str,
    boxer: &'static str,
    primitive: primitive::Runtime,
}

impl Field {
    fn new(source: &DirectFieldDecl, c_field: &c::Field) -> Result<Self> {
        let TypeRef::Primitive(primitive) = source.ty() else {
            return Err(Error::UnsupportedTarget {
                target: "python",
                shape: "non-primitive direct record field",
            });
        };
        let primitive = primitive::Runtime::new(*primitive);
        let python_name = Self::python_name(source.key())?;
        Ok(Self {
            value_name: Identifier::escape(format!("{python_name}_value"))?.to_string(),
            python_name,
            c_name: c_field.name().to_owned(),
            parser: primitive.parser()?,
            boxer: primitive.boxer()?,
            primitive,
        })
    }

    fn primitive(&self) -> primitive::Runtime {
        self.primitive
    }

    fn python_name(key: &FieldKey) -> Result<String> {
        Ok(match key {
            FieldKey::Named(name) => Name::new(name).function(),
            FieldKey::Position(position) => format!("field_{position}"),
            _ => {
                return Err(Error::UnsupportedTarget {
                    target: "python",
                    shape: "unknown record field key",
                });
            }
        })
    }
}
