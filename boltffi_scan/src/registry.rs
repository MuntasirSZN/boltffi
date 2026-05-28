use std::collections::HashMap;

use boltffi_ast::RecordId;

/// A declared type a source reference can resolve to.
///
/// The scanner records every exported type's identity in a first pass so
/// a later reference such as `other: Point` resolves to the right
/// [`TypeExpr`](boltffi_ast::TypeExpr) variant. Only records exist today;
/// enums, classes, callbacks, and custom types add variants as their
/// scan slices land.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum DeclaredType {
    /// A record declared in the contract.
    Record(RecordId),
}

/// Maps an exported type's source name to its declared identity.
///
/// Keys are leaf identifiers (`Point`), matching how references are
/// written in signatures. Fully-qualified path resolution is a later
/// refinement; today two types that share a leaf name in different
/// modules collide, last registration winning.
#[derive(Clone, Debug, Default)]
pub(crate) struct TypeRegistry {
    by_name: HashMap<String, DeclaredType>,
}

impl TypeRegistry {
    pub(crate) fn new() -> Self {
        Self::default()
    }

    pub(crate) fn register_record(&mut self, name: impl Into<String>, id: RecordId) {
        self.by_name.insert(name.into(), DeclaredType::Record(id));
    }

    pub(crate) fn resolve(&self, name: &str) -> Option<&DeclaredType> {
        self.by_name.get(name)
    }
}
