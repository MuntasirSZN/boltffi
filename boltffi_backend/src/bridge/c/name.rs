use boltffi_binding::{CanonicalName, FieldKey, NamePart};

use crate::core::{Error, Result};

#[derive(Clone, Copy, Debug)]
pub struct Spelling<'name> {
    name: &'name CanonicalName,
}

impl<'name> Spelling<'name> {
    pub fn new(name: &'name CanonicalName) -> Self {
        Self { name }
    }

    pub fn parameter(self) -> String {
        self.join("_", |part| part.as_str().to_owned())
    }

    pub fn typedef(self) -> String {
        format!("___{}", self.join("", capitalized))
    }

    pub fn constant(self) -> String {
        self.join("_", uppercased)
    }

    fn join(self, separator: &str, transform: impl Fn(&NamePart) -> String) -> String {
        self.name
            .parts()
            .iter()
            .map(transform)
            .collect::<Vec<_>>()
            .join(separator)
    }
}

pub struct Field<'field>(&'field FieldKey);

impl<'field> Field<'field> {
    pub fn new(field: &'field FieldKey) -> Self {
        Self(field)
    }

    pub fn spelling(&self) -> Result<String> {
        match self.0 {
            FieldKey::Named(name) => Ok(Spelling::new(name).parameter()),
            FieldKey::Position(position) => Ok(format!("field_{position}")),
            _ => Err(Error::UnsupportedCAbi {
                shape: "unknown field key",
            }),
        }
    }
}

pub struct EnumConstant<'name> {
    owner: &'name CanonicalName,
    variant: &'name CanonicalName,
}

impl<'name> EnumConstant<'name> {
    pub fn new(owner: &'name CanonicalName, variant: &'name CanonicalName) -> Self {
        Self { owner, variant }
    }

    pub fn spelling(&self) -> String {
        [
            Spelling::new(self.owner).constant(),
            Spelling::new(self.variant).constant(),
        ]
        .join("_")
    }
}

fn capitalized(part: &NamePart) -> String {
    let mut characters = part.as_str().chars();
    characters.next().map_or_else(String::new, |first| {
        first.to_uppercase().chain(characters).collect::<String>()
    })
}

fn uppercased(part: &NamePart) -> String {
    part.as_str().to_ascii_uppercase()
}
