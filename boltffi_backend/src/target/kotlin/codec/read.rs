use boltffi_binding::{
    BuiltinType, CallbackId, ClassId, CodecRead, CustomTypeId, ElementCount, EnumId, MapKind, Op,
    Primitive, RecordId,
};

use crate::{
    core::{Error, Result},
    target::kotlin::syntax::{ArgumentList, Expression, Identifier},
};

pub struct Reader {
    reader: Identifier,
}

impl Reader {
    pub fn new(reader: Identifier) -> Self {
        Self { reader }
    }

    fn call(&self, method: &'static str) -> Result<Expression> {
        Ok(Expression::call(
            Expression::identifier(self.reader.clone()),
            Identifier::parse(method)?,
            ArgumentList::default(),
        ))
    }

    fn unsupported(shape: &'static str) -> Result<Expression> {
        Err(Error::UnsupportedTarget {
            target: "kotlin",
            shape,
        })
    }
}

impl CodecRead for Reader {
    type Expr = Result<Expression>;

    fn primitive(&mut self, primitive: Primitive) -> Self::Expr {
        match primitive {
            Primitive::Bool => self.call("readBool"),
            Primitive::I8 => self.call("readI8"),
            Primitive::U8 => self.call("readU8"),
            Primitive::I16 => self.call("readI16"),
            Primitive::U16 => self.call("readU16"),
            Primitive::I32 => self.call("readI32"),
            Primitive::U32 => self.call("readU32"),
            Primitive::I64 | Primitive::ISize => self.call("readI64"),
            Primitive::U64 | Primitive::USize => self.call("readU64"),
            Primitive::F32 => self.call("readF32"),
            Primitive::F64 => self.call("readF64"),
            _ => Self::unsupported("unknown primitive wire read"),
        }
    }

    fn string(&mut self) -> Self::Expr {
        self.call("readString")
    }

    fn bytes(&mut self) -> Self::Expr {
        self.call("readBytes")
    }

    fn direct_record(&mut self, _id: RecordId) -> Self::Expr {
        Self::unsupported("direct-record wire read")
    }

    fn encoded_record(&mut self, _id: RecordId) -> Self::Expr {
        Self::unsupported("encoded-record wire read")
    }

    fn c_style_enum(&mut self, _id: EnumId) -> Self::Expr {
        Self::unsupported("c-style enum wire read")
    }

    fn data_enum(&mut self, _id: EnumId) -> Self::Expr {
        Self::unsupported("data enum wire read")
    }

    fn class_handle(&mut self, _id: ClassId) -> Self::Expr {
        Self::unsupported("class handle wire read")
    }

    fn callback_handle(&mut self, _id: CallbackId) -> Self::Expr {
        Self::unsupported("callback handle wire read")
    }

    fn custom(&mut self, _id: CustomTypeId, representation: Self::Expr) -> Self::Expr {
        representation
    }

    fn builtin(&mut self, _kind: BuiltinType) -> Self::Expr {
        Self::unsupported("builtin wire read")
    }

    fn optional(&mut self, _inner: Self::Expr) -> Self::Expr {
        Self::unsupported("optional wire read")
    }

    fn sequence(&mut self, _len: &Op<ElementCount>, _element: Self::Expr) -> Self::Expr {
        Self::unsupported("sequence wire read")
    }

    fn tuple(&mut self, _elements: Vec<Self::Expr>) -> Self::Expr {
        Self::unsupported("tuple wire read")
    }

    fn result(&mut self, _ok: Self::Expr, _err: Self::Expr) -> Self::Expr {
        Self::unsupported("result wire read")
    }

    fn map(&mut self, _kind: MapKind, _key: Self::Expr, _value: Self::Expr) -> Self::Expr {
        Self::unsupported("map wire read")
    }
}
