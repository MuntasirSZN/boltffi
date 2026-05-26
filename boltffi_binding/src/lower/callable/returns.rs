use boltffi_ast::{ReturnDef, TypeExpr};

use crate::{
    ElementMeta, ErrorDecl, HandlePresence, HandleTarget, LiftPlan, Primitive, ReadPlan,
    ReturnDecl, TypeRef, ValueRef,
};

use super::super::{
    LowerError, codecs, enums, error::UnsupportedType, ids::DeclarationIds, index::Index, records,
    surface::SurfaceLower, types,
};

use super::{CallableOwner, substitute_self_type};

/// Lowers a source [`ReturnDef`] into the IR pair the surrounding
/// [`CallableDecl`] records: a [`ReturnDecl<S>`] for the success
/// value and an [`ErrorDecl<S>`] for the failure channel.
///
/// [`CallableDecl`]: crate::CallableDecl
pub(super) fn lower<S: SurfaceLower>(
    idx: &Index<'_>,
    ids: &DeclarationIds,
    owner: CallableOwner<'_>,
    return_def: &ReturnDef,
) -> Result<(ReturnDecl<S>, ErrorDecl<S>), LowerError> {
    match return_def {
        ReturnDef::Void => Ok((
            ReturnDecl::new(ElementMeta::new(None, None, None), LiftPlan::Void),
            ErrorDecl::none(),
        )),
        ReturnDef::Value(type_expr) => {
            if let TypeExpr::Result { ok, err } = type_expr {
                let ok_type_expr = substitute_self_type(owner, ok)?;
                let err_type_expr = substitute_self_type(owner, err)?;
                let lift = lower_lift::<S>(idx, ids, &ok_type_expr)?.into_out();
                let error = lower_error::<S>(idx, ids, &err_type_expr)?;
                return Ok((
                    ReturnDecl::new(ElementMeta::new(None, None, None), lift),
                    error,
                ));
            }
            if matches!(type_expr, TypeExpr::Unit) {
                return Err(LowerError::unsupported_type(
                    UnsupportedType::UnitInValuePosition,
                ));
            }
            let type_expr = substitute_self_type(owner, type_expr)?;
            let lift = lower_plain_lift::<S>(idx, ids, &type_expr)?;
            Ok((
                ReturnDecl::new(ElementMeta::new(None, None, None), lift),
                ErrorDecl::none(),
            ))
        }
    }
}

fn lower_plain_lift<S: SurfaceLower>(
    idx: &Index<'_>,
    ids: &DeclarationIds,
    type_expr: &TypeExpr,
) -> Result<LiftPlan<S>, LowerError> {
    match specialize_return::<S>(idx, ids, type_expr)? {
        Some(lift) => Ok(lift),
        None => lower_lift::<S>(idx, ids, type_expr),
    }
}

fn specialize_return<S: SurfaceLower>(
    idx: &Index<'_>,
    ids: &DeclarationIds,
    type_expr: &TypeExpr,
) -> Result<Option<LiftPlan<S>>, LowerError> {
    Ok(match type_expr {
        TypeExpr::Option(inner) => {
            primitive(inner).map(|primitive| LiftPlan::ScalarOption { primitive })
        }
        TypeExpr::Vec(inner) => {
            direct_vec_element(idx, ids, inner)?.map(|element| LiftPlan::DirectVec { element })
        }
        _ => None,
    })
}

fn primitive(type_expr: &TypeExpr) -> Option<Primitive> {
    if let TypeExpr::Primitive(p) = type_expr {
        Some(Primitive::from(*p))
    } else {
        None
    }
}

fn direct_vec_element(
    idx: &Index<'_>,
    ids: &DeclarationIds,
    type_expr: &TypeExpr,
) -> Result<Option<TypeRef>, LowerError> {
    match type_expr {
        TypeExpr::Primitive(_) => Ok(Some(types::lower(ids, type_expr)?)),
        TypeExpr::Record(id) if idx.record(id).is_some_and(records::is_direct) => {
            Ok(Some(types::lower(ids, type_expr)?))
        }
        _ => Ok(None),
    }
}

fn lower_error<S: SurfaceLower>(
    idx: &Index<'_>,
    ids: &DeclarationIds,
    type_expr: &TypeExpr,
) -> Result<ErrorDecl<S>, LowerError> {
    Ok(ErrorDecl::EncodedReturn {
        ty: types::lower(ids, type_expr)?,
        read: ReadPlan::new(codecs::node(idx, ids, type_expr, ValueRef::self_value())?),
        shape: S::encoded_return_shape(),
    })
}

/// Picks the [`LiftPlan`] for one return value from its source type.
///
/// Mirrors the parameter-side dispatch but emits lift-side IR
/// variants. Out-pointer variants activate when a `Result<T, E>`
/// return gives the native return slot to the error channel.
///
/// [`TypeExpr::Unit`] lowers to [`LiftPlan::Void`] so that
/// `Result<(), E>` produces a void success channel paired with the
/// error channel without routing an empty value through the codec lane.
fn lower_lift<S: SurfaceLower>(
    idx: &Index<'_>,
    ids: &DeclarationIds,
    type_expr: &TypeExpr,
) -> Result<LiftPlan<S>, LowerError> {
    match type_expr {
        TypeExpr::Unit => Ok(LiftPlan::Void),
        TypeExpr::Primitive(_) => Ok(LiftPlan::Direct {
            ty: types::lower(ids, type_expr)?,
        }),
        TypeExpr::Record(id) if idx.record(id).is_some_and(records::is_direct) => {
            Ok(LiftPlan::Direct {
                ty: types::lower(ids, type_expr)?,
            })
        }
        TypeExpr::Enum(id) if idx.enumeration(id).is_some_and(enums::is_c_style) => {
            Ok(LiftPlan::Direct {
                ty: types::lower(ids, type_expr)?,
            })
        }
        TypeExpr::String
        | TypeExpr::Bytes
        | TypeExpr::Record(_)
        | TypeExpr::Enum(_)
        | TypeExpr::Vec(_)
        | TypeExpr::Option(_)
        | TypeExpr::Tuple(_)
        | TypeExpr::Result { .. }
        | TypeExpr::Map { .. } => {
            let ty = types::lower(ids, type_expr)?;
            let codec = codecs::node(idx, ids, type_expr, ValueRef::self_value())?;
            Ok(LiftPlan::Encoded {
                ty,
                read: ReadPlan::new(codec),
                shape: S::encoded_return_shape(),
            })
        }
        TypeExpr::Closure(closure) => Ok(LiftPlan::Handle {
            target: HandleTarget::Closure(Box::new(types::lower_closure(ids, closure)?)),
            carrier: S::closure_handle_carrier(),
            presence: HandlePresence::Required,
        }),
        TypeExpr::Class { id, presence } => Ok(LiftPlan::Handle {
            target: HandleTarget::Class(ids.class(id)?),
            carrier: S::class_handle_carrier(),
            presence: lower_presence(*presence),
        }),
        TypeExpr::Trait {
            id,
            form: _,
            presence,
        } => Ok(LiftPlan::Handle {
            target: HandleTarget::Callback(ids.callback(id)?),
            carrier: S::callback_handle_carrier(),
            presence: lower_presence(*presence),
        }),
        TypeExpr::Custom(_) => {
            let _ = types::lower(ids, type_expr)?;
            Err(LowerError::unsupported_type(UnsupportedType::SelfType))
        }
        TypeExpr::SelfType | TypeExpr::Parameter(_) => {
            Err(types::lower(ids, type_expr).expect_err("unsupported value-position type expr"))
        }
    }
}

fn lower_presence(presence: boltffi_ast::HandlePresence) -> HandlePresence {
    match presence {
        boltffi_ast::HandlePresence::Required => HandlePresence::Required,
        boltffi_ast::HandlePresence::Nullable => HandlePresence::Nullable,
    }
}
