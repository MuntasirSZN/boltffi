use boltffi_binding::{
    BinderId, BuiltinType, CallbackId, ClassId, CodecWrite, CustomTypeId, ElementCount, EnumDecl,
    EnumId, MapKind, Native, Op, Primitive, RecordId, ValueRef,
};

use crate::{
    core::{Error, RenderContext, Result},
    target::swift::{
        SwiftHost,
        codec::value::{ValueExpression, ValueScope},
        primitive::SwiftPrimitive,
        syntax::{ArgumentList, Expression, Identifier, Statement},
    },
};

pub struct Writer<'context, 'bindings> {
    name: Identifier,
    scope: ValueScope,
    context: &'context RenderContext<'bindings, Native>,
}

impl<'context, 'bindings> Writer<'context, 'bindings> {
    pub fn new(
        name: Identifier,
        scope: impl Into<ValueScope>,
        context: &'context RenderContext<'bindings, Native>,
    ) -> Self {
        Self {
            name,
            scope: scope.into(),
            context,
        }
    }

    fn unsupported<T>(&self, shape: &'static str) -> Result<T> {
        Err(SwiftHost::unsupported(shape))
    }

    fn write(&self, method: &str, value: &ValueRef) -> Result<Statement> {
        Ok(Statement::expression(Expression::call(
            Expression::member(&self.name, method),
            [ValueExpression::new(value, self.scope.clone()).render()?]
                .into_iter()
                .collect::<ArgumentList>(),
        )))
    }

    fn write_encodable(&self, value: &ValueRef) -> Result<Statement> {
        ValueExpression::new(value, self.scope.clone())
            .render()
            .map(|value| {
                Statement::expression(Expression::call(
                    Expression::member(value, "encode"),
                    [Expression::labeled("to", Expression::address(&self.name))]
                        .into_iter()
                        .collect::<ArgumentList>(),
                ))
            })
    }

    fn value(&self, value: &ValueRef) -> Result<Expression> {
        ValueExpression::new(value, self.scope.clone()).render()
    }

    fn single_statement(statements: Vec<Result<Statement>>) -> Result<Statement> {
        let statements = statements.into_iter().collect::<Result<Vec<_>>>()?;
        match statements.as_slice() {
            [statement] => Ok(statement.clone()),
            _ => Err(SwiftHost::unsupported("multi-statement codec write")),
        }
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
                invariant: "missing enum type in Swift codec writer",
            }),
        }
    }
}

impl CodecWrite for Writer<'_, '_> {
    type Stmt = Result<Statement>;

    fn primitive(&mut self, primitive: Primitive, value: &ValueRef) -> Vec<Self::Stmt> {
        vec![self.value(value).and_then(|value| {
            SwiftPrimitive::new(primitive).write_statement(self.name.clone(), value)
        })]
    }

    fn string(&mut self, value: &ValueRef) -> Vec<Self::Stmt> {
        vec![self.write("writeString", value)]
    }

    fn bytes(&mut self, value: &ValueRef) -> Vec<Self::Stmt> {
        vec![self.write("writeBytes", value)]
    }

    fn direct_record(&mut self, _: RecordId, _: &ValueRef) -> Vec<Self::Stmt> {
        vec![self.unsupported("direct record codec write")]
    }

    fn encoded_record(&mut self, _: RecordId, value: &ValueRef) -> Vec<Self::Stmt> {
        vec![self.write_encodable(value)]
    }

    fn c_style_enum(&mut self, id: EnumId, value: &ValueRef) -> Vec<Self::Stmt> {
        vec![self.value(value).and_then(|value| {
            SwiftPrimitive::new(self.c_style_enum_repr(id)?)
                .write_statement(self.name.clone(), Expression::member(value, "rawValue"))
        })]
    }

    fn data_enum(&mut self, _: EnumId, value: &ValueRef) -> Vec<Self::Stmt> {
        vec![self.write_encodable(value)]
    }

    fn class_handle(&mut self, _: ClassId, _: &ValueRef) -> Vec<Self::Stmt> {
        vec![self.unsupported("class handle codec write")]
    }

    fn callback_handle(&mut self, _: CallbackId, _: &ValueRef) -> Vec<Self::Stmt> {
        vec![self.unsupported("callback handle codec write")]
    }

    fn custom<F>(&mut self, _: CustomTypeId, _: &ValueRef, _: F) -> Vec<Self::Stmt>
    where
        F: FnOnce(&mut Self, &ValueRef) -> Vec<Self::Stmt>,
    {
        vec![self.unsupported("custom codec write")]
    }

    fn builtin(&mut self, _: BuiltinType, _: &ValueRef) -> Vec<Self::Stmt> {
        vec![self.unsupported("builtin codec write")]
    }

    fn optional(
        &mut self,
        value: &ValueRef,
        binder: BinderId,
        inner: Vec<Self::Stmt>,
    ) -> Vec<Self::Stmt> {
        vec![self.value(value).and_then(|value| {
            Ok(Statement::expression(
                Expression::trailing_closure_parameters(
                    Expression::member(self.name.clone(), "writeOptional"),
                    [value].into_iter().collect::<ArgumentList>(),
                    [self.name.clone(), ValueExpression::binder(binder)?],
                    Self::single_statement(inner)?,
                ),
            ))
        })]
    }

    fn sequence(
        &mut self,
        value: &ValueRef,
        _: &Op<ElementCount>,
        binder: BinderId,
        element: Vec<Self::Stmt>,
    ) -> Vec<Self::Stmt> {
        vec![self.value(value).and_then(|value| {
            Ok(Statement::expression(
                Expression::trailing_closure_parameters(
                    Expression::member(self.name.clone(), "writeSequence"),
                    [value].into_iter().collect::<ArgumentList>(),
                    [self.name.clone(), ValueExpression::binder(binder)?],
                    Self::single_statement(element)?,
                ),
            ))
        })]
    }

    fn tuple(&mut self, _: &ValueRef, _: Vec<Vec<Self::Stmt>>) -> Vec<Self::Stmt> {
        vec![self.unsupported("tuple codec write")]
    }

    fn result(
        &mut self,
        _: &ValueRef,
        _: BinderId,
        _: Vec<Self::Stmt>,
        _: Vec<Self::Stmt>,
    ) -> Vec<Self::Stmt> {
        vec![self.unsupported("result codec write")]
    }

    fn map(
        &mut self,
        _: MapKind,
        _: &ValueRef,
        _: BinderId,
        _: Vec<Self::Stmt>,
        _: BinderId,
        _: Vec<Self::Stmt>,
    ) -> Vec<Self::Stmt> {
        vec![self.unsupported("map codec write")]
    }
}
