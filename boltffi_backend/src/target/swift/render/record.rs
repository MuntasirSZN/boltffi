use askama::Template;
use boltffi_binding::{
    DirectFieldDecl, DirectRecordDecl, EncodedFieldDecl, EncodedRecordDecl, ExportedMethodDecl,
    FieldKey, Native, NativeSymbol, RecordDecl,
};

use crate::{
    bridge::c::CBridgeContract,
    core::{Emitted, Error, RenderContext, Result},
    target::swift::{
        SwiftHost,
        codec::{Reader, Writer},
        name_style::Name,
        render::{
            Documentation, SwiftType,
            function::{AssociatedFunction, Receiver},
        },
        syntax::{Expression, Identifier, Statement, TypeName},
    },
};

#[derive(Template)]
#[template(path = "target/swift/record.swift", escape = "none")]
struct RecordTemplate<'a> {
    record: &'a Record,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Record {
    documentation: Documentation,
    name: TypeName,
    conforms_to_error: bool,
    body: RecordBody,
    fields: Vec<Field>,
    initializers: Vec<AssociatedFunction>,
    static_methods: Vec<AssociatedFunction>,
    instance_methods: Vec<AssociatedFunction>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct Field {
    documentation: Documentation,
    name: Identifier,
    ty: TypeName,
    body: FieldBody,
}

#[derive(Clone, Debug, Eq, PartialEq)]
enum RecordBody {
    Direct { c_type: TypeName },
    Encoded,
}

#[derive(Clone, Debug, Eq, PartialEq)]
enum FieldBody {
    Direct { c_name: Identifier },
    Encoded { read: Expression, write: Statement },
}

impl Record {
    pub fn from_declaration(
        declaration: &RecordDecl<Native>,
        bridge: &CBridgeContract,
        context: &RenderContext<Native>,
    ) -> Result<Self> {
        match declaration {
            RecordDecl::Direct(record) => Self::from_direct(record, bridge, context),
            RecordDecl::Encoded(record) => Self::from_encoded(record, bridge, context),
            _ => Err(SwiftHost::unsupported("unknown record declaration")),
        }
    }

    pub fn from_declaration_as_error(
        declaration: &RecordDecl<Native>,
        bridge: &CBridgeContract,
        context: &RenderContext<Native>,
    ) -> Result<Self> {
        Self::from_declaration(declaration, bridge, context).map(|mut record| {
            record.conforms_to_error = true;
            record
        })
    }

    pub fn render(&self) -> Result<Emitted> {
        let mut source = RecordTemplate { record: self }.render()?;
        source.push_str("\n\n");
        let emitted = Emitted::primary(source);
        match self.requires_wire_runtime() {
            true => Ok(emitted.with_aux(AssociatedFunction::wire_helper()?)),
            false => Ok(emitted),
        }
    }

    fn name(&self) -> &TypeName {
        &self.name
    }

    fn documentation(&self) -> &Documentation {
        &self.documentation
    }

    fn error_conformance(&self) -> &str {
        match self.conforms_to_error {
            true => ", Error",
            false => "",
        }
    }

    fn direct(&self) -> bool {
        matches!(self.body, RecordBody::Direct { .. })
    }

    fn encoded(&self) -> bool {
        matches!(self.body, RecordBody::Encoded)
    }

    fn c_type(&self) -> &TypeName {
        match &self.body {
            RecordBody::Direct { c_type } => c_type,
            RecordBody::Encoded => unreachable!(),
        }
    }

    fn fields(&self) -> &[Field] {
        &self.fields
    }

    fn initializers(&self) -> &[AssociatedFunction] {
        &self.initializers
    }

    fn static_methods(&self) -> &[AssociatedFunction] {
        &self.static_methods
    }

    fn instance_methods(&self) -> &[AssociatedFunction] {
        &self.instance_methods
    }

    fn requires_wire_runtime(&self) -> bool {
        self.body.requires_wire_runtime()
            || self
                .initializers
                .iter()
                .chain(self.static_methods.iter())
                .chain(self.instance_methods.iter())
                .any(AssociatedFunction::requires_wire_runtime)
    }

    fn from_direct(
        record: &DirectRecordDecl<Native>,
        bridge: &CBridgeContract,
        context: &RenderContext<Native>,
    ) -> Result<Self> {
        let c_record =
            bridge
                .source_direct_record(record.id())
                .ok_or(Error::BrokenBridgeContract {
                    bridge: SwiftHost::TARGET,
                    invariant: "missing direct record C type for Swift record",
                })?;
        if record.fields().len() != c_record.fields().len() {
            return Err(Error::BrokenBridgeContract {
                bridge: SwiftHost::TARGET,
                invariant: "direct record field count mismatch",
            });
        }
        Ok(Self {
            documentation: Documentation::new(record.meta().doc(), ""),
            name: Name::new(record.name()).type_name(),
            conforms_to_error: false,
            body: RecordBody::Direct {
                c_type: TypeName::new(c_record.name()),
            },
            fields: record
                .fields()
                .iter()
                .zip(c_record.fields())
                .map(|(field, c_field)| Field::from_direct(field, c_field.name()))
                .collect::<Result<Vec<_>>>()?,
            initializers: record
                .initializers()
                .iter()
                .map(|initializer| {
                    AssociatedFunction::from_initializer(initializer, bridge, context)
                })
                .collect::<Result<Vec<_>>>()?,
            static_methods: Self::methods(record.methods(), None, bridge, context)?,
            instance_methods: Self::methods(
                record.methods(),
                Some(Receiver::direct()),
                bridge,
                context,
            )?,
        })
    }

    fn from_encoded(
        record: &EncodedRecordDecl<Native>,
        bridge: &CBridgeContract,
        context: &RenderContext<Native>,
    ) -> Result<Self> {
        let reader = Identifier::parse("reader")?;
        let writer = Identifier::parse("writer")?;
        Ok(Self {
            documentation: Documentation::new(record.meta().doc(), ""),
            name: Name::new(record.name()).type_name(),
            conforms_to_error: false,
            body: RecordBody::Encoded,
            fields: record
                .fields()
                .iter()
                .map(|field| Field::from_encoded(field, context, &reader, &writer))
                .collect::<Result<Vec<_>>>()?,
            initializers: record
                .initializers()
                .iter()
                .map(|initializer| {
                    AssociatedFunction::from_initializer(initializer, bridge, context)
                })
                .collect::<Result<Vec<_>>>()?,
            static_methods: Self::methods(record.methods(), None, bridge, context)?,
            instance_methods: Self::methods(
                record.methods(),
                Some(Receiver::encoded(record.name(), record.write(), context)?),
                bridge,
                context,
            )?,
        })
    }

    fn methods(
        methods: &[ExportedMethodDecl<Native, NativeSymbol>],
        receiver: Option<Receiver>,
        bridge: &CBridgeContract,
        context: &RenderContext<Native>,
    ) -> Result<Vec<AssociatedFunction>> {
        methods
            .iter()
            .filter(|method| method.callable().receiver().is_some() == receiver.is_some())
            .map(|method| {
                AssociatedFunction::from_method(method, receiver.clone(), bridge, context)
            })
            .collect()
    }
}

impl RecordBody {
    fn requires_wire_runtime(&self) -> bool {
        matches!(self, Self::Encoded)
    }
}

impl Field {
    fn from_direct(field: &DirectFieldDecl, c_name: &str) -> Result<Self> {
        Ok(Self {
            documentation: Documentation::new(field.meta().doc(), "    "),
            name: Self::field_name(field.key())?,
            ty: SwiftType::primitive(field.ty().primitive())?,
            body: FieldBody::Direct {
                c_name: Identifier::parse(c_name)?,
            },
        })
    }

    fn from_encoded(
        field: &EncodedFieldDecl,
        context: &RenderContext<Native>,
        reader: &Identifier,
        writer: &Identifier,
    ) -> Result<Self> {
        let name = Self::field_name(field.key())?;
        let read = field
            .read()
            .render_with(&mut Reader::new(reader.clone(), context))?;
        let write = field
            .write()
            .render_with(&mut Writer::new(
                writer.clone(),
                Expression::new("self"),
                context,
            ))
            .into_iter()
            .collect::<Result<Vec<_>>>()?;
        match write.as_slice() {
            [write] => Ok(Self {
                documentation: Documentation::new(field.meta().doc(), "    "),
                name,
                ty: SwiftType::type_ref(field.ty(), context)?,
                body: FieldBody::Encoded {
                    read,
                    write: write.clone(),
                },
            }),
            _ => Err(SwiftHost::unsupported(
                "multi-statement encoded record field",
            )),
        }
    }

    fn name(&self) -> &Identifier {
        &self.name
    }

    fn documentation(&self) -> &Documentation {
        &self.documentation
    }

    fn ty(&self) -> &TypeName {
        &self.ty
    }

    fn assignment(&self) -> Expression {
        Expression::new(format!("self.{} = {}", self.name, self.name))
    }

    fn read(&self) -> &Expression {
        match &self.body {
            FieldBody::Encoded { read, .. } => read,
            FieldBody::Direct { .. } => unreachable!(),
        }
    }

    fn write(&self) -> &Statement {
        match &self.body {
            FieldBody::Encoded { write, .. } => write,
            FieldBody::Direct { .. } => unreachable!(),
        }
    }

    fn c_initializer_argument(&self) -> Expression {
        match &self.body {
            FieldBody::Direct { c_name } => {
                Expression::labeled(&self.name, Expression::member("c", c_name))
            }
            FieldBody::Encoded { .. } => unreachable!(),
        }
    }

    fn c_value_argument(&self) -> Expression {
        match &self.body {
            FieldBody::Direct { c_name } => Expression::labeled(c_name, &self.name),
            FieldBody::Encoded { .. } => unreachable!(),
        }
    }

    fn field_name(key: &FieldKey) -> Result<Identifier> {
        match key {
            FieldKey::Named(name) => Name::new(name).field(),
            FieldKey::Position(position) => Identifier::parse(format!("field{position}")),
            _ => Err(SwiftHost::unsupported("unknown record field key")),
        }
    }
}
