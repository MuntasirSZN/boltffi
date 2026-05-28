use boltffi_ast::{ClosureKind, ClosureType, Primitive, ReturnDef, TypeExpr};

use crate::ScanError;

pub(crate) fn scan_type(ty: &syn::Type) -> Result<TypeExpr, ScanError> {
    match ty {
        syn::Type::ImplTrait(impl_trait) => closure(impl_trait, ty),
        _ => primitive(ty)
            .map(TypeExpr::Primitive)
            .ok_or_else(|| ScanError::unsupported_type(ty)),
    }
}

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

fn primitive(ty: &syn::Type) -> Option<Primitive> {
    let syn::Type::Path(type_path) = ty else {
        return None;
    };
    if type_path.qself.is_some() {
        return None;
    }
    let ident = type_path.path.get_ident()?;
    Primitive::from_rust_name(&ident.to_string())
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
