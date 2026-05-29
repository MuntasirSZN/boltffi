use boltffi_ast::MethodDef;

use crate::declared_types::DeclaredTypes;
use crate::marker::Marker;
use crate::type_expr::Scanner;
use crate::{ModulePath, ScanError};

use super::signature;

pub(super) fn scan(
    item: &syn::ItemImpl,
    parent: &str,
    module: &ModulePath,
    declared_types: &DeclaredTypes,
) -> Result<Vec<MethodDef>, ScanError> {
    let scanner = Scanner::new(declared_types, module);
    item.items
        .iter()
        .filter_map(|impl_item| scan_item(impl_item, parent, &scanner))
        .collect()
}

fn scan_item(
    item: &syn::ImplItem,
    parent: &str,
    scanner: &Scanner<'_>,
) -> Option<Result<MethodDef, ScanError>> {
    match item {
        syn::ImplItem::Fn(method) => match exported(&method.attrs, &method.vis, "method") {
            Ok(true) => Some(signature::method(&method.sig, parent, scanner)),
            Ok(false) => None,
            Err(error) => Some(Err(error)),
        },
        syn::ImplItem::Const(item) => non_method(
            &item.attrs,
            Some(&item.vis),
            parent,
            item.ident.to_string(),
            "associated const",
        ),
        syn::ImplItem::Type(item) => non_method(
            &item.attrs,
            Some(&item.vis),
            parent,
            item.ident.to_string(),
            "associated type",
        ),
        syn::ImplItem::Macro(item) => non_method(
            &item.attrs,
            None,
            parent,
            item.mac
                .path
                .segments
                .last()
                .map_or_else(|| "macro".to_owned(), |segment| segment.ident.to_string()),
            "macro",
        ),
        _ => Some(Err(ScanError::UnsupportedImplItem {
            item: format!("{parent}::item"),
        })),
    }
}

fn exported(
    attrs: &[syn::Attribute],
    visibility: &syn::Visibility,
    item: &str,
) -> Result<bool, ScanError> {
    match Marker::detect(attrs)? {
        Some(Marker::Skip) => Ok(false),
        Some(marker) => Err(ScanError::InvalidMarkerPlacement {
            marker: marker.as_str().to_owned(),
            item: item.to_owned(),
        }),
        None => Ok(matches!(visibility, syn::Visibility::Public(_))),
    }
}

fn non_method(
    attrs: &[syn::Attribute],
    visibility: Option<&syn::Visibility>,
    parent: &str,
    name: String,
    item: &str,
) -> Option<Result<MethodDef, ScanError>> {
    match Marker::detect(attrs) {
        Ok(Some(Marker::Skip)) => None,
        Ok(Some(marker)) => Some(Err(ScanError::InvalidMarkerPlacement {
            marker: marker.as_str().to_owned(),
            item: item.to_owned(),
        })),
        Ok(None)
            if visibility
                .is_some_and(|visibility| !matches!(visibility, syn::Visibility::Public(_))) =>
        {
            None
        }
        Ok(None) => Some(Err(ScanError::UnsupportedImplItem {
            item: format!("{parent}::{name}"),
        })),
        Err(error) => Some(Err(error)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ModulePath;
    use crate::declared_types::DeclaredTypes;
    use boltffi_ast::{
        CanonicalName, MethodDef, MethodId, NamePart, Primitive, Receiver, ReturnDef, TypeExpr,
    };

    fn parse(source: &str) -> syn::ItemImpl {
        syn::parse_str(source).expect("valid impl block")
    }

    fn name(parts: &[&str]) -> CanonicalName {
        CanonicalName::new(parts.iter().copied().map(NamePart::new).collect())
    }

    fn scan(
        source: &str,
        parent: &str,
        declared_types: &DeclaredTypes,
    ) -> Result<Vec<MethodDef>, ScanError> {
        super::scan(
            &parse(source),
            parent,
            &ModulePath::root("demo"),
            declared_types,
        )
    }

    #[test]
    fn scans_borrowing_method_with_resolved_param_and_return() {
        let mut declared_types = DeclaredTypes::new();
        declared_types.register_record(boltffi_ast::RecordId::new("demo::Point"));
        let methods = scan(
            "impl Point { pub fn distance(&self, other: Point) -> f64 { 0.0 } }",
            "demo::Point",
            &declared_types,
        )
        .expect("scan");

        assert_eq!(methods.len(), 1);
        assert_eq!(methods[0].id, MethodId::new("demo::Point::distance"));
        assert_eq!(methods[0].name, name(&["distance"]));
        assert_eq!(methods[0].receiver, Receiver::Shared);
        assert_eq!(
            methods[0].parameters[0].type_expr,
            TypeExpr::Record(boltffi_ast::RecordId::new("demo::Point"))
        );
        assert_eq!(
            methods[0].returns,
            ReturnDef::Value(TypeExpr::Primitive(Primitive::F64))
        );
    }

    #[test]
    fn associated_function_returning_self_has_no_receiver() {
        let methods = scan(
            "impl Point { pub fn origin() -> Self { todo!() } }",
            "demo::Point",
            &DeclaredTypes::new(),
        )
        .expect("scan");

        assert_eq!(methods[0].receiver, Receiver::None);
        assert_eq!(methods[0].returns, ReturnDef::Value(TypeExpr::SelfType));
    }

    #[test]
    fn captures_each_receiver_shape() {
        let methods = scan(
            "impl Point { \
                pub fn shared(&self) {} \
                pub fn exclusive(&mut self) {} \
                pub fn consuming(self) {} \
            }",
            "demo::Point",
            &DeclaredTypes::new(),
        )
        .expect("scan");

        assert_eq!(methods[0].receiver, Receiver::Shared);
        assert_eq!(methods[1].receiver, Receiver::Mutable);
        assert_eq!(methods[2].receiver, Receiver::Owned);
    }

    #[test]
    fn skips_private_and_explicitly_skipped_methods() {
        let methods = scan(
            "impl Point { \
                pub fn exported(&self) {} \
                fn helper(&self) {} \
                #[skip] pub fn skipped(&self) {} \
                #[boltffi::skip] pub fn also_skipped(&self) {} \
            }",
            "demo::Point",
            &DeclaredTypes::new(),
        )
        .expect("scan");

        assert_eq!(methods.len(), 1);
        assert_eq!(methods[0].name, name(&["exported"]));
    }

    #[test]
    fn rejects_malformed_skip_marker_on_method() {
        let error = scan(
            "impl Point { #[skip(reason)] pub fn hidden(&self) {} }",
            "demo::Point",
            &DeclaredTypes::new(),
        )
        .expect_err("malformed skip marker must reject");

        assert_eq!(
            error,
            ScanError::InvalidMarker {
                attribute: "skip(reason)".to_owned()
            }
        );
    }

    #[test]
    fn rejects_non_skip_boltffi_markers_on_methods() {
        let error = scan(
            "impl Point { #[export] pub fn hidden(&self) {} }",
            "demo::Point",
            &DeclaredTypes::new(),
        )
        .expect_err("method marker rejected");

        assert_eq!(
            error,
            ScanError::InvalidMarkerPlacement {
                marker: "export".to_owned(),
                item: "method".to_owned(),
            }
        );
    }

    #[test]
    fn rejects_public_non_method_items_before_dropping_them() {
        let associated_const = scan(
            "impl Point { pub const VERSION: u32 = 1; }",
            "demo::Point",
            &DeclaredTypes::new(),
        )
        .expect_err("associated const rejected");
        let associated_type = scan(
            "impl Point { pub type Value = u32; }",
            "demo::Point",
            &DeclaredTypes::new(),
        )
        .expect_err("associated type rejected");

        assert_eq!(
            associated_const,
            ScanError::UnsupportedImplItem {
                item: "demo::Point::VERSION".to_owned(),
            }
        );
        assert_eq!(
            associated_type,
            ScanError::UnsupportedImplItem {
                item: "demo::Point::Value".to_owned(),
            }
        );
    }

    #[test]
    fn ignores_private_and_skipped_non_method_items() {
        let methods = scan(
            "impl Point { \
                const PRIVATE: u32 = 1; \
                #[skip] pub const PUBLIC: u32 = 2; \
                pub fn exported(&self) {} \
            }",
            "demo::Point",
            &DeclaredTypes::new(),
        )
        .expect("scan");

        assert_eq!(methods.len(), 1);
        assert_eq!(methods[0].name, name(&["exported"]));
    }

    #[test]
    fn rejects_impl_macros_because_their_exports_are_not_syntactic() {
        let error = scan(
            "impl Point { exported_methods!(); }",
            "demo::Point",
            &DeclaredTypes::new(),
        )
        .expect_err("macro rejected");

        assert_eq!(
            error,
            ScanError::UnsupportedImplItem {
                item: "demo::Point::exported_methods".to_owned(),
            }
        );
    }
}
