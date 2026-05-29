use std::collections::HashMap;

use boltffi_ast::{ClassId, EnumId, RecordId, TraitId};

use crate::ScanError;
use crate::impl_target;
use crate::marked::MarkedItems;

#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) enum DeclaredType {
    Record(RecordId),
    Enum(EnumId),
    Trait(TraitId),
    Class(ClassId),
}

#[derive(Clone, Debug, Default)]
pub(super) struct DeclaredTypes {
    by_path: HashMap<String, DeclaredType>,
}

impl DeclaredTypes {
    #[cfg(test)]
    pub(super) fn new() -> Self {
        Self::default()
    }

    pub(super) fn index(marked: &MarkedItems<'_>) -> Result<Self, ScanError> {
        marked
            .records()
            .iter()
            .map(|marked| {
                Ok(DeclaredType::Record(RecordId::new(
                    marked.module().qualified(&marked.item().ident.to_string()),
                )))
            })
            .chain(marked.enums().iter().map(|marked| {
                Ok(DeclaredType::Enum(EnumId::new(
                    marked.module().qualified(&marked.item().ident.to_string()),
                )))
            }))
            .chain(marked.traits().iter().map(|marked| {
                Ok(DeclaredType::Trait(TraitId::new(
                    marked.module().qualified(&marked.item().ident.to_string()),
                )))
            }))
            .chain(marked.classes().iter().map(|marked| {
                impl_target::Target::class(marked.item()).and_then(|target| {
                    target
                        .resolve(marked.module())
                        .map(ClassId::new)
                        .map(DeclaredType::Class)
                        .ok_or_else(|| ScanError::UnsupportedClassImpl {
                            target: target.spelling().to_owned(),
                        })
                })
            }))
            .try_fold(Self::default(), |mut declared_types, declared_type| {
                declared_types.register(declared_type?)?;
                Ok(declared_types)
            })
    }

    #[cfg(test)]
    pub(super) fn register_record(&mut self, id: RecordId) {
        self.register(DeclaredType::Record(id))
            .expect("test declaration registration must not conflict");
    }

    #[cfg(test)]
    pub(super) fn register_enum(&mut self, id: EnumId) {
        self.register(DeclaredType::Enum(id))
            .expect("test declaration registration must not conflict");
    }

    #[cfg(test)]
    pub(super) fn register_trait(&mut self, id: TraitId) {
        self.register(DeclaredType::Trait(id))
            .expect("test declaration registration must not conflict");
    }

    #[cfg(test)]
    pub(super) fn register_class(&mut self, id: ClassId) {
        self.register(DeclaredType::Class(id))
            .expect("test declaration registration must not conflict");
    }

    pub(super) fn resolve(&self, path: &str) -> Option<&DeclaredType> {
        self.by_path.get(path)
    }

    fn register(&mut self, declared_type: DeclaredType) -> Result<(), ScanError> {
        let path = declared_type.path().to_owned();
        match self.by_path.get(&path) {
            Some(existing)
                if existing.kind() == declared_type.kind()
                    && declared_type.kind().allows_redeclaration() =>
            {
                Ok(())
            }
            Some(existing) => Err(ScanError::ConflictingDeclarations {
                path,
                first: existing.kind().as_str().to_owned(),
                second: declared_type.kind().as_str().to_owned(),
            }),
            None => {
                self.by_path.insert(path, declared_type);
                Ok(())
            }
        }
    }
}

impl DeclaredType {
    fn path(&self) -> &str {
        match self {
            Self::Record(id) => id.as_str(),
            Self::Enum(id) => id.as_str(),
            Self::Trait(id) => id.as_str(),
            Self::Class(id) => id.as_str(),
        }
    }

    fn kind(&self) -> DeclaredKind {
        match self {
            Self::Record(_) => DeclaredKind::Record,
            Self::Enum(_) => DeclaredKind::Enum,
            Self::Trait(_) => DeclaredKind::Trait,
            Self::Class(_) => DeclaredKind::Class,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum DeclaredKind {
    Record,
    Enum,
    Trait,
    Class,
}

impl DeclaredKind {
    const fn as_str(self) -> &'static str {
        match self {
            Self::Record => "record",
            Self::Enum => "enum",
            Self::Trait => "trait",
            Self::Class => "class",
        }
    }

    const fn allows_redeclaration(self) -> bool {
        matches!(self, Self::Class)
    }
}
