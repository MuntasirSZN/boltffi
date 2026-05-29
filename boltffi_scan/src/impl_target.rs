use crate::{ModulePath, ScanError, spelling};

pub(super) struct Target<'source> {
    path: Option<&'source syn::Path>,
    spelling: String,
}

impl<'source> Target<'source> {
    pub(super) fn scan(item: &'source syn::ItemImpl) -> Self {
        let path = match item.self_ty.as_ref() {
            syn::Type::Path(type_path) => Some(&type_path.path),
            _ => None,
        };
        Self {
            path,
            spelling: spelling::ty(&item.self_ty),
        }
    }

    pub(super) fn class(item: &'source syn::ItemImpl) -> Result<Self, ScanError> {
        let target = Self::scan(item);
        if !item.generics.params.is_empty() || item.generics.where_clause.is_some() {
            return Err(ScanError::UnsupportedGenerics {
                item: format!("class {}", target.spelling()),
            });
        }
        if item.trait_.is_some() {
            return Err(ScanError::UnsupportedClassImplShape {
                target: target.spelling().to_owned(),
            });
        }
        Ok(target)
    }

    pub(super) fn resolve(&self, module: &ModulePath) -> Option<String> {
        self.path
            .filter(|path| {
                path.segments
                    .iter()
                    .all(|segment| segment.arguments.is_empty())
            })
            .and_then(|path| module.resolve(path))
    }

    pub(super) fn ident(&self) -> Option<&syn::Ident> {
        self.path
            .and_then(|path| path.segments.last())
            .filter(|segment| segment.arguments.is_empty())
            .map(|segment| &segment.ident)
    }

    pub(super) fn spelling(&self) -> &str {
        &self.spelling
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(source: &str) -> syn::ItemImpl {
        syn::parse_str(source).expect("impl block")
    }

    #[test]
    fn resolves_plain_impl_targets_from_module_context() {
        let module = ModulePath::root("demo").child("engine");
        let item = parse("impl Runtime {}");
        let target = Target::scan(&item);

        assert_eq!(
            target.resolve(&module),
            Some("demo::engine::Runtime".to_owned())
        );
        assert_eq!(
            target.ident().map(ToString::to_string),
            Some("Runtime".to_owned())
        );
        assert_eq!(target.spelling(), "Runtime");
    }

    #[test]
    fn rejects_targets_that_would_erase_type_arguments() {
        let module = ModulePath::root("demo");
        let item = parse("impl Runtime<u32> {}");
        let target = Target::scan(&item);

        assert_eq!(target.resolve(&module), None);
        assert_eq!(target.ident(), None);
        assert_eq!(target.spelling(), "Runtime<u32>");
    }

    #[test]
    fn rejects_non_path_targets() {
        let module = ModulePath::root("demo");
        let item = parse("impl (Runtime, State) {}");
        let target = Target::scan(&item);

        assert_eq!(target.resolve(&module), None);
        assert_eq!(target.ident(), None);
        assert_eq!(target.spelling(), "(Runtime, State)");
    }
}
