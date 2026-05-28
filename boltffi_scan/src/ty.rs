use boltffi_ast::{ClosureKind, ClosureType, Primitive, ReturnDef, TypeExpr};

use crate::ScanError;

/// Scans a source type into a [`TypeExpr`].
///
/// Handles primitives, `String`, the standard containers (`Vec`,
/// `Option`, `Result`, `HashMap`/`BTreeMap`), tuples, and inline
/// `impl Fn`-family closures. Named user types still reject until the
/// type registry lands; references, bare function pointers, and smart
/// pointers arrive in later slices. An unrecognized type is rejected with
/// its verbatim spelling rather than dropped.
pub(crate) fn scan_type(ty: &syn::Type) -> Result<TypeExpr, ScanError> {
    match ty {
        syn::Type::ImplTrait(impl_trait) => closure(impl_trait, ty),
        syn::Type::Tuple(tuple) => tuple_type(tuple),
        syn::Type::Path(type_path) => path_type(type_path, ty),
        _ => Err(ScanError::unsupported_type(ty)),
    }
}

/// Scans a callable return position into a [`ReturnDef`].
///
/// Free functions and inline closure signatures share this, so the
/// void-versus-value decision lives in one place. An explicit `-> ()`
/// records as [`ReturnDef::Void`], the same as a missing return.
pub(crate) fn scan_return(output: &syn::ReturnType) -> Result<ReturnDef, ScanError> {
    match output {
        syn::ReturnType::Default => Ok(ReturnDef::Void),
        syn::ReturnType::Type(_, ty) if is_unit(ty) => Ok(ReturnDef::Void),
        syn::ReturnType::Type(_, ty) => Ok(ReturnDef::Value(scan_type(ty)?)),
    }
}

fn is_unit(ty: &syn::Type) -> bool {
    matches!(ty, syn::Type::Tuple(tuple) if tuple.elems.is_empty())
}

fn path_type(type_path: &syn::TypePath, source: &syn::Type) -> Result<TypeExpr, ScanError> {
    if type_path.qself.is_some() {
        return Err(ScanError::unsupported_type(source));
    }
    let segment = type_path
        .path
        .segments
        .last()
        .ok_or_else(|| ScanError::unsupported_type(source))?;
    match segment.ident.to_string().as_str() {
        "String" => Ok(TypeExpr::String),
        "Vec" => Ok(TypeExpr::vec(single_argument(segment, source)?)),
        "Option" => Ok(TypeExpr::option(single_argument(segment, source)?)),
        "Result" => {
            let (ok, err) = two_arguments(segment, source)?;
            Ok(TypeExpr::result(ok, err))
        }
        "HashMap" | "BTreeMap" => {
            let (key, value) = two_arguments(segment, source)?;
            Ok(TypeExpr::map(key, value))
        }
        name => Primitive::from_rust_name(name)
            .map(TypeExpr::Primitive)
            .ok_or_else(|| ScanError::unsupported_type(source)),
    }
}

fn tuple_type(tuple: &syn::TypeTuple) -> Result<TypeExpr, ScanError> {
    if tuple.elems.is_empty() {
        return Ok(TypeExpr::Unit);
    }
    let elements = tuple
        .elems
        .iter()
        .map(scan_type)
        .collect::<Result<Vec<_>, _>>()?;
    Ok(TypeExpr::tuple(elements))
}

fn type_arguments(segment: &syn::PathSegment) -> Vec<&syn::Type> {
    match &segment.arguments {
        syn::PathArguments::AngleBracketed(bracketed) => bracketed
            .args
            .iter()
            .filter_map(|argument| match argument {
                syn::GenericArgument::Type(ty) => Some(ty),
                _ => None,
            })
            .collect(),
        _ => Vec::new(),
    }
}

fn single_argument(segment: &syn::PathSegment, source: &syn::Type) -> Result<TypeExpr, ScanError> {
    match type_arguments(segment).as_slice() {
        [argument] => scan_type(argument),
        _ => Err(ScanError::unsupported_type(source)),
    }
}

fn two_arguments(
    segment: &syn::PathSegment,
    source: &syn::Type,
) -> Result<(TypeExpr, TypeExpr), ScanError> {
    match type_arguments(segment).as_slice() {
        [first, second] => Ok((scan_type(first)?, scan_type(second)?)),
        _ => Err(ScanError::unsupported_type(source)),
    }
}

fn closure(impl_trait: &syn::TypeImplTrait, source: &syn::Type) -> Result<TypeExpr, ScanError> {
    let (kind, arguments) = impl_trait
        .bounds
        .iter()
        .find_map(|bound| match bound {
            syn::TypeParamBound::Trait(trait_bound) => closure_bound(trait_bound),
            _ => None,
        })
        .ok_or_else(|| ScanError::unsupported_type(source))?;
    let parameters = arguments
        .inputs
        .iter()
        .map(scan_type)
        .collect::<Result<Vec<_>, _>>()?;
    let returns = scan_return(&arguments.output)?;
    Ok(TypeExpr::closure(ClosureType::new(
        kind, parameters, returns,
    )))
}

fn closure_bound(
    bound: &syn::TraitBound,
) -> Option<(ClosureKind, &syn::ParenthesizedGenericArguments)> {
    let segment = bound.path.segments.last()?;
    let kind = closure_kind(&segment.ident.to_string())?;
    let syn::PathArguments::Parenthesized(arguments) = &segment.arguments else {
        return None;
    };
    Some((kind, arguments))
}

fn closure_kind(name: &str) -> Option<ClosureKind> {
    Some(match name {
        "Fn" => ClosureKind::Fn,
        "FnMut" => ClosureKind::FnMut,
        "FnOnce" => ClosureKind::FnOnce,
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ty(source: &str) -> syn::Type {
        syn::parse_str(source).expect("valid type")
    }

    #[test]
    fn scans_every_primitive_type_exactly() {
        [
            ("bool", Primitive::Bool),
            ("i8", Primitive::I8),
            ("u8", Primitive::U8),
            ("i16", Primitive::I16),
            ("u16", Primitive::U16),
            ("i32", Primitive::I32),
            ("u32", Primitive::U32),
            ("i64", Primitive::I64),
            ("u64", Primitive::U64),
            ("isize", Primitive::ISize),
            ("usize", Primitive::USize),
            ("f32", Primitive::F32),
            ("f64", Primitive::F64),
        ]
        .into_iter()
        .for_each(|(source, primitive)| {
            assert_eq!(
                scan_type(&ty(source)),
                Ok(TypeExpr::Primitive(primitive)),
                "primitive {source} should scan exactly"
            );
        });
    }

    #[test]
    fn scans_string_and_sequence_containers() {
        assert_eq!(scan_type(&ty("String")), Ok(TypeExpr::String));
        assert_eq!(
            scan_type(&ty("Vec<i32>")),
            Ok(TypeExpr::vec(TypeExpr::Primitive(Primitive::I32)))
        );
        assert_eq!(
            scan_type(&ty("Option<String>")),
            Ok(TypeExpr::option(TypeExpr::String))
        );
    }

    #[test]
    fn scans_result_and_map_containers() {
        assert_eq!(
            scan_type(&ty("Result<i32, String>")),
            Ok(TypeExpr::result(
                TypeExpr::Primitive(Primitive::I32),
                TypeExpr::String
            ))
        );
        assert_eq!(
            scan_type(&ty("HashMap<String, i32>")),
            Ok(TypeExpr::map(
                TypeExpr::String,
                TypeExpr::Primitive(Primitive::I32)
            ))
        );
    }

    #[test]
    fn scans_tuples_and_unit() {
        assert_eq!(
            scan_type(&ty("(i32, String)")),
            Ok(TypeExpr::tuple(vec![
                TypeExpr::Primitive(Primitive::I32),
                TypeExpr::String
            ]))
        );
        assert_eq!(scan_type(&ty("()")), Ok(TypeExpr::Unit));
    }

    #[test]
    fn scans_nested_containers() {
        assert_eq!(
            scan_type(&ty("Option<Vec<i32>>")),
            Ok(TypeExpr::option(TypeExpr::vec(TypeExpr::Primitive(
                Primitive::I32
            ))))
        );
    }

    #[test]
    fn resolves_qualified_std_paths_by_last_segment() {
        assert_eq!(scan_type(&ty("std::string::String")), Ok(TypeExpr::String));
        assert_eq!(
            scan_type(&ty("std::vec::Vec<u8>")),
            Ok(TypeExpr::vec(TypeExpr::Primitive(Primitive::U8)))
        );
    }

    #[test]
    fn named_user_type_rejects_until_registry_lands() {
        assert!(matches!(
            scan_type(&ty("Point")),
            Err(ScanError::UnsupportedType { spelling }) if spelling == "Point"
        ));
    }

    #[test]
    fn impl_trait_closure_can_follow_marker_bounds() {
        let TypeExpr::Closure {
            signature,
            presence,
        } = scan_type(&ty("impl Send + Fn(u32) -> u32")).expect("scan")
        else {
            panic!("expected closure");
        };

        assert_eq!(presence, boltffi_ast::HandlePresence::Required);
        assert_eq!(signature.kind, ClosureKind::Fn);
        assert_eq!(
            signature.parameters,
            vec![TypeExpr::Primitive(Primitive::U32)]
        );
        assert_eq!(
            signature.returns,
            ReturnDef::Value(TypeExpr::Primitive(Primitive::U32))
        );
    }

    #[test]
    fn impl_trait_without_fn_bound_is_rejected() {
        assert!(matches!(
            scan_type(&ty("impl Iterator<Item = u32>")),
            Err(ScanError::UnsupportedType { spelling }) if spelling == "unrecognized type"
        ));
    }

    #[test]
    fn closure_with_unsupported_argument_reports_that_argument() {
        assert!(matches!(
            scan_type(&ty("impl Fn(Point)")),
            Err(ScanError::UnsupportedType { spelling }) if spelling == "Point"
        ));
    }
}
