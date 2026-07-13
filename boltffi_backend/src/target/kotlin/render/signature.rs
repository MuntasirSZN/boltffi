use crate::{
    core::{Error, Result},
    target::kotlin::syntax::{Identifier, TypeName},
};

/// Rejects exported members that would duplicate a declaration emitted by the
/// class or callback-handle template (`close()`, `boltffiHandle()`, ...),
/// since the generated Kotlin would not compile. Only zero-parameter members
/// collide -- overloads that take parameters are legal Kotlin, so callers pass
/// the names of their zero-parameter members only.
pub fn validate_reserved_members<'a>(
    scope: &TypeName,
    reserved: &[&str],
    zero_parameter_members: impl IntoIterator<Item = &'a Identifier>,
) -> Result<()> {
    zero_parameter_members
        .into_iter()
        .find(|name| reserved.contains(&name.as_str()))
        .map_or(Ok(()), |name| {
            Err(Error::KotlinNameCollision {
                scope: scope.to_string(),
                name: format!("{name}()"),
            })
        })
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Parameter {
    name: Identifier,
    ty: TypeName,
}

impl Parameter {
    pub fn new(name: Identifier, ty: TypeName) -> Self {
        Self { name, ty }
    }

    pub fn name(&self) -> &Identifier {
        &self.name
    }

    pub fn ty(&self) -> &TypeName {
        &self.ty
    }
}
