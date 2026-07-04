use boltffi_binding::{
    BuiltinType, CallbackId, ClassId, CodecRead, CustomTypeId, ElementCount, EnumDecl, EnumId,
    MapKind, Native, Op, Primitive, RecordId,
};

use crate::{
    core::{Error, RenderContext, Result},
    target::swift::{
        SwiftHost,
        primitive::SwiftPrimitive,
        render::SwiftType,
        syntax::{ArgumentList, Expression, Identifier},
    },
};

pub struct Reader<'context, 'bindings> {
    name: Identifier,
    context: &'context RenderContext<'bindings, Native>,
}

impl<'context, 'bindings> Reader<'context, 'bindings> {
    pub fn new(name: Identifier, context: &'context RenderContext<'bindings, Native>) -> Self {
        Self { name, context }
    }

    fn unsupported<T>(&self, shape: &'static str) -> Result<T> {
        Err(SwiftHost::unsupported(shape))
    }

    fn read(&self, method: &str) -> Expression {
        Expression::call(
            Expression::member(&self.name, method),
            ArgumentList::default(),
        )
    }

    fn c_style_enum_repr(&self, id: EnumId) -> Result<Primitive> {
        match self.context.enumeration(id) {
            Some(EnumDecl::CStyle(enumeration)) => Ok(enumeration.repr().primitive()),
            Some(EnumDecl::Data(_)) => {
                self.unsupported("data enum where C-style enum was expected")
            }
            Some(_) => self.unsupported("unknown enum where C-style enum was expected"),
            None => Err(Error::BrokenBridgeContract {
                bridge: SwiftHost::TARGET,
                invariant: "missing enum type in Swift codec reader",
            }),
        }
    }
}

impl<'context, 'bindings> CodecRead for Reader<'context, 'bindings> {
    type Expr = Result<Expression>;

    fn primitive(&mut self, primitive: Primitive) -> Self::Expr {
        SwiftPrimitive::new(primitive).read_expression(self.name.clone())
    }

    fn string(&mut self) -> Self::Expr {
        Ok(self.read("readString"))
    }

    fn bytes(&mut self) -> Self::Expr {
        Ok(self.read("readBytes"))
    }

    fn direct_record(&mut self, _: RecordId) -> Self::Expr {
        self.unsupported("direct record codec read")
    }

    fn encoded_record(&mut self, id: RecordId) -> Self::Expr {
        Ok(Expression::call(
            Expression::member(SwiftType::record(id, self.context)?, "decode"),
            [Expression::labeled("from", Expression::address(&self.name))]
                .into_iter()
                .collect::<ArgumentList>(),
        ))
    }

    fn c_style_enum(&mut self, id: EnumId) -> Self::Expr {
        Ok(Expression::forced(Expression::call(
            SwiftType::enumeration(id, self.context)?,
            [Expression::labeled(
                "rawValue",
                SwiftPrimitive::new(self.c_style_enum_repr(id)?)
                    .read_expression(self.name.clone())?,
            )]
            .into_iter()
            .collect::<ArgumentList>(),
        )))
    }

    fn data_enum(&mut self, id: EnumId) -> Self::Expr {
        Ok(Expression::call(
            Expression::member(SwiftType::enumeration(id, self.context)?, "decode"),
            [Expression::labeled("from", Expression::address(&self.name))]
                .into_iter()
                .collect::<ArgumentList>(),
        ))
    }

    fn class_handle(&mut self, _: ClassId) -> Self::Expr {
        self.unsupported("class handle codec read")
    }

    fn callback_handle(&mut self, _: CallbackId) -> Self::Expr {
        self.unsupported("callback handle codec read")
    }

    fn custom(&mut self, _: CustomTypeId, _: Self::Expr) -> Self::Expr {
        self.unsupported("custom codec read")
    }

    fn builtin(&mut self, _: BuiltinType) -> Self::Expr {
        self.unsupported("builtin codec read")
    }

    fn optional(&mut self, inner: Self::Expr) -> Self::Expr {
        Ok(Expression::trailing_closure(
            Expression::member(&self.name, "readOptional"),
            ArgumentList::default(),
            self.name.clone(),
            inner?,
        ))
    }

    fn sequence(&mut self, _: &Op<ElementCount>, element: Self::Expr) -> Self::Expr {
        Ok(Expression::trailing_closure(
            Expression::member(&self.name, "readSequence"),
            ArgumentList::default(),
            self.name.clone(),
            element?,
        ))
    }

    fn tuple(&mut self, _: Vec<Self::Expr>) -> Self::Expr {
        self.unsupported("tuple codec read")
    }

    fn result(&mut self, _: Self::Expr, _: Self::Expr) -> Self::Expr {
        self.unsupported("result codec read")
    }

    fn map(&mut self, _: MapKind, _: Self::Expr, _: Self::Expr) -> Self::Expr {
        self.unsupported("map codec read")
    }
}
