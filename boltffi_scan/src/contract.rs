use boltffi_ast::{PackageInfo, RecordDef, RecordId, SourceContract};

use crate::registry::{DeclaredType, TypeRegistry};
use crate::{ModulePath, ScanError, function, methods, record};

/// Scans a parsed source file into a [`SourceContract`].
///
/// Runs two passes so references resolve regardless of declaration
/// order: the first registers every declared type, the second scans each
/// declaration against that registry and attaches `impl` methods to their
/// record. This interim entry treats every top-level `struct` as a record
/// and every `fn` as a free function, and attaches the methods of any
/// `impl` on a known record. Attribute routing (`#[data]`, `#[export]`),
/// classes, enums, and the remaining declaration kinds arrive with their
/// slices; other item kinds (modules, uses, trait impls on unknown types)
/// are left for them.
pub fn scan_contract(
    file: &syn::File,
    package: PackageInfo,
    module: &ModulePath,
) -> Result<SourceContract, ScanError> {
    let registry = collect_types(file, module);
    let mut records = file
        .items
        .iter()
        .filter_map(|item| match item {
            syn::Item::Struct(item) => Some(record::scan_struct(item, module, &registry)),
            _ => None,
        })
        .collect::<Result<Vec<_>, _>>()?;
    attach_methods(file, &registry, &mut records)?;

    let functions = file
        .items
        .iter()
        .filter_map(|item| match item {
            syn::Item::Fn(item) => Some(function::scan_function(item, module, &registry)),
            _ => None,
        })
        .collect::<Result<Vec<_>, _>>()?;

    let mut contract = SourceContract::new(package);
    contract.records = records;
    contract.functions = functions;
    Ok(contract)
}

fn collect_types(file: &syn::File, module: &ModulePath) -> TypeRegistry {
    file.items
        .iter()
        .filter_map(|item| match item {
            syn::Item::Struct(item) => Some(item),
            _ => None,
        })
        .fold(TypeRegistry::new(), |mut registry, item| {
            let id = RecordId::new(module.qualified(&item.ident.to_string()));
            registry.register_record(item.ident.to_string(), id);
            registry
        })
}

fn attach_methods(
    file: &syn::File,
    registry: &TypeRegistry,
    records: &mut [RecordDef],
) -> Result<(), ScanError> {
    for item in &file.items {
        let syn::Item::Impl(item_impl) = item else {
            continue;
        };
        let Some(self_ident) = self_type_ident(item_impl) else {
            continue;
        };
        let Some(DeclaredType::Record(id)) = registry.resolve(&self_ident.to_string()) else {
            continue;
        };
        let scanned = methods::scan(item_impl, id.as_str(), registry)?;
        if let Some(record) = records.iter_mut().find(|record| &record.id == id) {
            record.methods.extend(scanned);
        }
    }
    Ok(())
}

fn self_type_ident(item: &syn::ItemImpl) -> Option<&syn::Ident> {
    let syn::Type::Path(type_path) = item.self_ty.as_ref() else {
        return None;
    };
    type_path.path.segments.last().map(|segment| &segment.ident)
}

#[cfg(test)]
mod tests {
    use super::*;
    use boltffi_ast::{Primitive, Receiver, ReturnDef, TypeExpr};

    fn parse(source: &str) -> syn::File {
        syn::parse_str(source).expect("valid source file")
    }

    fn scan(source: &str) -> SourceContract {
        scan_contract(
            &parse(source),
            PackageInfo::new("demo", None),
            &ModulePath::root("demo"),
        )
        .expect("scan")
    }

    fn point(contract: &SourceContract) -> &RecordDef {
        contract
            .records
            .iter()
            .find(|record| record.id == RecordId::new("demo::Point"))
            .expect("Point record")
    }

    #[test]
    fn resolves_record_reference_regardless_of_declaration_order() {
        let contract =
            scan("pub struct Shape { pub center: Point } pub struct Point { pub x: f64 }");

        assert_eq!(contract.records.len(), 2);
        let shape = contract
            .records
            .iter()
            .find(|record| record.id == RecordId::new("demo::Shape"))
            .expect("Shape record");
        assert_eq!(
            shape.fields[0].type_expr,
            TypeExpr::Record(RecordId::new("demo::Point"))
        );
    }

    #[test]
    fn scans_functions_and_resolves_their_record_references() {
        let contract = scan("pub struct Point { pub x: f64 } pub fn origin() -> Point { todo!() }");

        assert_eq!(contract.functions.len(), 1);
        assert_eq!(
            contract.functions[0].returns,
            ReturnDef::Value(TypeExpr::Record(RecordId::new("demo::Point")))
        );
    }

    #[test]
    fn ignores_item_kinds_without_a_scan_slice() {
        let contract = scan("use std::collections::HashMap; pub struct Point { pub x: f64 }");

        assert_eq!(contract.records.len(), 1);
        assert!(contract.functions.is_empty());
    }

    #[test]
    fn attaches_impl_methods_to_their_record() {
        let contract = scan(
            "pub struct Point { pub x: f64, pub y: f64 } \
             impl Point { \
                 pub fn origin() -> Self { todo!() } \
                 pub fn distance(&self, other: Point) -> f64 { 0.0 } \
             }",
        );
        let point = point(&contract);

        assert_eq!(point.methods.len(), 2);
        assert_eq!(point.methods[0].receiver, Receiver::None);
        assert_eq!(
            point.methods[0].returns,
            ReturnDef::Value(TypeExpr::SelfType)
        );
        assert_eq!(point.methods[1].receiver, Receiver::Shared);
        assert_eq!(
            point.methods[1].parameters[0].type_expr,
            TypeExpr::Record(RecordId::new("demo::Point"))
        );
        assert_eq!(
            point.methods[1].returns,
            ReturnDef::Value(TypeExpr::Primitive(Primitive::F64))
        );
    }
}
