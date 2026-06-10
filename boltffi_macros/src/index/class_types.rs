use std::collections::{HashMap, HashSet};

use syn::{Attribute, Item, ItemImpl, Type};

use crate::index::type_paths::TypePathKey;
use crate::index::{CrateIndex, PathResolver, SourceModule};

#[derive(Default, Clone)]
pub struct ClassTypeRegistry {
    paths: HashSet<Vec<String>>,
    unique_names: HashSet<String>,
    path_resolver: PathResolver,
}

impl ClassTypeRegistry {
    fn insert(&mut self, qualified_path: Vec<String>) {
        self.paths.insert(qualified_path);
    }

    fn finalize_unique_names(&mut self) {
        let name_counts = self.paths.iter().fold(
            HashMap::<String, usize>::new(),
            |mut counts, qualified_path| {
                if let Some(name) = qualified_path.last() {
                    *counts.entry(name.clone()).or_insert(0) += 1;
                }
                counts
            },
        );

        self.unique_names = self
            .paths
            .iter()
            .filter_map(|qualified_path| {
                qualified_path
                    .last()
                    .filter(|name| name_counts.get(*name).copied() == Some(1))
                    .cloned()
            })
            .collect();
    }

    pub fn contains(&self, ty: &Type) -> bool {
        self.type_path_key(ty).is_some_and(|type_path_key| {
            if type_path_key.is_single_segment() {
                return type_path_key
                    .first_segment()
                    .is_some_and(|name| self.unique_names.contains(name));
            }

            self.paths
                .iter()
                .any(|registered_path| registered_path.as_slice() == type_path_key.segments())
                || self
                    .paths
                    .iter()
                    .any(|registered_path| type_path_key.has_suffix(registered_path))
        })
    }

    fn type_path_key(&self, ty: &Type) -> Option<TypePathKey> {
        match ty {
            Type::Path(type_path) if type_path.qself.is_none() => {
                let resolved_path = self.path_resolver.resolve(&type_path.path).into_path();
                Some(TypePathKey::from_path(&resolved_path))
            }
            Type::Group(group) => self.type_path_key(group.elem.as_ref()),
            Type::Paren(paren) => self.type_path_key(paren.elem.as_ref()),
            _ => None,
        }
    }
}

#[cfg(test)]
impl ClassTypeRegistry {
    pub fn with_entries(entries: &[&str]) -> Self {
        let mut registry = Self::default();
        entries
            .iter()
            .map(|entry| entry.split("::").map(str::to_string).collect::<Vec<_>>())
            .for_each(|segments| registry.insert(segments));
        registry.finalize_unique_names();
        registry
    }

    pub fn with_entries_and_use_aliases(entries: &[&str], aliases: &[(&str, &str)]) -> Self {
        let mut registry = Self::with_entries(entries);
        registry.path_resolver = PathResolver::with_use_aliases(aliases);
        registry
    }
}

pub fn registry_for_current_crate() -> syn::Result<ClassTypeRegistry> {
    Ok(CrateIndex::for_current_crate()?.class_types().clone())
}

pub fn build_class_type_registry(
    source_modules: &[SourceModule],
    path_resolver: PathResolver,
) -> syn::Result<ClassTypeRegistry> {
    let mut registry = ClassTypeRegistry {
        path_resolver,
        ..ClassTypeRegistry::default()
    };
    collect_root_types(source_modules, &mut registry)?;
    registry.finalize_unique_names();
    Ok(registry)
}

fn collect_root_types(
    source_modules: &[SourceModule],
    registry: &mut ClassTypeRegistry,
) -> syn::Result<()> {
    source_modules.iter().try_for_each(|source_module| {
        let mut collector = ClassTypeCollector {
            module_path: source_module.module_path().clone().into_strings(),
            registry,
        };
        source_module
            .syntax()
            .items
            .iter()
            .try_for_each(|item| collector.collect_item(item))
    })
}

struct ClassTypeCollector<'a> {
    module_path: Vec<String>,
    registry: &'a mut ClassTypeRegistry,
}

impl<'a> ClassTypeCollector<'a> {
    fn collect_item(&mut self, item: &Item) -> syn::Result<()> {
        match item {
            Item::Impl(item_impl) => {
                self.collect_impl(item_impl);
                Ok(())
            }
            Item::Mod(item_mod) => {
                let Some((_, items)) = &item_mod.content else {
                    return Ok(());
                };
                self.module_path.push(item_mod.ident.to_string());
                let collect_result = items
                    .iter()
                    .try_for_each(|nested| self.collect_item(nested));
                self.module_path.pop();
                collect_result
            }
            _ => Ok(()),
        }
    }

    fn collect_impl(&mut self, item_impl: &ItemImpl) {
        if !Self::is_class_export_impl(item_impl) {
            return;
        }

        let Some(type_path_key) = TypePathKey::from_type(item_impl.self_ty.as_ref()) else {
            return;
        };

        self.registry
            .insert(self.qualified_class_path(type_path_key));
    }

    fn qualified_class_path(&self, type_path_key: TypePathKey) -> Vec<String> {
        let segments = type_path_key.into_segments();
        if segments.len() == 1 {
            return self.module_path.iter().cloned().chain(segments).collect();
        }
        segments
    }

    fn is_class_export_impl(item_impl: &ItemImpl) -> bool {
        Self::has_boltffi_attribute(&item_impl.attrs, "export")
    }

    fn has_boltffi_attribute(attributes: &[Attribute], name: &str) -> bool {
        attributes
            .iter()
            .any(|attribute| Self::is_boltffi_attribute(attribute, name))
    }

    fn is_boltffi_attribute(attribute: &Attribute, name: &str) -> bool {
        let path = attribute.path();
        if path.is_ident(name) {
            return true;
        }

        path.segments.len() == 2
            && path
                .segments
                .first()
                .is_some_and(|segment| segment.ident == "boltffi")
            && path
                .segments
                .last()
                .is_some_and(|segment| segment.ident == name)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use syn::parse_quote;

    #[test]
    fn renamed_import_resolves_to_registered_class_path() {
        let registry = ClassTypeRegistry::with_entries_and_use_aliases(
            &["map::Marker"],
            &[("Pin", "crate::map::Marker")],
        );

        assert!(registry.contains(&parse_quote!(Pin)));
    }

    #[test]
    fn unrelated_namespaced_export_attribute_is_ignored() {
        let item_impl: ItemImpl = syn::parse_quote! {
            #[other::export]
            impl Marker {}
        };
        let mut registry = ClassTypeRegistry::default();
        let mut collector = ClassTypeCollector {
            module_path: vec!["map".to_string()],
            registry: &mut registry,
        };

        collector.collect_impl(&item_impl);
        collector.registry.finalize_unique_names();

        assert!(!collector.registry.contains(&parse_quote!(Marker)));
    }
}
