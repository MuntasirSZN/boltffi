use boltffi_binding::{
    BinderId, BuiltinType, CallbackId, ClassId, CodecWrite, CustomTypeId, ElementCount, EnumId,
    MapKind, Op, Primitive, RecordId, ValueRef, WritePlan,
};

use crate::{
    core::{Error, Result},
    target::kotlin::{
        codec::{size::Sizer, value::ValueExpression},
        name_style::Name,
        primitive::KotlinPrimitive,
        syntax::{ArgumentList, Expression, Identifier, Statement},
    },
};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EncodedWrite {
    setup: Vec<Statement>,
    argument: Expression,
    cleanup: Vec<Statement>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WireBuffer {
    buffer: Identifier,
    writer: Identifier,
}

pub struct Writer {
    writer: Identifier,
}

impl EncodedWrite {
    pub fn new(setup: Vec<Statement>, argument: Expression, cleanup: Vec<Statement>) -> Self {
        Self {
            setup,
            argument,
            cleanup,
        }
    }

    pub fn into_parts(self) -> (Vec<Statement>, Expression, Vec<Statement>) {
        (self.setup, self.argument, self.cleanup)
    }
}

impl WireBuffer {
    pub fn new(name: &Name) -> Result<Self> {
        Ok(Self {
            buffer: name.generated("wire")?,
            writer: name.generated("writer")?,
        })
    }

    pub fn writer(&self) -> &Identifier {
        &self.writer
    }

    pub fn write(self, plan: &WritePlan) -> Result<EncodedWrite> {
        let size = plan.size_with(&mut Sizer)?;
        let writes = plan
            .render_with(&mut Writer::new(self.writer.clone()))
            .into_iter()
            .collect::<Result<Vec<_>>>()?;
        self.write_statements(size, writes)
    }

    pub fn write_statements(
        self,
        size: Expression,
        writes: Vec<Statement>,
    ) -> Result<EncodedWrite> {
        let setup = std::iter::once(Statement::value(
            self.buffer.clone(),
            Expression::call(
                "WireWriterPool",
                Identifier::parse("acquire")?,
                [size].into_iter().collect::<ArgumentList>(),
            ),
        ))
        .chain(std::iter::once(Statement::value(
            self.writer.clone(),
            Expression::property(
                Expression::identifier(self.buffer.clone()),
                Identifier::parse("writer")?,
            ),
        )))
        .chain(writes)
        .collect();
        let argument = Expression::call(
            Expression::identifier(self.buffer.clone()),
            Identifier::parse("bytes")?,
            ArgumentList::default(),
        );
        let cleanup = vec![Statement::expression(Expression::call(
            Expression::identifier(self.buffer),
            Identifier::parse("close")?,
            ArgumentList::default(),
        ))];
        Ok(EncodedWrite::new(setup, argument, cleanup))
    }
}

impl Writer {
    pub fn new(writer: Identifier) -> Self {
        Self { writer }
    }

    fn value(value: &ValueRef) -> Result<Expression> {
        ValueExpression::new(value).render()
    }

    fn writer_call(&self, method: Identifier, value: &ValueRef) -> Result<Statement> {
        Ok(Statement::expression(Expression::call(
            Expression::identifier(self.writer.clone()),
            method,
            [Self::value(value)?].into_iter().collect::<ArgumentList>(),
        )))
    }

    fn primitive_method(primitive: Primitive) -> Result<Identifier> {
        KotlinPrimitive::new(primitive)
            .wire_method_suffix()
            .and_then(|suffix| Identifier::parse(format!("write{suffix}")))
    }

    fn unsupported(shape: &'static str) -> Vec<Result<Statement>> {
        vec![Err(Error::UnsupportedTarget {
            target: "kotlin",
            shape,
        })]
    }
}

impl CodecWrite for Writer {
    type Stmt = Result<Statement>;

    fn primitive(&mut self, primitive: Primitive, value: &ValueRef) -> Vec<Self::Stmt> {
        vec![Self::primitive_method(primitive).and_then(|method| self.writer_call(method, value))]
    }

    fn string(&mut self, value: &ValueRef) -> Vec<Self::Stmt> {
        vec![Identifier::parse("writeString").and_then(|method| self.writer_call(method, value))]
    }

    fn bytes(&mut self, value: &ValueRef) -> Vec<Self::Stmt> {
        vec![Identifier::parse("writeBytes").and_then(|method| self.writer_call(method, value))]
    }

    fn direct_record(&mut self, _id: RecordId, _value: &ValueRef) -> Vec<Self::Stmt> {
        Self::unsupported("direct-record wire write")
    }

    fn encoded_record(&mut self, _id: RecordId, _value: &ValueRef) -> Vec<Self::Stmt> {
        Self::unsupported("encoded-record wire write")
    }

    fn c_style_enum(&mut self, _id: EnumId, _value: &ValueRef) -> Vec<Self::Stmt> {
        Self::unsupported("c-style enum wire write")
    }

    fn data_enum(&mut self, _id: EnumId, _value: &ValueRef) -> Vec<Self::Stmt> {
        Self::unsupported("data enum wire write")
    }

    fn class_handle(&mut self, _id: ClassId, _value: &ValueRef) -> Vec<Self::Stmt> {
        Self::unsupported("class handle wire write")
    }

    fn callback_handle(&mut self, _id: CallbackId, _value: &ValueRef) -> Vec<Self::Stmt> {
        Self::unsupported("callback handle wire write")
    }

    fn custom(
        &mut self,
        _id: CustomTypeId,
        _value: &ValueRef,
        representation: Vec<Self::Stmt>,
    ) -> Vec<Self::Stmt> {
        representation
    }

    fn builtin(&mut self, _kind: BuiltinType, _value: &ValueRef) -> Vec<Self::Stmt> {
        Self::unsupported("builtin wire write")
    }

    fn optional(
        &mut self,
        _value: &ValueRef,
        _binder: BinderId,
        _inner: Vec<Self::Stmt>,
    ) -> Vec<Self::Stmt> {
        Self::unsupported("optional wire write")
    }

    fn sequence(
        &mut self,
        _value: &ValueRef,
        _len: &Op<ElementCount>,
        _binder: BinderId,
        _element: Vec<Self::Stmt>,
    ) -> Vec<Self::Stmt> {
        Self::unsupported("sequence wire write")
    }

    fn tuple(&mut self, _value: &ValueRef, _elements: Vec<Vec<Self::Stmt>>) -> Vec<Self::Stmt> {
        Self::unsupported("tuple wire write")
    }

    fn result(
        &mut self,
        _value: &ValueRef,
        _binder: BinderId,
        _ok: Vec<Self::Stmt>,
        _err: Vec<Self::Stmt>,
    ) -> Vec<Self::Stmt> {
        Self::unsupported("result wire write")
    }

    fn map(
        &mut self,
        _kind: MapKind,
        _value: &ValueRef,
        _key_binder: BinderId,
        _key: Vec<Self::Stmt>,
        _value_binder: BinderId,
        _map_value: Vec<Self::Stmt>,
    ) -> Vec<Self::Stmt> {
        Self::unsupported("map wire write")
    }
}
