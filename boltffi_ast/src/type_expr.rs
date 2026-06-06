use serde::{Deserialize, Serialize};

use crate::{ClassId, CustomTypeId, EnumId, Primitive, RecordId, ReturnDef, TraitId};

/// A Rust source type and its scanned semantic expression.
///
/// `spelling` preserves the type text written by the Rust author, such as
/// `crate::models::Point` or `Vec<u8>`. `expr` stores the resolved BoltFFI type
/// expression used by lowering and validation.
#[derive(Clone, Debug, Eq, Hash, PartialEq, Serialize, Deserialize)]
pub struct RustType {
    spelling: String,
    expr: TypeExpr,
}

impl RustType {
    /// Builds a Rust source type.
    pub fn new(spelling: impl Into<String>, expr: TypeExpr) -> Self {
        Self {
            spelling: spelling.into(),
            expr,
        }
    }

    /// Builds a Rust type from a semantic expression when source spelling is unavailable.
    pub fn from_expr(expr: TypeExpr) -> Self {
        Self {
            spelling: expr.fallback_spelling(),
            expr,
        }
    }

    /// Returns the Rust type spelling.
    pub fn spelling(&self) -> &str {
        &self.spelling
    }

    /// Returns the scanned semantic type expression.
    pub const fn expr(&self) -> &TypeExpr {
        &self.expr
    }

    /// Consumes the source type and returns its semantic expression.
    pub fn into_expr(self) -> TypeExpr {
        self.expr
    }
}

impl From<TypeExpr> for RustType {
    fn from(expr: TypeExpr) -> Self {
        Self::from_expr(expr)
    }
}

/// Form in which a Rust trait appears as a source value.
///
/// Names the supported Rust spellings for trait-typed values: a
/// monomorphized `impl Trait`, an owned `Box<dyn Trait>`, or a shared
/// `Arc<dyn Trait>`.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq, Serialize, Deserialize)]
pub enum TraitUseForm {
    /// `impl Trait`.
    ImplTrait,
    /// `Box<dyn Trait>`.
    BoxedDyn,
    /// `Arc<dyn Trait>`.
    ArcDyn,
}

/// Whether a handle-typed source value is required or optional.
///
/// The scanner folds source shapes such as `Option<Engine>` and
/// `Option<Box<dyn Listener>>` into the handle-bearing type expression
/// instead of keeping an outer [`TypeExpr::Option`].
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq, Serialize, Deserialize)]
pub enum HandlePresence {
    /// Caller must supply a live handle.
    Required,
    /// Caller may omit the handle; a zero/null sentinel encodes absence.
    Nullable,
}

/// A type expression in the exported Rust surface.
///
/// This is the shape produced after scanning a Rust type from a field,
/// parameter, or return. Known exported names have been turned into IDs, and
/// ordinary Rust containers remain as a tree. For example,
/// `Option<Vec<Point>>` becomes `Option(Vec(Record(point_id)))`, `(u32,
/// String)` becomes `Tuple([Primitive(U32), String])`, inline closure
/// signatures become `Closure`, and `Self` stays explicit when it appears
/// inside an impl block.
#[derive(Clone, Debug, Eq, Hash, PartialEq, Serialize, Deserialize)]
pub enum TypeExpr {
    /// A primitive Rust scalar.
    Primitive(Primitive),
    /// The Rust unit type `()`.
    ///
    /// A callable that returns nothing uses
    /// [`ReturnDef::Void`](crate::ReturnDef::Void). This variant records
    /// unit when it appears as a written value type, such as
    /// `Result<(), E>`.
    Unit,
    /// A record declaration by ID.
    Record(RecordId),
    /// An enum declaration by ID.
    Enum(EnumId),
    /// A class-style object reference.
    ///
    /// Source `Option<Engine>` is represented as `Class { id, presence:
    /// Nullable }`; required class values use `presence: Required`.
    Class {
        /// The class declaration this reference resolves to.
        id: ClassId,
        /// Whether the source value is nullable.
        presence: HandlePresence,
    },
    /// A reference to a Rust trait the user wrote.
    ///
    /// The source entity is a trait, such as `trait Listener { ... }`,
    /// and this variant references it. Source `Option<Box<dyn T>>`
    /// collapses to `Trait { form: BoxedDyn, presence: Nullable }`.
    Trait {
        /// The trait declaration this reference resolves to.
        id: TraitId,
        /// Rust form used at this trait-typed source value.
        form: TraitUseForm,
        /// Whether the source value is nullable.
        presence: HandlePresence,
    },
    /// An inline closure value such as `impl Fn(u32) -> String`.
    Closure {
        /// Closure signature written by the source.
        signature: Box<ClosureType>,
        /// Whether the closure value is nullable.
        presence: HandlePresence,
    },
    /// A custom type declaration by ID.
    Custom(CustomTypeId),
    /// The Rust `Self` type inside an impl, trait, or callback context.
    SelfType,
    /// A `Vec<T>` source type.
    Vec(Box<TypeExpr>),
    /// An `Option<T>` source type.
    ///
    /// Used for ordinary optional values, such as `Option<i32>` and
    /// `Option<String>`. Source `Option<...>` wrapping a supported
    /// trait-typed value lands in [`TypeExpr::Trait`] with
    /// `presence: Nullable`.
    Option(Box<TypeExpr>),
    /// A `Result<T, E>` source type.
    ///
    /// Preserves both source type arguments in order. The same expression
    /// can appear in returns, fields, parameters, or nested containers.
    Result {
        /// Success type written as the first `Result` argument.
        ok: Box<TypeExpr>,
        /// Error type written as the second `Result` argument.
        err: Box<TypeExpr>,
    },
    /// A tuple type such as `(u32, String)`.
    ///
    /// Tuples are ordinary value types in the AST. A function returning
    /// `(u32, String)` is represented as `ReturnDef::Value(TypeExpr::Tuple(_))`,
    /// while a function returning `Result<(u32, String), Error>` is represented
    /// as `ReturnDef::Value(TypeExpr::Result { ok: TypeExpr::Tuple(_), err: ... })`.
    /// The empty tuple is *not* represented here; use [`TypeExpr::Unit`].
    Tuple(Vec<TypeExpr>),
    /// A map-like source type.
    Map {
        /// Key type written by the source map.
        key: Box<TypeExpr>,
        /// Value type written by the source map.
        value: Box<TypeExpr>,
    },
    /// A UTF-8 string source type.
    String,
    /// A byte buffer source type.
    Bytes,
    /// A type parameter used by a generic declaration the scanner chose to keep.
    Parameter(TypeParameter),
}

impl TypeExpr {
    /// Builds a `Vec<T>` type expression.
    ///
    /// The `element` parameter is the source type written inside the vector.
    ///
    /// Returns a vector type expression.
    pub fn vec(element: TypeExpr) -> Self {
        Self::Vec(Box::new(element))
    }

    /// Builds an `Option<T>` type expression.
    ///
    /// The `inner` parameter is the source type written inside the option.
    ///
    /// Returns an optional type expression.
    pub fn option(inner: TypeExpr) -> Self {
        Self::Option(Box::new(inner))
    }

    /// Builds a `Result<T, E>` type expression.
    ///
    /// The `ok` parameter is the success type. The `err` parameter is the error
    /// type.
    ///
    /// Returns a result type expression for nested or non-callable positions.
    pub fn result(ok: TypeExpr, err: TypeExpr) -> Self {
        Self::Result {
            ok: Box::new(ok),
            err: Box::new(err),
        }
    }

    /// Builds an inline closure type expression.
    ///
    /// The `closure` parameter contains the callable signature written inside a
    /// closure-like parameter type.
    ///
    /// Returns a closure type expression.
    pub fn closure(closure: ClosureType) -> Self {
        Self::closure_with_presence(closure, HandlePresence::Required)
    }

    /// Builds an inline closure type expression with explicit nullability.
    ///
    /// The `closure` parameter contains the source signature. The `presence`
    /// parameter records whether the closure value may be absent.
    ///
    /// Returns a closure type expression.
    pub fn closure_with_presence(closure: ClosureType, presence: HandlePresence) -> Self {
        Self::Closure {
            signature: Box::new(closure),
            presence,
        }
    }

    /// Builds a trait-reference type expression.
    ///
    /// The `id` parameter is the trait declaration. The `form` parameter is
    /// the Rust form used in source. The `presence` parameter is the
    /// source nullability.
    pub fn r#trait(id: TraitId, form: TraitUseForm, presence: HandlePresence) -> Self {
        Self::Trait { id, form, presence }
    }

    /// Builds a class-reference type expression.
    ///
    /// The `id` parameter is the class declaration. The `presence`
    /// parameter is the source nullability.
    pub fn class(id: ClassId, presence: HandlePresence) -> Self {
        Self::Class { id, presence }
    }

    /// Builds a tuple type expression.
    ///
    /// The `elements` parameter preserves the tuple element types in source
    /// order. A one-element tuple still has one element here; the scanner does
    /// not need a special case for Rust's trailing-comma syntax once parsing is
    /// finished.
    ///
    /// Returns a tuple value type, suitable for fields, parameters, nested
    /// containers, and `ReturnDef::Value`.
    pub fn tuple(elements: Vec<TypeExpr>) -> Self {
        Self::Tuple(elements)
    }

    /// Builds a map type expression.
    ///
    /// The `key` parameter is the source key type. The `value` parameter is the
    /// source value type.
    ///
    /// Returns a map type expression.
    pub fn map(key: TypeExpr, value: TypeExpr) -> Self {
        Self::Map {
            key: Box::new(key),
            value: Box::new(value),
        }
    }

    fn fallback_spelling(&self) -> String {
        match self {
            Self::Primitive(primitive) => primitive.rust_name().to_owned(),
            Self::Unit => "()".to_owned(),
            Self::Record(id) => id.as_str().to_owned(),
            Self::Enum(id) => id.as_str().to_owned(),
            Self::Class { id, .. } => id.as_str().to_owned(),
            Self::Trait { id, .. } => id.as_str().to_owned(),
            Self::Closure { signature, .. } => signature.fallback_spelling(),
            Self::Custom(id) => id.as_str().to_owned(),
            Self::SelfType => "Self".to_owned(),
            Self::Vec(element) => format!("Vec<{}>", element.fallback_spelling()),
            Self::Option(inner) => format!("Option<{}>", inner.fallback_spelling()),
            Self::Result { ok, err } => {
                format!(
                    "Result<{}, {}>",
                    ok.fallback_spelling(),
                    err.fallback_spelling()
                )
            }
            Self::Tuple(elements) => {
                let rendered = elements
                    .iter()
                    .map(Self::fallback_spelling)
                    .collect::<Vec<_>>()
                    .join(", ");
                format!("({rendered})")
            }
            Self::Map { key, value } => {
                format!(
                    "std::collections::HashMap<{}, {}>",
                    key.fallback_spelling(),
                    value.fallback_spelling()
                )
            }
            Self::String => "String".to_owned(),
            Self::Bytes => "Vec<u8>".to_owned(),
            Self::Parameter(parameter) => parameter.name.as_str().to_owned(),
        }
    }
}

/// Callable trait used by a closure-shaped Rust type.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq, Serialize, Deserialize)]
pub enum ClosureTrait {
    /// `Fn(...)`.
    Fn,
    /// `FnMut(...)`.
    FnMut,
    /// `FnOnce(...)`.
    FnOnce,
}

/// Rust source form used for an inline closure type.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq, Serialize, Deserialize)]
pub enum ClosureKind {
    /// Bare `fn(...)` function pointer.
    FunctionPointer,
    /// `impl Fn*(...)` opaque closure type.
    ImplTrait(ClosureTrait),
    /// `Box<dyn Fn*(...)>` owned closure trait object.
    BoxedTraitObject(ClosureTrait),
}

/// An inline closure signature used as a type expression.
#[derive(Clone, Debug, Eq, Hash, PartialEq, Serialize, Deserialize)]
pub struct ClosureType {
    /// Rust source form that carried the closure signature.
    pub kind: ClosureKind,
    /// Types accepted by the closure in source order.
    pub parameters: Vec<RustType>,
    /// Return type written by the closure signature.
    pub returns: ReturnDef,
}

impl ClosureType {
    /// Builds an inline closure signature.
    ///
    /// The `kind` parameter records the source spelling the parser saw.
    /// The `parameters` parameter preserves closure parameter types in source order.
    /// The `returns` parameter is the closure return type.
    ///
    /// Returns a closure signature suitable for [`TypeExpr::Closure`].
    pub fn new<I>(kind: ClosureKind, parameters: I, returns: ReturnDef) -> Self
    where
        I: IntoIterator,
        I::Item: Into<RustType>,
    {
        Self {
            kind,
            parameters: parameters.into_iter().map(Into::into).collect(),
            returns,
        }
    }

    fn fallback_spelling(&self) -> String {
        let parameters = self
            .parameters
            .iter()
            .map(|parameter| parameter.spelling())
            .collect::<Vec<_>>()
            .join(", ");
        let signature = match &self.returns {
            ReturnDef::Void => format!("({parameters})"),
            ReturnDef::Value(rust_type) => format!("({parameters}) -> {}", rust_type.spelling()),
        };
        match self.kind {
            ClosureKind::FunctionPointer => format!("fn{signature}"),
            ClosureKind::ImplTrait(trait_kind) => {
                format!("impl {}{signature}", trait_kind.as_ref())
            }
            ClosureKind::BoxedTraitObject(trait_kind) => {
                format!("Box<dyn {}{signature}>", trait_kind.as_ref())
            }
        }
    }
}

impl AsRef<str> for ClosureTrait {
    fn as_ref(&self) -> &str {
        match self {
            Self::Fn => "Fn",
            Self::FnMut => "FnMut",
            Self::FnOnce => "FnOnce",
        }
    }
}

/// A named type parameter referenced by a source type expression.
///
/// Generic exports may be rejected or specialized after scanning. Preserving
/// the parameter name gives those errors the original source shape.
#[derive(Clone, Debug, Eq, Hash, PartialEq, Serialize, Deserialize)]
pub struct TypeParameter {
    /// Parameter name as written in Rust source.
    pub name: String,
}

impl TypeParameter {
    /// Builds a type parameter reference.
    ///
    /// The `name` parameter is stored exactly as the scanner reported it.
    ///
    /// Returns a type parameter expression for generic source syntax.
    pub fn new(name: impl Into<String>) -> Self {
        Self { name: name.into() }
    }
}
