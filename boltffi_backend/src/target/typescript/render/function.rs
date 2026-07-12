use askama::Template as AskamaTemplate;
use boltffi_binding::{
    DirectValueType, DirectVectorElementType, ErrorChannel, FunctionDecl, HandlePresence,
    HandleTarget, IntoRust, ParamPlanRender, Primitive, Receive, ReturnPlanRender, ReturnValueSlot,
    TypeRef, Wasm32, wasm32,
};

use crate::core::{Emitted, Error, RenderContext, Result};

use super::super::{
    codec::{ReadKind, Reader, SizeKind, Sizer, WriteKind, Writer},
    name_style::Name,
    render::{Type, direct_vector::DirectVector, scalar_option::ScalarOption},
    syntax::{ArgumentList, Expression, Identifier, Statement, TypeName},
};

#[derive(AskamaTemplate)]
#[template(path = "target/typescript/function.ts", escape = "none")]
pub struct Function {
    name: Identifier,
    parameters: Vec<Parameter>,
    returns: TypeName,
    body: Vec<Statement>,
}

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
struct Parameter {
    name: Identifier,
    ty: TypeName,
    setup: Vec<Statement>,
    arguments: Vec<Expression>,
    cleanup: Vec<Statement>,
}

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
struct Return {
    ty: TypeName,
    conversion: ReturnConversion,
}

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
enum ReturnConversion {
    Void,
    Direct,
    Boolean,
    String,
    Bytes,
    Encoded {
        reader: Identifier,
        decode: Expression,
    },
    DirectVector {
        take: Identifier,
    },
    ScalarOption {
        unpack: Identifier,
    },
}

struct ParameterRenderer<'context> {
    name: Identifier,
    context: &'context RenderContext<'context, Wasm32>,
}

struct ReturnRenderer<'context> {
    context: &'context RenderContext<'context, Wasm32>,
}

impl Function {
    pub fn from_declaration(
        declaration: &FunctionDecl<Wasm32>,
        context: &RenderContext<Wasm32>,
    ) -> Result<Self> {
        let callable = declaration.callable();
        if callable.execution().uses_async_execution() {
            return Err(Self::unsupported("asynchronous function"));
        }
        if !matches!(callable.error().channel(), ErrorChannel::None) {
            return Err(Self::unsupported("fallible function"));
        }
        let parameters = callable
            .params()
            .iter()
            .map(|parameter| {
                let name = Name::new(parameter.name()).identifier()?;
                parameter
                    .payload()
                    .as_value()
                    .ok_or_else(|| Self::unsupported("closure parameter"))?
                    .render_with(&mut ParameterRenderer { name, context })
            })
            .collect::<Result<Vec<_>>>()?;
        let returns = callable
            .returns()
            .plan()
            .render_with(&mut ReturnRenderer { context })?;
        let arguments = parameters
            .iter()
            .flat_map(|parameter| parameter.arguments.iter().cloned())
            .collect::<ArgumentList>();
        let call = Expression::native_call(
            Identifier::parse(declaration.symbol().name().as_str())?,
            arguments,
        );
        let call = returns.render(call);
        let setup = parameters
            .iter()
            .flat_map(|parameter| parameter.setup.iter().cloned());
        let cleanup = parameters
            .iter()
            .flat_map(|parameter| parameter.cleanup.iter().cloned())
            .collect::<Vec<_>>();
        let body = setup
            .chain(match cleanup.is_empty() {
                true => call,
                false => vec![Statement::try_finally(call, cleanup)],
            })
            .collect();
        Ok(Self {
            name: Name::new(declaration.name()).identifier()?,
            parameters,
            returns: returns.ty,
            body,
        })
    }

    pub fn render(&self) -> Result<Emitted> {
        Ok(Emitted::primary(AskamaTemplate::render(self)?))
    }

    fn unsupported(shape: &'static str) -> Error {
        Error::UnsupportedTarget {
            target: "typescript",
            shape,
        }
    }
}

impl Parameter {
    fn direct(name: Identifier, primitive: Primitive) -> Result<Self> {
        Ok(Self {
            ty: Type::primitive(primitive)?,
            arguments: vec![Expression::identifier(name.clone())],
            name,
            setup: Vec::new(),
            cleanup: Vec::new(),
        })
    }

    fn direct_enum(
        name: Identifier,
        id: boltffi_binding::EnumId,
        context: &RenderContext<Wasm32>,
    ) -> Result<Self> {
        let ty = context
            .enumeration(id)
            .map(|enumeration| TypeName::named(Name::new(enumeration.name()).type_name()))
            .ok_or_else(|| Function::unsupported("enum without declaration"))?;
        Ok(Self {
            ty,
            arguments: vec![Expression::identifier(name.clone())],
            name,
            setup: Vec::new(),
            cleanup: Vec::new(),
        })
    }

    fn encoded(
        name: Identifier,
        ty: &TypeRef,
        codec: &boltffi_binding::WritePlan,
        context: &RenderContext<Wasm32>,
    ) -> Result<Self> {
        let value = Expression::identifier(name.clone());
        let size = codec.size_with(&mut Sizer::new(value.clone(), context))?;
        let writer = Identifier::parse(format!("__boltffi_{name}_writer"))?;
        let writes = codec
            .render_with(&mut Writer::new(writer.clone(), value.clone(), context))
            .into_iter()
            .collect::<Result<Vec<_>>>()?;
        let allocation_method = match (size.kind(), writes.as_slice()) {
            (Some(SizeKind::String), [write])
                if matches!(write.kind(), Some(WriteKind::String)) =>
            {
                Some(Identifier::known("allocString"))
            }
            (Some(SizeKind::Bytes), [write]) if matches!(write.kind(), Some(WriteKind::Bytes)) => {
                Some(Identifier::known("allocBytes"))
            }
            _ => None,
        };
        let size = size.into_expression();
        let writes = writes
            .into_iter()
            .map(|write| write.into_statement())
            .collect::<Vec<_>>();
        let Some(allocation_method) = allocation_method else {
            let writer_value = Expression::identifier(writer.clone());
            return Ok(Self {
                ty: Type::from_ref(ty, context)?,
                setup: std::iter::once(Statement::constant(
                    writer.clone(),
                    Expression::call(
                        Expression::identifier(Identifier::known("_module")),
                        Identifier::known("allocWriter"),
                        [size].into_iter().collect::<ArgumentList>(),
                    ),
                ))
                .chain(writes)
                .collect(),
                arguments: ["ptr", "len"]
                    .into_iter()
                    .map(|property| {
                        Expression::property(writer_value.clone(), Identifier::known(property))
                    })
                    .collect(),
                cleanup: vec![Statement::expression(Expression::call(
                    Expression::identifier(Identifier::known("_module")),
                    Identifier::known("freeWriter"),
                    [writer_value].into_iter().collect::<ArgumentList>(),
                ))],
                name,
            });
        };
        let allocation = Identifier::parse(format!("__boltffi_{name}_allocation"))?;
        let allocation_value = Expression::identifier(allocation.clone());
        Ok(Self {
            ty: Type::from_ref(ty, context)?,
            setup: vec![Statement::constant(
                allocation.clone(),
                Expression::call(
                    Expression::identifier(Identifier::known("_module")),
                    allocation_method,
                    [value].into_iter().collect::<ArgumentList>(),
                ),
            )],
            arguments: ["ptr", "len"]
                .into_iter()
                .map(|property| {
                    Expression::property(allocation_value.clone(), Identifier::known(property))
                })
                .collect(),
            cleanup: vec![Statement::expression(Expression::call(
                Expression::identifier(Identifier::known("_module")),
                Identifier::known("freeAlloc"),
                [allocation_value].into_iter().collect::<ArgumentList>(),
            ))],
            name,
        })
    }

    fn direct_vector(
        name: Identifier,
        element: &DirectVectorElementType,
        receive: Receive,
    ) -> Result<Self> {
        let vector = DirectVector::new(element, receive)?;
        let allocation = Identifier::parse(format!("__boltffi_{name}_allocation"))?;
        let allocation_value = Expression::identifier(allocation.clone());
        let value = Expression::identifier(name.clone());
        let mut cleanup = match vector.writeback() {
            true => vec![Statement::expression(Expression::call(
                Expression::identifier(Identifier::known("_module")),
                Identifier::known("copyPrimitiveBufferInto"),
                [
                    allocation_value.clone(),
                    value.clone(),
                    Expression::string(vector.element_literal()),
                ]
                .into_iter()
                .collect::<ArgumentList>(),
            ))],
            false => Vec::new(),
        };
        cleanup.push(Statement::expression(Expression::call(
            Expression::identifier(Identifier::known("_module")),
            Identifier::known("freePrimitiveBuffer"),
            [allocation_value.clone()]
                .into_iter()
                .collect::<ArgumentList>(),
        )));
        Ok(Self {
            ty: vector.parameter_type()?,
            setup: vec![Statement::constant(
                allocation.clone(),
                Expression::call(
                    Expression::identifier(Identifier::known("_module")),
                    vector.allocation_method(),
                    [value].into_iter().collect::<ArgumentList>(),
                ),
            )],
            arguments: ["ptr", "len"]
                .into_iter()
                .map(|property| {
                    Expression::property(allocation_value.clone(), Identifier::known(property))
                })
                .collect(),
            cleanup,
            name,
        })
    }

    fn scalar_option(name: Identifier, primitive: Primitive) -> Result<Self> {
        let option = ScalarOption::new(primitive)?;
        Ok(Self {
            ty: option.ty()?,
            arguments: vec![option.argument(Expression::identifier(name.clone()))],
            name,
            setup: Vec::new(),
            cleanup: Vec::new(),
        })
    }
}

impl Return {
    fn render(&self, call: Expression) -> Vec<Statement> {
        match &self.conversion {
            ReturnConversion::Void => vec![Statement::expression(call)],
            ReturnConversion::Direct => vec![Statement::return_value(call)],
            ReturnConversion::Boolean => vec![Statement::return_value(call.not_zero())],
            ReturnConversion::String => vec![Statement::return_value(Expression::call(
                Expression::identifier(Identifier::known("_module")),
                Identifier::known("takePackedUtf8String"),
                [call.cast(TypeName::bigint())]
                    .into_iter()
                    .collect::<ArgumentList>(),
            ))],
            ReturnConversion::Bytes => vec![Statement::return_value(Expression::call(
                Expression::identifier(Identifier::known("_module")),
                Identifier::known("takePackedU8Array"),
                [call.cast(TypeName::bigint())]
                    .into_iter()
                    .collect::<ArgumentList>(),
            ))],
            ReturnConversion::Encoded { reader, decode } => vec![
                Statement::constant(
                    reader.clone(),
                    Expression::call(
                        Expression::identifier(Identifier::known("_module")),
                        Identifier::known("takePackedBuffer"),
                        [call.cast(TypeName::bigint())]
                            .into_iter()
                            .collect::<ArgumentList>(),
                    ),
                ),
                Statement::return_value(decode.clone()),
            ],
            ReturnConversion::DirectVector { take } => vec![
                Statement::expression(call),
                Statement::return_value(Expression::call(
                    Expression::identifier(Identifier::known("_module")),
                    take.clone(),
                    ArgumentList::default(),
                )),
            ],
            ReturnConversion::ScalarOption { unpack } => {
                vec![Statement::return_value(Expression::call(
                    Expression::identifier(Identifier::known("_module")),
                    unpack.clone(),
                    [call].into_iter().collect::<ArgumentList>(),
                ))]
            }
        }
    }
}

impl<'plan> ParamPlanRender<'plan, Wasm32, IntoRust> for ParameterRenderer<'_> {
    type Output = Result<Parameter>;

    fn direct(&mut self, ty: &'plan DirectValueType, _receive: Receive) -> Self::Output {
        match ty {
            DirectValueType::Primitive(primitive) => {
                Parameter::direct(self.name.clone(), *primitive)
            }
            DirectValueType::Enum(id) => {
                Parameter::direct_enum(self.name.clone(), *id, self.context)
            }
            DirectValueType::Record(_) => Err(Function::unsupported("direct aggregate parameter")),
            _ => Err(Function::unsupported("unknown direct parameter")),
        }
    }

    fn encoded(
        &mut self,
        ty: &'plan TypeRef,
        codec: &'plan boltffi_binding::WritePlan,
        shape: wasm32::BufferShape,
        _receive: Receive,
    ) -> Self::Output {
        match shape {
            wasm32::BufferShape::Slice => {
                Parameter::encoded(self.name.clone(), ty, codec, self.context)
            }
            wasm32::BufferShape::Packed => Err(Function::unsupported("packed encoded parameter")),
            _ => Err(Function::unsupported("unknown encoded parameter shape")),
        }
    }

    fn handle(
        &mut self,
        _target: &'plan HandleTarget,
        _carrier: wasm32::HandleCarrier,
        _presence: HandlePresence,
        _receive: Receive,
    ) -> Self::Output {
        Err(Function::unsupported("handle parameter"))
    }

    fn scalar_option(&mut self, primitive: Primitive) -> Self::Output {
        Parameter::scalar_option(self.name.clone(), primitive)
    }

    fn direct_vector(
        &mut self,
        element: &'plan DirectVectorElementType,
        receive: Receive,
    ) -> Self::Output {
        Parameter::direct_vector(self.name.clone(), element, receive)
    }
}

impl<'plan> ReturnPlanRender<'plan, Wasm32, boltffi_binding::OutOfRust> for ReturnRenderer<'_> {
    type Output = Result<Return>;

    fn void(&mut self) -> Self::Output {
        Ok(Return {
            ty: TypeName::void(),
            conversion: ReturnConversion::Void,
        })
    }

    fn direct(&mut self, slot: ReturnValueSlot, ty: &'plan DirectValueType) -> Self::Output {
        if !matches!(slot, ReturnValueSlot::ReturnSlot) {
            return Err(Function::unsupported("direct out-pointer return"));
        }
        match ty {
            DirectValueType::Primitive(primitive) => Ok(Return {
                ty: Type::primitive(*primitive)?,
                conversion: match primitive {
                    Primitive::Bool => ReturnConversion::Boolean,
                    _ => ReturnConversion::Direct,
                },
            }),
            DirectValueType::Enum(id) => Ok(Return {
                ty: self
                    .context
                    .enumeration(*id)
                    .map(|enumeration| Name::new(enumeration.name()).type_name())
                    .ok_or_else(|| Function::unsupported("enum without declaration"))?,
                conversion: ReturnConversion::Direct,
            }),
            DirectValueType::Record(_) => Err(Function::unsupported("direct aggregate return")),
            _ => Err(Function::unsupported("unknown direct return")),
        }
    }

    fn encoded(
        &mut self,
        slot: ReturnValueSlot,
        ty: &'plan TypeRef,
        codec: &'plan boltffi_binding::ReadPlan,
        shape: wasm32::BufferShape,
    ) -> Self::Output {
        if !matches!(slot, ReturnValueSlot::ReturnSlot)
            || !matches!(shape, wasm32::BufferShape::Packed)
        {
            return Err(Function::unsupported("encoded return placement"));
        }
        let reader = Identifier::known("__boltffiReader");
        let read = codec.render_with(&mut Reader::new(reader.clone(), self.context))?;
        let kind = read.kind();
        let decode = read.into_expression();
        match kind {
            Some(ReadKind::String) => Ok(Return {
                ty: Type::from_ref(ty, self.context)?,
                conversion: ReturnConversion::String,
            }),
            Some(ReadKind::Bytes) => Ok(Return {
                ty: Type::from_ref(ty, self.context)?,
                conversion: ReturnConversion::Bytes,
            }),
            Some(ReadKind::Primitive(_)) | None => Ok(Return {
                ty: Type::from_ref(ty, self.context)?,
                conversion: ReturnConversion::Encoded { reader, decode },
            }),
        }
    }

    fn handle(
        &mut self,
        _slot: ReturnValueSlot,
        _target: &'plan HandleTarget,
        _carrier: wasm32::HandleCarrier,
        _presence: HandlePresence,
    ) -> Self::Output {
        Err(Function::unsupported("handle return"))
    }

    fn scalar_option(&mut self, primitive: Primitive) -> Self::Output {
        let option = ScalarOption::new(primitive)?;
        Ok(Return {
            ty: option.ty()?,
            conversion: ReturnConversion::ScalarOption {
                unpack: option.unpack_method(),
            },
        })
    }

    fn direct_vector(&mut self, element: &'plan DirectVectorElementType) -> Self::Output {
        let vector = DirectVector::new(element, Receive::ByValue)?;
        Ok(Return {
            ty: vector.return_type(),
            conversion: ReturnConversion::DirectVector {
                take: vector.take_method(),
            },
        })
    }

    fn closure(
        &mut self,
        _closure: &'plan boltffi_binding::ClosureReturn<Wasm32, boltffi_binding::OutOfRust>,
    ) -> Self::Output {
        Err(Function::unsupported("closure return"))
    }
}
