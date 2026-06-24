use boltffi_binding::{EncodedFieldDecl, FieldKey, Native};

use crate::{
    core::{Error, RenderContext, Result},
    target::kotlin::{
        codec::{Reader, Sizer, Writer},
        name_style::Name,
        render::type_name::KotlinType,
        syntax::{Expression, Identifier, Statement, TypeName},
    },
};

const KOTLIN_TARGET: &str = "kotlin";

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EncodedField {
    name: Identifier,
    ty: TypeName,
    read: Expression,
    write: Statement,
    size: Expression,
}

impl EncodedField {
    pub fn from_declaration(
        field: &EncodedFieldDecl,
        context: &RenderContext<Native>,
        reader: &Identifier,
        writer: &Identifier,
        current: Expression,
    ) -> Result<Self> {
        let mut writer = Writer::new(writer.clone(), context)?.current(current.clone());
        let write = field
            .write()
            .render_with(&mut writer)
            .into_iter()
            .collect::<Result<Vec<_>>>()?;
        match write.as_slice() {
            [write] => Ok(Self {
                name: Self::identifier(field.key())?,
                ty: KotlinType::type_ref(field.ty(), context)?,
                read: field
                    .read()
                    .render_with(&mut Reader::new(reader.clone(), context))?,
                write: write.clone(),
                size: field
                    .write()
                    .size_with(&mut Sizer::new(context)?.current(current))?,
            }),
            _ => Err(Error::UnsupportedTarget {
                target: KOTLIN_TARGET,
                shape: "multi-statement encoded field",
            }),
        }
    }

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

    pub fn size(&self) -> &Expression {
        &self.size
    }

    fn identifier(key: &FieldKey) -> Result<Identifier> {
        match key {
            FieldKey::Named(name) => Name::new(name).parameter(),
            FieldKey::Position(position) => Identifier::parse(format!("field{position}")),
            _ => Err(Error::UnsupportedTarget {
                target: KOTLIN_TARGET,
                shape: "unknown encoded field key",
            }),
        }
    }
}
