//! Constant declaration lowering.
//!
//! Walks every [`ConstantDef`] the source contract exposes and produces
//! a [`ConstantDecl<S>`] that names the constant, records the binding
//! [`TypeRef`] foreign code observes, and carries the literal value as
//! [`ConstantValueDecl::Inline`].
//!
//! Only inline literals are produced today. Bool, integer (range-
//! checked against the declared primitive width), float, string, and
//! enum-variant (via `ConstExpr::Path(...)` on an enum-typed constant)
//! all lower to the matching [`DefaultValue`] variant. Byte-string,
//! array, tuple, and raw constant expressions remain rejected with
//! [`UnsupportedType::DefaultValue`]: their values are not expressible
//! as a single literal in every target language, and supporting them
//! requires the [`ConstantValueDecl::Accessor`] path (a Rust-side
//! reader symbol the source AST does not carry yet).
//!
//! [`ConstantDef`]: boltffi_ast::ConstantDef
//! [`ConstantDecl<S>`]: crate::ConstantDecl
//! [`ConstantValueDecl::Inline`]: crate::ConstantValueDecl::Inline
//! [`ConstantValueDecl::Accessor`]: crate::ConstantValueDecl::Accessor
//! [`DefaultValue`]: crate::DefaultValue
//! [`TypeRef`]: crate::TypeRef
//! [`UnsupportedType::DefaultValue`]: super::error::UnsupportedType::DefaultValue

use boltffi_ast::{
    CanonicalName as SourceName, ConstExpr, ConstantDef as SourceConstant, EnumDef as SourceEnum,
    Literal, Path as SourcePath, Primitive as SourcePrimitive, TypeExpr, VariantPayload,
};

use crate::{CanonicalName, ConstantDecl, ConstantValueDecl, DefaultValue, IntegerValue};

use super::{
    LowerError, error::UnsupportedType, ids::DeclarationIds, index::Index, metadata,
    surface::SurfaceLower, types,
};

pub(super) fn lower<S: SurfaceLower>(
    idx: &Index<'_>,
    ids: &DeclarationIds,
) -> Result<Vec<ConstantDecl<S>>, LowerError> {
    idx.constants()
        .iter()
        .map(|constant| lower_one::<S>(idx, ids, constant))
        .collect()
}

fn lower_one<S: SurfaceLower>(
    idx: &Index<'_>,
    ids: &DeclarationIds,
    constant: &SourceConstant,
) -> Result<ConstantDecl<S>, LowerError> {
    let constant_id = ids.constant(&constant.id)?;
    let ty = types::lower(ids, &constant.type_expr)?;
    let value = inline_value(idx, constant)?;
    Ok(ConstantDecl::new(
        constant_id,
        CanonicalName::from(&constant.name),
        metadata::decl_meta(constant.doc.as_ref(), constant.deprecated.as_ref()),
        ConstantValueDecl::inline(ty, value),
    ))
}

fn inline_value(idx: &Index<'_>, constant: &SourceConstant) -> Result<DefaultValue, LowerError> {
    let Some(expected) = InlineConstantType::from_type_expr(idx, &constant.type_expr) else {
        return Err(LowerError::unsupported_type(UnsupportedType::DefaultValue));
    };
    expected.lower_value(constant)
}

#[derive(Clone, Copy)]
enum InlineConstantType<'src> {
    Bool,
    Integer(IntegerBounds),
    Float,
    String,
    Enum(&'src SourceEnum),
}

impl<'src> InlineConstantType<'src> {
    fn from_type_expr(idx: &Index<'src>, type_expr: &TypeExpr) -> Option<Self> {
        match type_expr {
            TypeExpr::Primitive(SourcePrimitive::Bool) => Some(Self::Bool),
            TypeExpr::Primitive(SourcePrimitive::F32 | SourcePrimitive::F64) => Some(Self::Float),
            TypeExpr::Primitive(primitive) => {
                IntegerBounds::for_primitive(*primitive).map(Self::Integer)
            }
            TypeExpr::String => Some(Self::String),
            TypeExpr::Enum(id) => idx.enumeration(id).map(Self::Enum),
            _ => None,
        }
    }

    fn lower_value(self, constant: &SourceConstant) -> Result<DefaultValue, LowerError> {
        match (self, &constant.value) {
            (Self::Bool, ConstExpr::Literal(Literal::Bool(value))) => {
                Ok(DefaultValue::Bool(*value))
            }
            (Self::Integer(bounds), ConstExpr::Literal(Literal::Integer(value)))
                if bounds.contains(value.value) =>
            {
                Ok(DefaultValue::Integer(IntegerValue::new(value.value)))
            }
            (Self::Float, ConstExpr::Literal(Literal::Float(literal))) => {
                metadata::parse_float_literal(literal)
                    .map(DefaultValue::Float)
                    .ok_or_else(|| LowerError::invalid_constant_value(&constant.id))
            }
            (Self::String, ConstExpr::Literal(Literal::String(value))) => {
                Ok(DefaultValue::String(value.clone()))
            }
            (Self::Enum(enumeration), ConstExpr::Path(path)) => {
                enum_variant_from_path(enumeration, path)
                    .ok_or_else(|| LowerError::invalid_constant_value(&constant.id))
            }
            (_, ConstExpr::Literal(Literal::Bytes(_)))
            | (_, ConstExpr::Array(_))
            | (_, ConstExpr::Tuple(_))
            | (_, ConstExpr::Raw(_)) => {
                Err(LowerError::unsupported_type(UnsupportedType::DefaultValue))
            }
            _ => Err(LowerError::invalid_constant_value(&constant.id)),
        }
    }
}

fn enum_variant_from_path(enumeration: &SourceEnum, path: &SourcePath) -> Option<DefaultValue> {
    let (enum_segment, variant_segment) = enum_variant_path_segments(path)?;
    if !canonical_name_matches_segment(&enumeration.name, enum_segment.name.as_str()) {
        return None;
    }
    let variant = enumeration.variants.iter().find(|variant| {
        matches!(variant.payload, VariantPayload::Unit)
            && canonical_name_matches_segment(&variant.name, variant_segment.name.as_str())
    })?;
    Some(DefaultValue::EnumVariant {
        enum_name: CanonicalName::from(&enumeration.name),
        variant_name: CanonicalName::from(&variant.name),
    })
}

fn enum_variant_path_segments(
    path: &SourcePath,
) -> Option<(&boltffi_ast::PathSegment, &boltffi_ast::PathSegment)> {
    let variant = path.segments.last()?;
    let enum_name = path.segments.get(path.segments.len().checked_sub(2)?)?;
    Some((enum_name, variant))
}

fn canonical_name_matches_segment(name: &SourceName, segment: &str) -> bool {
    let mut parts = name.parts();
    matches!(parts.next(), Some(part) if part.as_str() == segment) && parts.next().is_none()
}

#[derive(Clone, Copy)]
struct IntegerBounds {
    min: i128,
    max: i128,
}

impl IntegerBounds {
    const fn new(min: i128, max: i128) -> Self {
        Self { min, max }
    }

    const fn for_primitive(primitive: SourcePrimitive) -> Option<Self> {
        match primitive {
            SourcePrimitive::I8 => Some(Self::new(i8::MIN as i128, i8::MAX as i128)),
            SourcePrimitive::U8 => Some(Self::new(0, u8::MAX as i128)),
            SourcePrimitive::I16 => Some(Self::new(i16::MIN as i128, i16::MAX as i128)),
            SourcePrimitive::U16 => Some(Self::new(0, u16::MAX as i128)),
            SourcePrimitive::I32 => Some(Self::new(i32::MIN as i128, i32::MAX as i128)),
            SourcePrimitive::U32 => Some(Self::new(0, u32::MAX as i128)),
            SourcePrimitive::I64 | SourcePrimitive::ISize => {
                Some(Self::new(i64::MIN as i128, i64::MAX as i128))
            }
            SourcePrimitive::U64 | SourcePrimitive::USize => Some(Self::new(0, u64::MAX as i128)),
            SourcePrimitive::Bool | SourcePrimitive::F32 | SourcePrimitive::F64 => None,
        }
    }

    const fn contains(self, value: i128) -> bool {
        self.min <= value && value <= self.max
    }
}

#[cfg(test)]
mod tests {
    use boltffi_ast::{
        CanonicalName as SourceName, ConstExpr, ConstantDef, ConstantId as SourceConstantId,
        DeprecationInfo as SourceDeprecationInfo, DocComment as SourceDocComment, FloatLiteral,
        IntegerLiteral, Literal, PackageInfo as SourcePackage, Path as SourcePath, Primitive,
        SourceContract, TypeExpr,
    };

    use crate::lower::{LowerError, LowerErrorKind, UnsupportedType, lower};
    use crate::{
        Bindings, CanonicalName, ConstantDecl, ConstantId, ConstantValueDecl, Decl, DefaultValue,
        IntegerValue, Native, Primitive as BindingPrimitive, SurfaceLower, TypeRef, Wasm32,
    };

    fn package() -> SourceContract {
        SourceContract::new(SourcePackage::new("demo", Some("0.1.0".to_owned())))
    }

    fn name(part: &str) -> SourceName {
        SourceName::single(part)
    }

    fn constant(
        id: &str,
        constant_name: &str,
        type_expr: TypeExpr,
        value: ConstExpr,
    ) -> ConstantDef {
        ConstantDef::new(
            SourceConstantId::new(id),
            name(constant_name),
            type_expr,
            value,
        )
    }

    fn lower_constants<S: SurfaceLower>(
        constants: Vec<ConstantDef>,
    ) -> Result<Bindings<S>, LowerError> {
        let mut contract = package();
        contract.constants = constants;
        lower::<S>(&contract)
    }

    fn lower_constants_ok<S: SurfaceLower>(constants: Vec<ConstantDef>) -> Bindings<S> {
        lower_constants::<S>(constants).expect("constants should lower")
    }

    fn constant_decls<S: SurfaceLower>(bindings: &Bindings<S>) -> Vec<&ConstantDecl<S>> {
        bindings
            .decls()
            .iter()
            .filter_map(|decl| match decl {
                Decl::Constant(constant) => Some(constant.as_ref()),
                _ => None,
            })
            .collect()
    }

    fn only_constant<S: SurfaceLower>(bindings: &Bindings<S>) -> &ConstantDecl<S> {
        let decls = constant_decls(bindings);
        assert_eq!(decls.len(), 1, "expected exactly one constant declaration");
        decls[0]
    }

    #[test]
    fn bool_constant_lowers_inline_with_bool_default() {
        let bindings = lower_constants_ok::<Native>(vec![constant(
            "demo::ENABLED",
            "ENABLED",
            TypeExpr::Primitive(Primitive::Bool),
            ConstExpr::Literal(Literal::Bool(true)),
        )]);
        let decl = only_constant(&bindings);

        assert_eq!(decl.name(), &CanonicalName::single("ENABLED"));
        match decl.value() {
            ConstantValueDecl::Inline { ty, value, .. } => {
                assert_eq!(ty, &TypeRef::Primitive(BindingPrimitive::Bool));
                assert_eq!(value, &DefaultValue::Bool(true));
            }
            other => panic!("expected inline constant value, got {other:?}"),
        }
    }

    #[test]
    fn integer_constant_lowers_inline_with_integer_default() {
        let bindings = lower_constants_ok::<Native>(vec![constant(
            "demo::MAX_SIZE",
            "MAX_SIZE",
            TypeExpr::Primitive(Primitive::U32),
            ConstExpr::Literal(Literal::Integer(IntegerLiteral::new(1024, "1024"))),
        )]);
        let decl = only_constant(&bindings);

        match decl.value() {
            ConstantValueDecl::Inline { ty, value, .. } => {
                assert_eq!(ty, &TypeRef::Primitive(BindingPrimitive::U32));
                assert_eq!(value, &DefaultValue::Integer(IntegerValue::new(1024)));
            }
            other => panic!("expected inline integer constant, got {other:?}"),
        }
    }

    #[test]
    fn string_constant_lowers_inline_with_string_default() {
        let bindings = lower_constants_ok::<Native>(vec![constant(
            "demo::GREETING",
            "GREETING",
            TypeExpr::String,
            ConstExpr::Literal(Literal::String("hello".to_owned())),
        )]);
        let decl = only_constant(&bindings);

        match decl.value() {
            ConstantValueDecl::Inline { ty, value, .. } => {
                assert_eq!(ty, &TypeRef::String);
                assert_eq!(value, &DefaultValue::String("hello".to_owned()));
            }
            other => panic!("expected inline string constant, got {other:?}"),
        }
    }

    #[test]
    fn constant_literal_must_match_declared_type() {
        let error = lower_constants::<Native>(vec![constant(
            "demo::ENABLED",
            "ENABLED",
            TypeExpr::String,
            ConstExpr::Literal(Literal::Bool(true)),
        )])
        .expect_err("mismatched constant literal must reject");

        assert!(matches!(
            error.kind(),
            LowerErrorKind::InvalidConstantValue(constant) if constant == "demo::ENABLED"
        ));
    }

    #[test]
    fn integer_constant_must_fit_declared_type() {
        let error = lower_constants::<Native>(vec![constant(
            "demo::BYTE",
            "BYTE",
            TypeExpr::Primitive(Primitive::U8),
            ConstExpr::Literal(Literal::Integer(IntegerLiteral::new(256, "256"))),
        )])
        .expect_err("out-of-range integer constant must reject");

        assert!(matches!(
            error.kind(),
            LowerErrorKind::InvalidConstantValue(constant) if constant == "demo::BYTE"
        ));
    }

    #[test]
    fn constants_use_same_inline_value_on_wasm32() {
        let bindings = lower_constants_ok::<Wasm32>(vec![constant(
            "demo::MAX_SIZE",
            "MAX_SIZE",
            TypeExpr::Primitive(Primitive::U32),
            ConstExpr::Literal(Literal::Integer(IntegerLiteral::new(1024, "1024"))),
        )]);

        match only_constant(&bindings).value() {
            ConstantValueDecl::Inline { ty, value, .. } => {
                assert_eq!(ty, &TypeRef::Primitive(BindingPrimitive::U32));
                assert_eq!(value, &DefaultValue::Integer(IntegerValue::new(1024)));
            }
            other => panic!("expected inline integer constant, got {other:?}"),
        }
    }

    #[test]
    fn float_constant_lowers_inline_with_float_default() {
        let bindings = lower_constants_ok::<Native>(vec![constant(
            "demo::HALF",
            "HALF",
            TypeExpr::Primitive(Primitive::F64),
            ConstExpr::Literal(Literal::Float(FloatLiteral::new("0.5"))),
        )]);
        let decl = only_constant(&bindings);

        match decl.value() {
            ConstantValueDecl::Inline { ty, value, .. } => {
                assert_eq!(ty, &TypeRef::Primitive(BindingPrimitive::F64));
                match value {
                    DefaultValue::Float(float) => {
                        assert!((float.to_f64() - 0.5).abs() < 1e-12);
                    }
                    other => panic!("expected DefaultValue::Float, got {other:?}"),
                }
            }
            other => panic!("expected inline float constant, got {other:?}"),
        }
    }

    #[test]
    fn float_constant_strips_type_suffix_and_underscores() {
        let bindings = lower_constants_ok::<Native>(vec![constant(
            "demo::HZ",
            "HZ",
            TypeExpr::Primitive(Primitive::F64),
            ConstExpr::Literal(Literal::Float(FloatLiteral::new("1_000.5f64"))),
        )]);

        match only_constant(&bindings).value() {
            ConstantValueDecl::Inline {
                value: DefaultValue::Float(float),
                ..
            } => {
                assert!((float.to_f64() - 1000.5).abs() < 1e-12);
            }
            other => panic!("expected inline float constant, got {other:?}"),
        }
    }

    #[test]
    fn float_constant_with_garbled_source_is_rejected_with_invalid_value() {
        let error = lower_constants::<Native>(vec![constant(
            "demo::BAD",
            "BAD",
            TypeExpr::Primitive(Primitive::F64),
            ConstExpr::Literal(Literal::Float(FloatLiteral::new("not_a_number"))),
        )])
        .expect_err("unparseable float literal must reject");

        assert!(matches!(
            error.kind(),
            LowerErrorKind::InvalidConstantValue(constant) if constant == "demo::BAD"
        ));
    }

    #[test]
    fn bytes_constant_is_rejected_until_inline_bytes_land() {
        let error = lower_constants::<Native>(vec![constant(
            "demo::MAGIC",
            "MAGIC",
            TypeExpr::Bytes,
            ConstExpr::Literal(Literal::Bytes(vec![0xCA, 0xFE])),
        )])
        .expect_err("bytes constant must reject");

        assert!(matches!(
            error.kind(),
            LowerErrorKind::UnsupportedType(UnsupportedType::DefaultValue)
        ));
    }

    #[test]
    fn path_constant_on_non_enum_type_is_rejected_with_invalid_value() {
        let error = lower_constants::<Native>(vec![constant(
            "demo::ALIAS",
            "ALIAS",
            TypeExpr::String,
            ConstExpr::Path(SourcePath::single("OTHER")),
        )])
        .expect_err("path value on a String-typed constant must reject");

        assert!(matches!(
            error.kind(),
            LowerErrorKind::InvalidConstantValue(constant) if constant == "demo::ALIAS"
        ));
    }

    #[test]
    fn enum_constant_lowers_inline_with_enum_variant_default() {
        use boltffi_ast::{EnumDef, EnumId as SourceEnumId, PathRoot, PathSegment, VariantDef};

        // Set up an enum `demo::Mode` with variants Fast/Slow, then a
        // constant `DEFAULT_MODE: Mode = Mode::Fast`.
        let mut contract = package();
        let mut mode = EnumDef::new(SourceEnumId::new("demo::Mode"), name("Mode"));
        mode.variants.push(VariantDef::unit(name("Fast")));
        mode.variants.push(VariantDef::unit(name("Slow")));
        contract.enums.push(mode);
        contract.constants.push(constant(
            "demo::DEFAULT_MODE",
            "DEFAULT_MODE",
            TypeExpr::Enum(SourceEnumId::new("demo::Mode")),
            ConstExpr::Path(SourcePath::new(
                PathRoot::Relative,
                vec![PathSegment::new("Mode"), PathSegment::new("Fast")],
            )),
        ));

        let bindings = lower::<Native>(&contract).expect("enum constant should lower");
        let decl = only_constant(&bindings);

        match decl.value() {
            ConstantValueDecl::Inline {
                value:
                    DefaultValue::EnumVariant {
                        enum_name,
                        variant_name,
                    },
                ..
            } => {
                assert_eq!(enum_name, &CanonicalName::single("Mode"));
                assert_eq!(variant_name, &CanonicalName::single("Fast"));
            }
            other => panic!("expected inline enum-variant constant, got {other:?}"),
        }
    }

    #[test]
    fn enum_constant_accepts_qualified_path() {
        use boltffi_ast::{EnumDef, EnumId as SourceEnumId, PathRoot, PathSegment, VariantDef};

        let mut contract = package();
        let mut mode = EnumDef::new(SourceEnumId::new("demo::Mode"), name("Mode"));
        mode.variants.push(VariantDef::unit(name("Fast")));
        contract.enums.push(mode);
        contract.constants.push(constant(
            "demo::DEFAULT_MODE",
            "DEFAULT_MODE",
            TypeExpr::Enum(SourceEnumId::new("demo::Mode")),
            ConstExpr::Path(SourcePath::new(
                PathRoot::Crate,
                vec![
                    PathSegment::new("demo"),
                    PathSegment::new("Mode"),
                    PathSegment::new("Fast"),
                ],
            )),
        ));

        let bindings =
            lower::<Native>(&contract).expect("qualified enum-path constant should lower");

        match only_constant(&bindings).value() {
            ConstantValueDecl::Inline {
                value:
                    DefaultValue::EnumVariant {
                        enum_name,
                        variant_name,
                    },
                ..
            } => {
                assert_eq!(enum_name, &CanonicalName::single("Mode"));
                assert_eq!(variant_name, &CanonicalName::single("Fast"));
            }
            other => panic!("expected enum-variant default, got {other:?}"),
        }
    }

    #[test]
    fn enum_constant_rejects_path_for_another_enum() {
        use boltffi_ast::{EnumDef, EnumId as SourceEnumId, PathSegment, VariantDef};

        let mut contract = package();
        let mut mode = EnumDef::new(SourceEnumId::new("demo::Mode"), name("Mode"));
        mode.variants.push(VariantDef::unit(name("Fast")));
        contract.enums.push(mode);
        contract.constants.push(constant(
            "demo::DEFAULT_MODE",
            "DEFAULT_MODE",
            TypeExpr::Enum(SourceEnumId::new("demo::Mode")),
            ConstExpr::Path(SourcePath::new(
                boltffi_ast::PathRoot::Relative,
                vec![PathSegment::new("Other"), PathSegment::new("Fast")],
            )),
        ));

        let error = lower::<Native>(&contract).expect_err("wrong enum path must reject");

        assert!(matches!(
            error.kind(),
            LowerErrorKind::InvalidConstantValue(constant) if constant == "demo::DEFAULT_MODE"
        ));
    }

    #[test]
    fn enum_constant_rejects_unknown_variant_path() {
        use boltffi_ast::{EnumDef, EnumId as SourceEnumId, PathSegment, VariantDef};

        let mut contract = package();
        let mut mode = EnumDef::new(SourceEnumId::new("demo::Mode"), name("Mode"));
        mode.variants.push(VariantDef::unit(name("Fast")));
        contract.enums.push(mode);
        contract.constants.push(constant(
            "demo::DEFAULT_MODE",
            "DEFAULT_MODE",
            TypeExpr::Enum(SourceEnumId::new("demo::Mode")),
            ConstExpr::Path(SourcePath::new(
                boltffi_ast::PathRoot::Relative,
                vec![PathSegment::new("Mode"), PathSegment::new("Slow")],
            )),
        ));

        let error = lower::<Native>(&contract).expect_err("unknown enum variant must reject");

        assert!(matches!(
            error.kind(),
            LowerErrorKind::InvalidConstantValue(constant) if constant == "demo::DEFAULT_MODE"
        ));
    }

    #[test]
    fn enum_constant_rejects_payload_variant_path() {
        use boltffi_ast::{
            EnumDef, EnumId as SourceEnumId, FieldDef, PathSegment, VariantDef, VariantPayload,
        };

        let mut contract = package();
        let mut mode = EnumDef::new(SourceEnumId::new("demo::Mode"), name("Mode"));
        mode.variants.push(VariantDef {
            name: name("Fast"),
            discriminant: None,
            payload: VariantPayload::Tuple(vec![TypeExpr::Primitive(Primitive::U32)]),
            doc: None,
            user_attrs: Vec::new(),
            source: boltffi_ast::Source::exported(),
            source_span: None,
        });
        mode.variants.push(VariantDef {
            name: name("Slow"),
            discriminant: None,
            payload: VariantPayload::Struct(vec![FieldDef::new(
                name("value"),
                TypeExpr::Primitive(Primitive::U32),
            )]),
            doc: None,
            user_attrs: Vec::new(),
            source: boltffi_ast::Source::exported(),
            source_span: None,
        });
        contract.enums.push(mode);
        contract.constants.push(constant(
            "demo::DEFAULT_MODE",
            "DEFAULT_MODE",
            TypeExpr::Enum(SourceEnumId::new("demo::Mode")),
            ConstExpr::Path(SourcePath::new(
                boltffi_ast::PathRoot::Relative,
                vec![PathSegment::new("Mode"), PathSegment::new("Fast")],
            )),
        ));

        let error = lower::<Native>(&contract).expect_err("payload enum variant must reject");

        assert!(matches!(
            error.kind(),
            LowerErrorKind::InvalidConstantValue(constant) if constant == "demo::DEFAULT_MODE"
        ));
    }

    #[test]
    fn enum_constant_with_bare_variant_path_is_rejected() {
        use boltffi_ast::{EnumDef, EnumId as SourceEnumId};

        let mut contract = package();
        let mut mode = EnumDef::new(SourceEnumId::new("demo::Mode"), name("Mode"));
        mode.variants
            .push(boltffi_ast::VariantDef::unit(name("Fast")));
        contract.enums.push(mode);
        contract.constants.push(constant(
            "demo::DEFAULT_MODE",
            "DEFAULT_MODE",
            TypeExpr::Enum(SourceEnumId::new("demo::Mode")),
            ConstExpr::Path(SourcePath::single("Fast")),
        ));

        let error =
            lower::<Native>(&contract).expect_err("bare-variant path must reject (enum unknown)");

        assert!(matches!(
            error.kind(),
            LowerErrorKind::InvalidConstantValue(constant) if constant == "demo::DEFAULT_MODE"
        ));
    }

    #[test]
    fn raw_constant_is_rejected_until_accessor_lands() {
        let error = lower_constants::<Native>(vec![constant(
            "demo::COMPUTED",
            "COMPUTED",
            TypeExpr::Primitive(Primitive::U32),
            ConstExpr::Raw("1 + 2".to_owned()),
        )])
        .expect_err("raw constant expression must reject");

        assert!(matches!(
            error.kind(),
            LowerErrorKind::UnsupportedType(UnsupportedType::DefaultValue)
        ));
    }

    #[test]
    fn duplicate_constant_source_ids_are_rejected() {
        let error = lower_constants::<Native>(vec![
            constant(
                "demo::DUP",
                "DUP",
                TypeExpr::Primitive(Primitive::U32),
                ConstExpr::Literal(Literal::Integer(IntegerLiteral::new(1, "1"))),
            ),
            constant(
                "demo::DUP",
                "DUP_AGAIN",
                TypeExpr::Primitive(Primitive::U32),
                ConstExpr::Literal(Literal::Integer(IntegerLiteral::new(2, "2"))),
            ),
        ])
        .expect_err("duplicate constant id must reject");

        assert!(matches!(
            error.kind(),
            LowerErrorKind::DuplicateSourceId { .. }
        ));
    }

    #[test]
    fn multiple_constants_get_sequential_ids_in_source_order() {
        let bindings = lower_constants_ok::<Native>(vec![
            constant(
                "demo::ONE",
                "ONE",
                TypeExpr::Primitive(Primitive::U32),
                ConstExpr::Literal(Literal::Integer(IntegerLiteral::new(1, "1"))),
            ),
            constant(
                "demo::TWO",
                "TWO",
                TypeExpr::Primitive(Primitive::U32),
                ConstExpr::Literal(Literal::Integer(IntegerLiteral::new(2, "2"))),
            ),
            constant(
                "demo::THREE",
                "THREE",
                TypeExpr::Primitive(Primitive::U32),
                ConstExpr::Literal(Literal::Integer(IntegerLiteral::new(3, "3"))),
            ),
        ]);
        let ids: Vec<u32> = constant_decls(&bindings)
            .into_iter()
            .map(|decl| decl.id().raw())
            .collect();

        assert_eq!(ids, vec![0, 1, 2]);
        assert_eq!(constant_decls(&bindings)[0].id(), ConstantId::from_raw(0));
    }

    #[test]
    fn constant_doc_and_deprecation_propagate_to_decl_meta() {
        let mut greeting = constant(
            "demo::GREETING",
            "GREETING",
            TypeExpr::String,
            ConstExpr::Literal(Literal::String("hello".to_owned())),
        );
        greeting.doc = Some(SourceDocComment::new("standard greeting"));
        greeting.deprecated = Some(SourceDeprecationInfo {
            note: Some("use GREETING_V2".to_owned()),
            since: Some("0.5".to_owned()),
        });

        let bindings = lower_constants_ok::<Native>(vec![greeting]);
        let meta = only_constant(&bindings).meta();

        assert_eq!(
            meta.doc().map(|doc| doc.as_str()),
            Some("standard greeting")
        );
        assert_eq!(
            meta.deprecated()
                .and_then(|deprecated| deprecated.message()),
            Some("use GREETING_V2")
        );
    }

    #[test]
    fn constant_does_not_register_any_native_symbols() {
        let bindings = lower_constants_ok::<Native>(vec![constant(
            "demo::MAX",
            "MAX",
            TypeExpr::Primitive(Primitive::U32),
            ConstExpr::Literal(Literal::Integer(IntegerLiteral::new(99, "99"))),
        )]);

        assert_eq!(bindings.symbols().symbols().len(), 0);
    }
}
