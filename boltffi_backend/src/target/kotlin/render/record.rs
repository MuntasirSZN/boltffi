use askama::Template as AskamaTemplate;
use boltffi_binding::{
    DirectFieldDecl, DirectRecordDecl, EncodedFieldDecl, EncodedRecordDecl, FieldKey, Native,
    RecordDecl, RecordId,
};

use crate::{
    core::{Emitted, Error, RenderContext, Result},
    target::kotlin::{
        codec::Sizer,
        name_style::Name,
        primitive::KotlinPrimitive,
        render::field::EncodedField,
        syntax::{Expression, Identifier, Statement, TypeName},
    },
};

const KOTLIN_TARGET: &str = "kotlin";

#[derive(AskamaTemplate)]
#[template(path = "target/kotlin/record.kt", escape = "none")]
struct RecordTemplate {
    record: Record,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Record {
    name: TypeName,
    body: RecordBody,
    fields: Vec<Field>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
enum RecordBody {
    Direct { size: u64 },
    Encoded { size: Expression },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Field {
    name: Identifier,
    ty: TypeName,
    read: Expression,
    write: Statement,
    size: Option<Expression>,
}

impl Record {
    pub fn from_declaration(
        declaration: &RecordDecl<Native>,
        context: &RenderContext<Native>,
    ) -> Result<Self> {
        match declaration {
            RecordDecl::Direct(record) => Self::from_direct(record),
            RecordDecl::Encoded(record) => Self::from_encoded(record, context),
            _ => Err(Error::UnsupportedTarget {
                target: KOTLIN_TARGET,
                shape: "unknown record declaration",
            }),
        }
    }

    pub fn from_id(id: RecordId, context: &RenderContext<Native>) -> Result<Self> {
        context
            .record(id)
            .ok_or(Error::BrokenBridgeContract {
                bridge: KOTLIN_TARGET,
                invariant: "record type was not found in render context",
            })
            .and_then(|record| Self::from_declaration(record, context))
    }

    pub fn render(self) -> Result<Emitted> {
        Ok(Emitted::primary(RecordTemplate { record: self }.render()?))
    }

    pub fn name(&self) -> &TypeName {
        &self.name
    }

    pub fn size(&self) -> u64 {
        match self.body {
            RecordBody::Direct { size } => size,
            RecordBody::Encoded { .. } => 0,
        }
    }

    pub fn wire_size(&self) -> Option<&Expression> {
        match &self.body {
            RecordBody::Encoded { size } => Some(size),
            RecordBody::Direct { .. } => None,
        }
    }

    pub fn encoded(&self) -> bool {
        matches!(self.body, RecordBody::Encoded { .. })
    }

    pub fn fields(&self) -> &[Field] {
        &self.fields
    }

    pub fn empty(&self) -> bool {
        self.fields.is_empty()
    }

    pub fn type_name_from_id(id: RecordId, context: &RenderContext<Native>) -> Result<TypeName> {
        Self::from_id(id, context).map(|record| record.name)
    }

    pub fn encode_expression(value: Expression) -> Result<Expression> {
        Ok(Expression::call(
            value,
            Identifier::parse("toByteArray")?,
            Default::default(),
        ))
    }

    pub fn decode_expression(record: TypeName, value: Expression) -> Result<Expression> {
        Ok(Expression::call(
            record,
            Identifier::parse("fromByteArray")?,
            [value].into_iter().collect(),
        ))
    }

    fn from_direct(record: &DirectRecordDecl<Native>) -> Result<Self> {
        let buffer = Identifier::parse("buffer")?;
        Ok(Self {
            name: Name::new(record.name()).type_name(),
            body: RecordBody::Direct {
                size: record.layout().size().get(),
            },
            fields: record
                .fields()
                .iter()
                .map(|field| Field::from_direct(field, record, &buffer))
                .collect::<Result<Vec<_>>>()?,
        })
    }

    fn from_encoded(
        record: &EncodedRecordDecl<Native>,
        context: &RenderContext<Native>,
    ) -> Result<Self> {
        let reader = Identifier::parse("reader")?;
        let writer = Identifier::parse("writer")?;
        let current = Expression::this();
        let size = record
            .fields()
            .iter()
            .map(|field| {
                field
                    .write()
                    .size_with(&mut Sizer::new(context)?.current(current.clone()))
            })
            .collect::<Result<Vec<_>>>()?
            .into_iter()
            .reduce(Expression::add)
            .unwrap_or_else(|| Expression::integer(0));
        Ok(Self {
            name: Name::new(record.name()).type_name(),
            body: RecordBody::Encoded { size },
            fields: record
                .fields()
                .iter()
                .map(|field| Field::from_encoded(field, context, &reader, &writer, current.clone()))
                .collect::<Result<Vec<_>>>()?,
        })
    }
}

impl Field {
    pub fn name(&self) -> &Identifier {
        &self.name
    }

    pub fn ty(&self) -> &TypeName {
        &self.ty
    }

    pub fn read(&self) -> &Expression {
        &self.read
    }

    pub fn write(&self) -> &Statement {
        &self.write
    }

    fn from_direct(
        field: &DirectFieldDecl,
        record: &DirectRecordDecl<Native>,
        buffer: &Identifier,
    ) -> Result<Self> {
        let name = Self::identifier(field.key())?;
        let offset = record
            .layout()
            .field(field.key())
            .ok_or(Error::BrokenBridgeContract {
                bridge: KOTLIN_TARGET,
                invariant: "direct record field layout was not found",
            })?
            .offset()
            .get();
        let primitive = field.ty().primitive();
        Ok(Self {
            ty: KotlinPrimitive::new(primitive).api_type()?,
            read: KotlinPrimitive::new(primitive).buffer_read(buffer, offset)?,
            write: KotlinPrimitive::new(primitive).buffer_write(
                buffer,
                offset,
                Expression::identifier(name.clone()),
            )?,
            size: None,
            name,
        })
    }

    fn from_encoded(
        field: &EncodedFieldDecl,
        context: &RenderContext<Native>,
        reader: &Identifier,
        writer: &Identifier,
        current: Expression,
    ) -> Result<Self> {
        let name = Self::identifier(field.key())?;
        let field = EncodedField::from_declaration(field, context, reader, writer, current)?;
        Ok(Self {
            ty: field.ty().clone(),
            read: field.read().clone(),
            write: field.write().clone(),
            size: Some(field.size().clone()),
            name,
        })
    }

    fn identifier(key: &FieldKey) -> Result<Identifier> {
        match key {
            FieldKey::Named(name) => Name::new(name).parameter(),
            FieldKey::Position(position) => Identifier::parse(format!("field{position}")),
            _ => Err(Error::UnsupportedTarget {
                target: KOTLIN_TARGET,
                shape: "unknown direct record field key",
            }),
        }
    }
}
