use boltffi_binding::{
    BuiltinType, CallbackId, ClassId, CodecRead, CustomTypeId, ElementCount, EnumId, MapKind,
    Native, Op, Primitive, RecordId,
};

use crate::core::{Error, RenderContext, Result};

use super::super::{syntax::Syntax, type_name};
use super::{CStyleEnumRepresentation, primitive_read_method};

pub struct Reader<'context, 'bindings> {
    name: String,
    context: &'context RenderContext<'bindings, Native>,
}

pub struct ReadExpression {
    source: String,
    value: ReadValue,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ReadValue {
    String,
    Other,
}

impl<'context, 'bindings> Reader<'context, 'bindings> {
    pub fn new(
        name: impl Into<String>,
        context: &'context RenderContext<'bindings, Native>,
    ) -> Self {
        Self {
            name: name.into(),
            context,
        }
    }

    fn record_name(&self, id: RecordId) -> Result<String> {
        type_name::type_ref(&boltffi_binding::TypeRef::Record(id), self.context)
            .map(|ty| ty.to_string())
    }

    fn enum_name(&self, id: EnumId) -> Result<String> {
        type_name::type_ref(&boltffi_binding::TypeRef::Enum(id), self.context)
            .map(|ty| ty.to_string())
    }
}

impl ReadExpression {
    pub fn into_source(self) -> String {
        self.source
    }

    fn new(source: impl Into<String>) -> Self {
        Self {
            source: source.into(),
            value: ReadValue::Other,
        }
    }

    fn string(source: impl Into<String>) -> Self {
        Self {
            source: source.into(),
            value: ReadValue::String,
        }
    }

    fn result_error(self) -> String {
        match self.value {
            ReadValue::String => format!("$$BoltException({})", self.source),
            ReadValue::Other => self.source,
        }
    }
}

impl CodecRead for Reader<'_, '_> {
    type Expr = Result<ReadExpression>;

    fn primitive(&mut self, primitive: Primitive) -> Self::Expr {
        Ok(ReadExpression::new(format!(
            "{}.{}()",
            self.name,
            primitive_read_method(primitive)
        )))
    }

    fn string(&mut self) -> Self::Expr {
        Ok(ReadExpression::string(format!(
            "{}.readString()",
            self.name
        )))
    }

    fn interned_string(&mut self, _: &[String]) -> Self::Expr {
        unreachable!("InternedString codec read reached Dart renderer without host capability")
    }

    fn bytes(&mut self) -> Self::Expr {
        Ok(ReadExpression::new(format!(
            "{}.readUint8List()",
            self.name
        )))
    }

    fn direct_record(&mut self, id: RecordId) -> Self::Expr {
        Ok(ReadExpression::new(format!(
            "{}._m$wireDecode({})",
            self.record_name(id)?,
            self.name
        )))
    }

    fn encoded_record(&mut self, id: RecordId) -> Self::Expr {
        self.direct_record(id)
    }

    fn c_style_enum(&mut self, id: EnumId) -> Self::Expr {
        let representation = CStyleEnumRepresentation::resolve(id, self.context)?;
        Ok(ReadExpression::new(format!(
            "{}._m$fromDiscriminant({}.{}())",
            self.enum_name(id)?,
            self.name,
            representation.read_method()
        )))
    }

    fn data_enum(&mut self, id: EnumId) -> Self::Expr {
        Ok(ReadExpression::new(format!(
            "{}._m$wireDecode({})",
            self.enum_name(id)?,
            self.name
        )))
    }

    fn class_handle(&mut self, _: ClassId) -> Self::Expr {
        Err(Error::UnsupportedTarget {
            target: "dart",
            shape: "class handle in encoded payload",
        })
    }

    fn callback_handle(&mut self, _: CallbackId) -> Self::Expr {
        Err(Error::UnsupportedTarget {
            target: "dart",
            shape: "callback handle in encoded payload",
        })
    }

    fn custom(&mut self, _: CustomTypeId, representation: Self::Expr) -> Self::Expr {
        representation.map(|representation| ReadExpression::new(representation.source))
    }

    fn builtin(&mut self, kind: BuiltinType) -> Self::Expr {
        Ok(ReadExpression::new(format!(
            "{}.{}()",
            self.name,
            match kind {
                BuiltinType::Duration => "readDuration",
                BuiltinType::SystemTime => "readInstant",
                BuiltinType::Uuid => "readUUID",
                BuiltinType::Url => "readUri",
            }
        )))
    }

    fn optional(&mut self, inner: Self::Expr) -> Self::Expr {
        Ok(ReadExpression::new(format!(
            "{}.readU8() == 0 ? null : {}",
            self.name, inner?.source
        )))
    }

    fn sequence(&mut self, _: &Op<ElementCount>, element: Self::Expr) -> Self::Expr {
        Ok(ReadExpression::new(format!(
            "{}.readList((_l$reader) => {})",
            self.name,
            element?.source.replace(&self.name, "_l$reader")
        )))
    }

    fn tuple(&mut self, elements: Vec<Self::Expr>) -> Self::Expr {
        elements
            .into_iter()
            .collect::<Result<Vec<_>>>()
            .map(|elements| {
                ReadExpression::new(Syntax::record(
                    elements.into_iter().map(ReadExpression::into_source),
                ))
            })
    }

    fn result(&mut self, ok: Self::Expr, err: Self::Expr) -> Self::Expr {
        Ok(ReadExpression::new(format!(
            "{}.readU8() == 0 ? $$BoltResult.ok({}) : $$BoltResult.err({})",
            self.name,
            ok?.source,
            err?.result_error()
        )))
    }

    fn map(&mut self, _: MapKind, key: Self::Expr, value: Self::Expr) -> Self::Expr {
        Ok(ReadExpression::new(format!(
            "{}.readMap((_l$reader) => {}, (_l$reader) => {})",
            self.name,
            key?.source.replace(&self.name, "_l$reader"),
            value?.source.replace(&self.name, "_l$reader")
        )))
    }
}
