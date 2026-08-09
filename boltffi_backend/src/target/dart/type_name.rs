use boltffi_binding::{
    BuiltinType, CallbackId, ClassId, CustomTypeId, DirectValueType, DirectVectorElementType,
    EnumId, HandlePresence, HandleTarget, Native, Primitive, RecordId, TypeRef, TypeRefRender,
};

use crate::core::{Error, RenderContext, Result};

use super::{
    name_style::Name,
    syntax::{Syntax, TypeFragment},
};

pub fn type_ref(ty: &TypeRef, context: &RenderContext<Native>) -> Result<TypeFragment> {
    ty.render_with(&mut Renderer { context })
        .map(TypeSpelling::into_value)
}

pub fn direct_value(ty: &DirectValueType, context: &RenderContext<Native>) -> Result<TypeFragment> {
    match ty {
        DirectValueType::Primitive(primitive) => primitive_type(*primitive),
        DirectValueType::Record(id) => record(*id, context),
        DirectValueType::Enum(id) => enumeration(*id, context),
        _ => super::unsupported("unknown direct value type"),
    }
}

pub fn direct_vector(
    element: &DirectVectorElementType,
    context: &RenderContext<Native>,
) -> Result<TypeFragment> {
    match element {
        DirectVectorElementType::Primitive(primitive) => sequence_type(
            primitive_type(primitive.primitive())?,
            Some(primitive.primitive()),
        ),
        DirectVectorElementType::Record(id) => sequence_type(record(*id, context)?, None),
        _ => super::unsupported("unknown direct vector element"),
    }
}

pub fn handle(
    target: &HandleTarget,
    presence: HandlePresence,
    context: &RenderContext<Native>,
) -> Result<TypeFragment> {
    let ty = match target {
        HandleTarget::Class(id) => class(*id, context)?,
        HandleTarget::Callback(id) => callback(*id, context)?,
        HandleTarget::Stream(_) => {
            return Err(Error::UnsupportedTarget {
                target: "dart",
                shape: "stream handle as a public value",
            });
        }
        _ => return super::unsupported("unknown handle target"),
    };
    Ok(match presence {
        HandlePresence::Required => ty,
        HandlePresence::Nullable => TypeFragment::new(format!("{ty}?")),
        _ => return super::unsupported("unknown handle presence"),
    })
}

pub fn primitive_type(primitive: Primitive) -> Result<TypeFragment> {
    Ok(TypeFragment::new(match primitive {
        Primitive::Bool => "bool",
        Primitive::I8
        | Primitive::U8
        | Primitive::I16
        | Primitive::U16
        | Primitive::I32
        | Primitive::U32
        | Primitive::I64
        | Primitive::U64
        | Primitive::ISize
        | Primitive::USize => "int",
        Primitive::F32 | Primitive::F64 => "double",
        _ => return super::unsupported("unknown primitive type"),
    }))
}

fn record(id: RecordId, context: &RenderContext<Native>) -> Result<TypeFragment> {
    declaration_type(
        context.record(id).map(|declaration| declaration.name()),
        "missing record declaration",
    )
}

fn enumeration(id: EnumId, context: &RenderContext<Native>) -> Result<TypeFragment> {
    declaration_type(
        context
            .enumeration(id)
            .map(|declaration| declaration.name()),
        "missing enum declaration",
    )
}

fn class(id: ClassId, context: &RenderContext<Native>) -> Result<TypeFragment> {
    declaration_type(
        context.class(id).map(|declaration| declaration.name()),
        "missing class declaration",
    )
}

fn callback(id: CallbackId, context: &RenderContext<Native>) -> Result<TypeFragment> {
    declaration_type(
        context.callback(id).map(|declaration| declaration.name()),
        "missing callback declaration",
    )
}

fn declaration_type(
    name: Option<&boltffi_binding::CanonicalName>,
    shape: &'static str,
) -> Result<TypeFragment> {
    name.map(Name::new)
        .map(|name| name.upper_camel())
        .transpose()?
        .map(|name| TypeFragment::new(name.to_string()))
        .ok_or(Error::UnexpectedBindingShape {
            layer: "dart type",
            shape,
        })
}

struct Renderer<'context, 'bindings> {
    context: &'context RenderContext<'bindings, Native>,
}

struct TypeSpelling {
    value: TypeFragment,
    result_failure: Option<TypeFragment>,
}

impl TypeSpelling {
    fn value(value: TypeFragment) -> Self {
        Self {
            value,
            result_failure: None,
        }
    }

    fn error(value: TypeFragment) -> Self {
        Self {
            value: value.clone(),
            result_failure: Some(value),
        }
    }

    fn string() -> Self {
        Self {
            value: TypeFragment::new("String"),
            result_failure: Some(TypeFragment::new("$$BoltException")),
        }
    }

    fn into_value(self) -> TypeFragment {
        self.value
    }

    fn into_result_failure(self) -> Result<TypeFragment> {
        self.result_failure.ok_or(Error::UnsupportedTarget {
            target: "dart",
            shape: "Dart Result failure type",
        })
    }
}

impl TypeRefRender for Renderer<'_, '_> {
    type Output = Result<TypeSpelling>;

    fn primitive(&mut self, primitive: Primitive) -> Self::Output {
        primitive_type(primitive).map(TypeSpelling::value)
    }

    fn string(&mut self) -> Self::Output {
        Ok(TypeSpelling::string())
    }

    fn interned_string(&mut self, _: &[String]) -> Self::Output {
        unreachable!("InternedString type reached Dart renderer without host capability")
    }

    fn bytes(&mut self) -> Self::Output {
        Ok(TypeSpelling::value(TypeFragment::new(
            "$$typed_data.Uint8List",
        )))
    }

    fn record(&mut self, id: RecordId) -> Self::Output {
        let ty = record(id, self.context)?;
        match self
            .context
            .record(id)
            .is_some_and(|record| record.is_error_payload())
        {
            true => Ok(TypeSpelling::error(ty)),
            false => Ok(TypeSpelling::value(ty)),
        }
    }

    fn enumeration(&mut self, id: EnumId) -> Self::Output {
        let ty = enumeration(id, self.context)?;
        match self
            .context
            .enumeration(id)
            .is_some_and(|enumeration| enumeration.is_error_payload())
        {
            true => Ok(TypeSpelling::error(ty)),
            false => Ok(TypeSpelling::value(ty)),
        }
    }

    fn class(&mut self, id: ClassId) -> Self::Output {
        class(id, self.context).map(TypeSpelling::value)
    }

    fn callback(&mut self, id: CallbackId) -> Self::Output {
        callback(id, self.context).map(TypeSpelling::value)
    }

    fn custom(&mut self, id: CustomTypeId) -> Self::Output {
        self.context
            .custom_type_mapping(id)
            .map(|mapping| TypeFragment::new(mapping.target_type().as_str()))
            .map(TypeSpelling::value)
            .map(Ok)
            .unwrap_or_else(|| {
                self.context
                    .custom_type(id)
                    .ok_or(Error::UnexpectedBindingShape {
                        layer: "dart type",
                        shape: "missing custom type declaration",
                    })
                    .and_then(|declaration| {
                        Name::new(declaration.name())
                            .upper_camel()
                            .map(|name| TypeSpelling::value(TypeFragment::new(name.to_string())))
                    })
            })
    }

    fn builtin(&mut self, kind: BuiltinType) -> Self::Output {
        Ok(TypeSpelling::value(TypeFragment::new(match kind {
            BuiltinType::Duration => "Duration",
            BuiltinType::SystemTime => "DateTime",
            BuiltinType::Uuid => "$$BoltUUIDValue",
            BuiltinType::Url => "Uri",
        })))
    }

    fn optional(&mut self, inner: Self::Output) -> Self::Output {
        Ok(TypeSpelling::value(TypeFragment::new(format!(
            "{}?",
            inner?.into_value()
        ))))
    }

    fn sequence(&mut self, element: Self::Output) -> Self::Output {
        sequence_type(element?.into_value(), None).map(TypeSpelling::value)
    }

    fn tuple(&mut self, elements: Vec<Self::Output>) -> Self::Output {
        elements
            .into_iter()
            .collect::<Result<Vec<_>>>()
            .map(|elements| {
                TypeSpelling::value(TypeFragment::new(Syntax::record(
                    elements.into_iter().map(TypeSpelling::into_value),
                )))
            })
    }

    fn result(&mut self, ok: Self::Output, err: Self::Output) -> Self::Output {
        Ok(TypeSpelling::value(TypeFragment::new(format!(
            "$$BoltResult<{}, {}>",
            ok?.into_value(),
            err?.into_result_failure()?
        ))))
    }

    fn map(&mut self, key: Self::Output, value: Self::Output) -> Self::Output {
        Ok(TypeSpelling::value(TypeFragment::new(format!(
            "Map<{}, {}>",
            key?.into_value(),
            value?.into_value()
        ))))
    }
}

fn sequence_type(element: TypeFragment, primitive: Option<Primitive>) -> Result<TypeFragment> {
    Ok(TypeFragment::new(match primitive {
        Some(Primitive::Bool) => "$$BoltBoolList".to_owned(),
        Some(Primitive::I8) => "$$typed_data.Int8List".to_owned(),
        Some(Primitive::U8) => "$$typed_data.Uint8List".to_owned(),
        Some(Primitive::I16) => "$$typed_data.Int16List".to_owned(),
        Some(Primitive::U16) => "$$typed_data.Uint16List".to_owned(),
        Some(Primitive::I32) => "$$typed_data.Int32List".to_owned(),
        Some(Primitive::U32) => "$$typed_data.Uint32List".to_owned(),
        Some(Primitive::I64 | Primitive::ISize) => "$$typed_data.Int64List".to_owned(),
        Some(Primitive::U64 | Primitive::USize) => "$$typed_data.Uint64List".to_owned(),
        Some(Primitive::F32) => "$$typed_data.Float32List".to_owned(),
        Some(Primitive::F64) => "$$typed_data.Float64List".to_owned(),
        None => format!("List<{element}>"),
        Some(_) => return super::unsupported("unknown primitive sequence type"),
    }))
}
