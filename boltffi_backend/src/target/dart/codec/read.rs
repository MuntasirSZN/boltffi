use boltffi_binding::{
    BuiltinType, CallbackId, ClassId, CodecRead, CustomTypeId, ElementCount, EnumDecl, EnumId,
    MapKind, Native, Op, Primitive, RecordId,
};

use crate::core::{Error, RenderContext, Result};

use super::super::type_name;
use super::primitive_read_method;

pub struct Reader<'context, 'bindings> {
    name: String,
    context: &'context RenderContext<'bindings, Native>,
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

    fn c_style_enum_repr(&self, id: EnumId) -> Result<Primitive> {
        match self.context.enumeration(id) {
            Some(EnumDecl::CStyle(enumeration)) => Ok(enumeration.repr().primitive()),
            Some(_) => Err(Error::UnsupportedTarget {
                target: "dart",
                shape: "data enum where a C-style enum was expected",
            }),
            None => Err(Error::BrokenBridgeContract {
                bridge: "dart",
                invariant: "missing enum in Dart codec reader",
            }),
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

impl CodecRead for Reader<'_, '_> {
    type Expr = Result<String>;

    fn primitive(&mut self, primitive: Primitive) -> Self::Expr {
        Ok(format!(
            "{}.{}()",
            self.name,
            primitive_read_method(primitive)
        ))
    }

    fn string(&mut self) -> Self::Expr {
        Ok(format!("{}.readString()", self.name))
    }

    fn interned_string(&mut self, static_values: &[String]) -> Self::Expr {
        let values = static_values
            .iter()
            .map(|value| format!("{value:?}"))
            .collect::<Vec<_>>()
            .join(", ");
        Ok(format!(
            "{}.readInternedString(const [{values}])",
            self.name
        ))
    }

    fn bytes(&mut self) -> Self::Expr {
        Ok(format!("{}.readUint8List()", self.name))
    }

    fn direct_record(&mut self, id: RecordId) -> Self::Expr {
        Ok(format!(
            "{}._m$wireDecode({})",
            self.record_name(id)?,
            self.name
        ))
    }

    fn encoded_record(&mut self, id: RecordId) -> Self::Expr {
        self.direct_record(id)
    }

    fn c_style_enum(&mut self, id: EnumId) -> Self::Expr {
        Ok(format!(
            "{}._m$fromDiscriminant({}.{}())",
            self.enum_name(id)?,
            self.name,
            primitive_read_method(self.c_style_enum_repr(id)?)
        ))
    }

    fn data_enum(&mut self, id: EnumId) -> Self::Expr {
        Ok(format!(
            "{}._m$wireDecode({})",
            self.enum_name(id)?,
            self.name
        ))
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
        representation
    }

    fn builtin(&mut self, kind: BuiltinType) -> Self::Expr {
        Ok(format!(
            "{}.{}()",
            self.name,
            match kind {
                BuiltinType::Duration => "readDuration",
                BuiltinType::SystemTime => "readInstant",
                BuiltinType::Uuid => "readUUID",
                BuiltinType::Url => "readUri",
            }
        ))
    }

    fn optional(&mut self, inner: Self::Expr) -> Self::Expr {
        Ok(format!("{}.readU8() == 0 ? null : {}", self.name, inner?))
    }

    fn sequence(&mut self, _: &Op<ElementCount>, element: Self::Expr) -> Self::Expr {
        Ok(format!(
            "{}.readList((_l$reader) => {})",
            self.name,
            element?.replace(&self.name, "_l$reader")
        ))
    }

    fn tuple(&mut self, elements: Vec<Self::Expr>) -> Self::Expr {
        Ok(format!(
            "({})",
            elements.into_iter().collect::<Result<Vec<_>>>()?.join(", ")
        ))
    }

    fn result(&mut self, ok: Self::Expr, err: Self::Expr) -> Self::Expr {
        Ok(format!(
            "{}.readU8() == 0 ? $$BoltResult.ok({}) : $$BoltResult.err({})",
            self.name, ok?, err?
        ))
    }

    fn map(&mut self, _: MapKind, key: Self::Expr, value: Self::Expr) -> Self::Expr {
        Ok(format!(
            "{}.readMap((_l$reader) => {}, (_l$reader) => {})",
            self.name,
            key?.replace(&self.name, "_l$reader"),
            value?.replace(&self.name, "_l$reader")
        ))
    }
}
