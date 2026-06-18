use boltffi_binding::{
    CallbackId, EnumId, HandlePresence, HandleTarget, IncomingParam, IntoRust, Native, ParamDecl,
    ParamPlan, Receive, RecordId, TypeRef, native,
};

use crate::{
    bridge::{
        c::{self, Type, identifier::Identifier, syntax::TypeSyntax},
        python_cext::PythonCExtBridgeContract,
    },
    core::{Error, RenderContext, Result},
    target::python::{
        cpython::render::{
            callback, closure, custom, direct_vector, enumeration, primitive, record,
        },
        name_style::Name,
    },
};

pub struct Conversion {
    index: usize,
    name: String,
    kind: Kind,
    primitive: Option<primitive::Runtime>,
}

impl Conversion {
    pub fn from_parameter(
        owner: &str,
        index: usize,
        parameter: &ParamDecl<Native, IntoRust>,
        c_parameters: &[c::Parameter],
        bridge: &PythonCExtBridgeContract,
        context: &RenderContext<Native>,
    ) -> Result<Self> {
        match parameter.payload() {
            IncomingParam::Value(ParamPlan::Direct {
                ty: TypeRef::Primitive(primitive),
                receive: Receive::ByValue | Receive::ByRef,
            }) => Self::from_primitive(index, parameter, primitive::Runtime::new(*primitive)),
            IncomingParam::Value(ParamPlan::Direct {
                ty: TypeRef::Record(record),
                receive: Receive::ByValue | Receive::ByRef,
            }) => Self::from_record(index, parameter, *record, bridge, context),
            IncomingParam::Value(ParamPlan::Direct {
                ty: TypeRef::Enum(enumeration),
                receive: Receive::ByValue | Receive::ByRef,
            }) => Self::from_enum(index, parameter, *enumeration, bridge, context),
            IncomingParam::Value(ParamPlan::Encoded {
                ty: TypeRef::String,
                shape: native::BufferShape::Slice,
                receive,
                ..
            }) => Self::encoded(index, parameter, *receive, Encoded::String),
            IncomingParam::Value(ParamPlan::Encoded {
                ty: TypeRef::Bytes,
                shape: native::BufferShape::Slice,
                receive,
                ..
            }) => Self::encoded(index, parameter, *receive, Encoded::Bytes),
            IncomingParam::Value(ParamPlan::Encoded {
                ty: TypeRef::Custom(custom_type),
                shape: native::BufferShape::Slice,
                receive,
                ..
            }) => {
                let custom_types = custom::CustomTypes::from_context(context);
                Self::from_encoded_type(
                    index,
                    parameter,
                    *receive,
                    custom_types.representation(*custom_type)?,
                    bridge,
                    context,
                )
            }
            IncomingParam::Value(ParamPlan::Encoded {
                ty: TypeRef::Record(record),
                shape: native::BufferShape::Slice,
                receive,
                ..
            }) => Self::from_encoded_record(index, parameter, *record, *receive, bridge, context),
            IncomingParam::Value(ParamPlan::Encoded {
                ty: TypeRef::Enum(enumeration),
                shape: native::BufferShape::Slice,
                receive,
                ..
            }) => Self::from_encoded_type(
                index,
                parameter,
                *receive,
                &TypeRef::Enum(*enumeration),
                bridge,
                context,
            ),
            IncomingParam::Value(ParamPlan::Encoded {
                ty,
                shape: native::BufferShape::Slice,
                receive,
                ..
            }) => Self::from_encoded_type(index, parameter, *receive, ty, bridge, context),
            IncomingParam::Value(ParamPlan::DirectVec {
                element: element @ (TypeRef::Primitive(_) | TypeRef::Record(_) | TypeRef::Enum(_)),
            }) => Self::encoded(
                index,
                parameter,
                Receive::ByValue,
                Encoded::DirectVector(direct_vector::Element::from_type_ref(
                    element, bridge, context,
                )?),
            ),
            IncomingParam::Value(ParamPlan::ScalarOption { primitive }) => Self::encoded(
                index,
                parameter,
                Receive::ByValue,
                Encoded::OptionalPrimitive(primitive::Runtime::new(*primitive)),
            ),
            IncomingParam::Value(ParamPlan::Handle {
                target: HandleTarget::Class(_),
                carrier,
                ..
            }) => Self::from_handle(index, parameter, *carrier),
            IncomingParam::Value(ParamPlan::Handle {
                target: HandleTarget::Callback(callback),
                carrier: native::HandleCarrier::CallbackHandle,
                presence,
                receive: Receive::ByValue,
            }) => Self::from_callback(index, parameter, *callback, *presence, bridge, context),
            IncomingParam::Closure(closure) => Self::from_closure(
                owner,
                index,
                parameter,
                closure,
                c_parameters,
                bridge,
                context,
            ),
            IncomingParam::Value(ParamPlan::Direct { .. }) => Err(Error::UnsupportedTarget {
                target: "python",
                shape: "borrowed direct parameter",
            }),
            IncomingParam::Value(ParamPlan::Encoded { .. }) => Err(Error::UnsupportedTarget {
                target: "python",
                shape: "unsupported encoded parameter",
            }),
            IncomingParam::Value(ParamPlan::Handle {
                target: HandleTarget::Callback(_),
                ..
            }) => Err(Error::UnsupportedTarget {
                target: "python",
                shape: "unsupported callback handle parameter",
            }),
            IncomingParam::Value(ParamPlan::Handle { .. }) => Err(Error::UnsupportedTarget {
                target: "python",
                shape: "unknown handle parameter",
            }),
            IncomingParam::Value(ParamPlan::DirectVec { .. }) => Err(Error::UnsupportedTarget {
                target: "python",
                shape: "unsupported direct vector parameter",
            }),
            IncomingParam::Value(_) => Err(Error::UnsupportedTarget {
                target: "python",
                shape: "unknown parameter plan",
            }),
        }
    }

    pub fn primitive(&self) -> Option<primitive::Runtime> {
        self.primitive
    }

    pub fn class_receiver(carrier: native::HandleCarrier) -> Result<Self> {
        Self::handle_with_name(0, "receiver", carrier)
    }

    pub fn direct_record_receiver(
        record: RecordId,
        bridge: &PythonCExtBridgeContract,
        context: &RenderContext<Native>,
    ) -> Result<Self> {
        let symbols = record::Symbols::from_record_id(record, bridge, context)?;
        Self::direct_with_name(
            0,
            "receiver",
            symbols.c_type()?.to_owned(),
            symbols.parser(),
        )
    }

    pub fn encoded_record_receiver(
        record: RecordId,
        receive: Receive,
        bridge: &PythonCExtBridgeContract,
        context: &RenderContext<Native>,
    ) -> Result<Self> {
        let symbols = record::Symbols::from_record_id(record, bridge, context)?;
        Self::encoded_with_name(
            0,
            "receiver",
            receive,
            Encoded::RegisteredType(RegisteredType::new(symbols.parser(), symbols.boxer())),
        )
    }

    pub fn c_style_enum_receiver(
        enumeration: EnumId,
        bridge: &PythonCExtBridgeContract,
        context: &RenderContext<Native>,
    ) -> Result<Self> {
        let symbols = enumeration::Symbols::from_enum_id(enumeration, bridge, context)?;
        Self::direct_with_name(
            0,
            "receiver",
            symbols.c_type()?.to_owned(),
            symbols.parser(),
        )
    }

    pub fn data_enum_receiver(
        enumeration: EnumId,
        receive: Receive,
        bridge: &PythonCExtBridgeContract,
        context: &RenderContext<Native>,
    ) -> Result<Self> {
        let symbols = enumeration::Symbols::from_enum_id(enumeration, bridge, context)?;
        Self::encoded_with_name(
            0,
            "receiver",
            receive,
            Encoded::RegisteredType(RegisteredType::new(
                symbols.parser(),
                symbols.owned_decoder(),
            )),
        )
    }

    pub fn call_args(&self) -> Vec<String> {
        match &self.kind {
            Kind::Direct(_) => vec![self.name.clone()],
            Kind::Encoded(encoded) => encoded.call_args(),
            Kind::Closure(closure) => closure.call_args().into_iter().collect(),
        }
    }

    pub fn c_arity(&self) -> usize {
        match &self.kind {
            Kind::Direct(_) => 1,
            Kind::Encoded(encoded) => encoded.c_arity(),
            Kind::Closure(_) => closure::Parameter::c_arity(),
        }
    }

    pub const fn index(&self) -> usize {
        self.index
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn is_direct(&self) -> bool {
        matches!(self.kind, Kind::Direct(_))
    }

    pub fn is_encoded(&self) -> bool {
        matches!(self.kind, Kind::Encoded(_))
    }

    pub fn is_closure(&self) -> bool {
        matches!(self.kind, Kind::Closure(_))
    }

    pub fn is_string(&self) -> bool {
        matches!(&self.kind, Kind::Encoded(encoded) if matches!(encoded.value, Encoded::String))
    }

    pub fn is_bytes(&self) -> bool {
        matches!(&self.kind, Kind::Encoded(encoded) if matches!(encoded.value, Encoded::Bytes))
    }

    pub fn is_raw_wire(&self) -> bool {
        matches!(&self.kind, Kind::Encoded(encoded) if matches!(encoded.value, Encoded::RawWire))
    }

    pub fn has_closure_string_argument(&self) -> bool {
        matches!(&self.kind, Kind::Closure(closure) if closure.has_string_argument())
    }

    pub fn has_closure_bytes_argument(&self) -> bool {
        matches!(&self.kind, Kind::Closure(closure) if closure.has_bytes_argument())
    }

    pub fn has_closure_raw_wire_argument(&self) -> bool {
        matches!(&self.kind, Kind::Closure(closure) if closure.has_raw_wire_argument())
    }

    pub fn wire_primitive(&self) -> Option<primitive::Runtime> {
        match &self.kind {
            Kind::Encoded(encoded) => match encoded.value {
                Encoded::Primitive(primitive) | Encoded::OptionalPrimitive(primitive) => {
                    Some(primitive)
                }
                Encoded::String
                | Encoded::Bytes
                | Encoded::RegisteredType(_)
                | Encoded::RawWire
                | Encoded::DirectVector(_) => None,
            },
            Kind::Closure(_) | Kind::Direct(_) => None,
        }
    }

    pub fn closure_primitives(&self) -> impl Iterator<Item = primitive::Runtime> + '_ {
        match &self.kind {
            Kind::Closure(closure) => EitherIter::left(closure.primitives()),
            Kind::Direct(_) | Kind::Encoded(_) => EitherIter::right(std::iter::empty()),
        }
    }

    pub fn closure_wire_primitives(&self) -> impl Iterator<Item = primitive::Runtime> + '_ {
        match &self.kind {
            Kind::Closure(closure) => EitherIter::left(closure.wire_primitives()),
            Kind::Direct(_) | Kind::Encoded(_) => EitherIter::right(std::iter::empty()),
        }
    }

    pub fn direct_vector_element(&self) -> Option<direct_vector::Element> {
        match &self.kind {
            Kind::Encoded(encoded) => match &encoded.value {
                Encoded::DirectVector(element) => Some(element.clone()),
                Encoded::String
                | Encoded::Bytes
                | Encoded::Primitive(_)
                | Encoded::OptionalPrimitive(_)
                | Encoded::RegisteredType(_)
                | Encoded::RawWire => None,
            },
            Kind::Closure(_) | Kind::Direct(_) => None,
        }
    }

    pub fn closure_direct_vector_elements(
        &self,
    ) -> impl Iterator<Item = direct_vector::Element> + '_ {
        match &self.kind {
            Kind::Closure(closure) => EitherIter::left(closure.direct_vector_elements()),
            Kind::Direct(_) | Kind::Encoded(_) => EitherIter::right(std::iter::empty()),
        }
    }

    pub fn c_type(&self) -> &str {
        match &self.kind {
            Kind::Direct(direct) => direct.c_type.as_str(),
            Kind::Encoded(_) | Kind::Closure(_) => "",
        }
    }

    pub fn parser(&self) -> &str {
        match &self.kind {
            Kind::Direct(direct) => direct.parser.as_str(),
            Kind::Encoded(encoded) => encoded.parser.as_str(),
            Kind::Closure(closure) => closure.parser(),
        }
    }

    pub fn wire(&self) -> &str {
        match &self.kind {
            Kind::Direct(_) => "",
            Kind::Encoded(encoded) => encoded.wire.as_str(),
            Kind::Closure(_) => "",
        }
    }

    pub fn pointer(&self) -> &str {
        match &self.kind {
            Kind::Direct(_) => "",
            Kind::Encoded(encoded) => encoded.pointer.as_str(),
            Kind::Closure(_) => "",
        }
    }

    pub fn length(&self) -> &str {
        match &self.kind {
            Kind::Direct(_) => "",
            Kind::Encoded(encoded) => encoded.length.as_str(),
            Kind::Closure(_) => "",
        }
    }

    pub fn has_mutation(&self) -> bool {
        matches!(&self.kind, Kind::Encoded(encoded) if encoded.mutation.is_some())
    }

    pub fn mutation_buffer(&self) -> &str {
        match &self.kind {
            Kind::Encoded(encoded) => encoded
                .mutation
                .as_ref()
                .map(MutationOutput::buffer)
                .unwrap_or(""),
            Kind::Direct(_) | Kind::Closure(_) => "",
        }
    }

    pub fn mutation(&self) -> Option<MutationOutput> {
        match &self.kind {
            Kind::Encoded(encoded) => encoded.mutation.clone(),
            Kind::Direct(_) | Kind::Closure(_) => None,
        }
    }

    pub fn closure_declaration(&self) -> &str {
        match &self.kind {
            Kind::Closure(closure) => closure.declaration(),
            Kind::Direct(_) | Kind::Encoded(_) => "",
        }
    }

    pub fn closure_call_declaration(&self) -> &str {
        match &self.kind {
            Kind::Closure(closure) => closure.call_declaration(),
            Kind::Direct(_) | Kind::Encoded(_) => "",
        }
    }

    pub fn closure_call(&self) -> &str {
        match &self.kind {
            Kind::Closure(closure) => closure.call(),
            Kind::Direct(_) | Kind::Encoded(_) => "",
        }
    }

    pub fn closure_context_declaration(&self) -> &str {
        match &self.kind {
            Kind::Closure(closure) => closure.context_declaration(),
            Kind::Direct(_) | Kind::Encoded(_) => "",
        }
    }

    pub fn closure_context(&self) -> &str {
        match &self.kind {
            Kind::Closure(closure) => closure.context(),
            Kind::Direct(_) | Kind::Encoded(_) => "",
        }
    }

    pub fn closure_release_declaration(&self) -> &str {
        match &self.kind {
            Kind::Closure(closure) => closure.release_declaration(),
            Kind::Direct(_) | Kind::Encoded(_) => "",
        }
    }

    pub fn closure_release(&self) -> &str {
        match &self.kind {
            Kind::Closure(closure) => closure.release(),
            Kind::Direct(_) | Kind::Encoded(_) => "",
        }
    }

    pub fn closure_release_needed(&self) -> &str {
        match &self.kind {
            Kind::Closure(closure) => closure.release_needed(),
            Kind::Direct(_) | Kind::Encoded(_) => "",
        }
    }

    fn from_primitive(
        index: usize,
        parameter: &ParamDecl<Native, IntoRust>,
        primitive: primitive::Runtime,
    ) -> Result<Self> {
        let name = Identifier::escape(Name::new(parameter.name()).function())?.to_string();
        Self::direct_primitive(index, name, primitive)
    }

    fn from_record(
        index: usize,
        parameter: &ParamDecl<Native, IntoRust>,
        record: RecordId,
        bridge: &PythonCExtBridgeContract,
        context: &RenderContext<Native>,
    ) -> Result<Self> {
        let symbols = record::Symbols::from_record_id(record, bridge, context)?;
        let name = Identifier::escape(Name::new(parameter.name()).function())?.to_string();
        Self::direct_with_name(index, name, symbols.c_type()?.to_owned(), symbols.parser())
    }

    fn from_encoded_record(
        index: usize,
        parameter: &ParamDecl<Native, IntoRust>,
        record: RecordId,
        receive: Receive,
        bridge: &PythonCExtBridgeContract,
        context: &RenderContext<Native>,
    ) -> Result<Self> {
        let symbols = record::Symbols::from_record_id(record, bridge, context)?;
        Self::encoded(
            index,
            parameter,
            receive,
            Encoded::RegisteredType(RegisteredType::new(symbols.parser(), symbols.boxer())),
        )
    }

    fn from_handle(
        index: usize,
        parameter: &ParamDecl<Native, IntoRust>,
        carrier: native::HandleCarrier,
    ) -> Result<Self> {
        let name = Identifier::escape(Name::new(parameter.name()).function())?.to_string();
        Self::handle_with_name(index, name, carrier)
    }

    fn handle_with_name(
        index: usize,
        name: impl Into<String>,
        carrier: native::HandleCarrier,
    ) -> Result<Self> {
        let name = name.into();
        let carrier = primitive::Runtime::native_handle(carrier)?;
        Ok(Self {
            index,
            name,
            kind: Kind::Direct(Direct {
                c_type: carrier.c_type()?.to_owned(),
                parser: carrier.parser()?.to_owned(),
            }),
            primitive: Some(carrier),
        })
    }

    fn from_callback(
        index: usize,
        parameter: &ParamDecl<Native, IntoRust>,
        callback: CallbackId,
        presence: HandlePresence,
        bridge: &PythonCExtBridgeContract,
        context: &RenderContext<Native>,
    ) -> Result<Self> {
        let symbols = callback::Symbols::from_callback_id(callback, bridge, context)?;
        let name = Identifier::escape(Name::new(parameter.name()).function())?.to_string();
        Ok(Self {
            index,
            name,
            kind: Kind::Direct(Direct {
                c_type: TypeSyntax::new(&Type::CallbackHandle).anonymous()?,
                parser: symbols.parser(presence).to_owned(),
            }),
            primitive: None,
        })
    }

    fn from_closure(
        owner: &str,
        index: usize,
        parameter: &ParamDecl<Native, IntoRust>,
        closure: &boltffi_binding::ClosureParameter<Native, IntoRust>,
        c_parameters: &[c::Parameter],
        bridge: &PythonCExtBridgeContract,
        context: &RenderContext<Native>,
    ) -> Result<Self> {
        let name = Identifier::escape(Name::new(parameter.name()).function())?.to_string();
        let closure = closure::Parameter::new(
            owner,
            index,
            name.clone(),
            closure,
            c_parameters,
            bridge,
            context,
        )?;
        Ok(Self {
            index,
            name,
            kind: Kind::Closure(Box::new(closure)),
            primitive: None,
        })
    }

    fn from_enum(
        index: usize,
        parameter: &ParamDecl<Native, IntoRust>,
        enumeration: EnumId,
        bridge: &PythonCExtBridgeContract,
        context: &RenderContext<Native>,
    ) -> Result<Self> {
        let symbols = enumeration::Symbols::from_enum_id(enumeration, bridge, context)?;
        let name = Identifier::escape(Name::new(parameter.name()).function())?.to_string();
        Self::direct_with_name(index, name, symbols.c_type()?.to_owned(), symbols.parser())
    }

    fn encoded(
        index: usize,
        parameter: &ParamDecl<Native, IntoRust>,
        receive: Receive,
        encoded: Encoded,
    ) -> Result<Self> {
        let name = Identifier::escape(Name::new(parameter.name()).function())?.to_string();
        Self::encoded_with_name(index, name, receive, encoded)
    }

    fn from_encoded_type(
        index: usize,
        parameter: &ParamDecl<Native, IntoRust>,
        receive: Receive,
        ty: &TypeRef,
        bridge: &PythonCExtBridgeContract,
        context: &RenderContext<Native>,
    ) -> Result<Self> {
        match ty {
            TypeRef::Primitive(primitive) => Self::encoded(
                index,
                parameter,
                receive,
                Encoded::Primitive(primitive::Runtime::new(*primitive)),
            ),
            TypeRef::String => Self::encoded(index, parameter, receive, Encoded::String),
            TypeRef::Bytes => Self::encoded(index, parameter, receive, Encoded::Bytes),
            TypeRef::Record(record) => {
                let symbols = record::Symbols::from_record_id(*record, bridge, context)?;
                Self::encoded(
                    index,
                    parameter,
                    receive,
                    Encoded::RegisteredType(RegisteredType::new(symbols.parser(), symbols.boxer())),
                )
            }
            TypeRef::Enum(enumeration) => {
                let symbols = enumeration::Symbols::from_enum_id(*enumeration, bridge, context)?;
                Self::encoded(
                    index,
                    parameter,
                    receive,
                    Encoded::RegisteredType(RegisteredType::new(
                        symbols.parser(),
                        symbols.owned_decoder(),
                    )),
                )
            }
            _ => Self::encoded(index, parameter, receive, Encoded::RawWire),
        }
    }

    fn direct_primitive(
        index: usize,
        name: impl Into<String>,
        primitive: primitive::Runtime,
    ) -> Result<Self> {
        let name = name.into();
        Ok(Self {
            index,
            name,
            kind: Kind::Direct(Direct {
                c_type: primitive.c_type()?,
                parser: primitive.parser()?.to_owned(),
            }),
            primitive: Some(primitive),
        })
    }

    fn direct_with_name(
        index: usize,
        name: impl Into<String>,
        c_type: String,
        parser: impl Into<String>,
    ) -> Result<Self> {
        Ok(Self {
            index,
            name: name.into(),
            kind: Kind::Direct(Direct {
                c_type,
                parser: parser.into(),
            }),
            primitive: None,
        })
    }

    fn encoded_with_name(
        index: usize,
        name: impl Into<String>,
        receive: Receive,
        encoded: Encoded,
    ) -> Result<Self> {
        let name = name.into();
        let wire = format!("{name}_wire");
        let pointer = format!("{name}_ptr");
        let length = format!("{name}_len");
        let mutation = match receive {
            Receive::ByMutRef => encoded.mutation_output(&name)?,
            Receive::ByValue | Receive::ByRef => None,
            _ => {
                return Err(Error::UnsupportedTarget {
                    target: "python",
                    shape: "unknown encoded parameter receive mode",
                });
            }
        };
        let parser = encoded.parser()?;
        let primitive = encoded.primitive();
        Ok(Self {
            index,
            name,
            kind: Kind::Encoded(Box::new(EncodedParam {
                value: encoded,
                parser,
                wire,
                pointer,
                length,
                mutation,
            })),
            primitive,
        })
    }
}

enum Kind {
    Direct(Direct),
    Encoded(Box<EncodedParam>),
    Closure(Box<closure::Parameter>),
}

struct Direct {
    c_type: String,
    parser: String,
}

enum EitherIter<Left, Right> {
    Left(Left),
    Right(Right),
}

impl<Left, Right> EitherIter<Left, Right> {
    fn left(left: Left) -> Self {
        Self::Left(left)
    }

    fn right(right: Right) -> Self {
        Self::Right(right)
    }
}

impl<Item, Left, Right> Iterator for EitherIter<Left, Right>
where
    Left: Iterator<Item = Item>,
    Right: Iterator<Item = Item>,
{
    type Item = Item;

    fn next(&mut self) -> Option<Self::Item> {
        match self {
            Self::Left(left) => left.next(),
            Self::Right(right) => right.next(),
        }
    }
}

struct EncodedParam {
    value: Encoded,
    parser: String,
    wire: String,
    pointer: String,
    length: String,
    mutation: Option<MutationOutput>,
}

impl EncodedParam {
    fn call_args(&self) -> Vec<String> {
        match &self.value {
            Encoded::DirectVector(element) => vec![
                format!("(const {} *){}", element.c_type(), self.pointer),
                self.length.clone(),
            ],
            Encoded::String
            | Encoded::Bytes
            | Encoded::Primitive(_)
            | Encoded::OptionalPrimitive(_)
            | Encoded::RegisteredType(_)
            | Encoded::RawWire => [self.pointer.clone(), self.length.clone()]
                .into_iter()
                .chain(
                    self.mutation
                        .iter()
                        .map(|mutation| format!("&{}", mutation.buffer())),
                )
                .collect(),
        }
    }

    fn c_arity(&self) -> usize {
        2 + usize::from(self.mutation.is_some())
    }
}

#[derive(Clone)]
enum Encoded {
    String,
    Bytes,
    Primitive(primitive::Runtime),
    OptionalPrimitive(primitive::Runtime),
    RegisteredType(RegisteredType),
    RawWire,
    DirectVector(direct_vector::Element),
}

impl Encoded {
    fn parser(&self) -> Result<String> {
        match self {
            Self::String => Ok("boltffi_python_wire_string".to_owned()),
            Self::Bytes => Ok("boltffi_python_wire_bytes".to_owned()),
            Self::Primitive(primitive) => primitive.wire_encoder(),
            Self::OptionalPrimitive(primitive) => primitive.optional_wire_encoder(),
            Self::RegisteredType(registered) => Ok(registered.parser.clone()),
            Self::RawWire => Ok("boltffi_python_wire_raw".to_owned()),
            Self::DirectVector(element) => Ok(element.vector_parser().to_owned()),
        }
    }

    fn mutation_output(&self, name: &str) -> Result<Option<MutationOutput>> {
        match self {
            Self::RegisteredType(registered) => Ok(Some(MutationOutput::new(
                format!("{name}_out"),
                registered.owned_decoder.clone(),
            ))),
            Self::String
            | Self::Bytes
            | Self::Primitive(_)
            | Self::OptionalPrimitive(_)
            | Self::RawWire
            | Self::DirectVector(_) => Err(Error::UnsupportedTarget {
                target: "python",
                shape: "mutable encoded parameter",
            }),
        }
    }

    fn primitive(&self) -> Option<primitive::Runtime> {
        match self {
            Self::Primitive(primitive) | Self::OptionalPrimitive(primitive) => Some(*primitive),
            Self::String
            | Self::Bytes
            | Self::RegisteredType(_)
            | Self::RawWire
            | Self::DirectVector(_) => None,
        }
    }
}

#[derive(Clone)]
struct RegisteredType {
    parser: String,
    owned_decoder: String,
}

impl RegisteredType {
    fn new(parser: impl Into<String>, owned_decoder: impl Into<String>) -> Self {
        Self {
            parser: parser.into(),
            owned_decoder: owned_decoder.into(),
        }
    }
}

#[derive(Clone)]
pub struct MutationOutput {
    buffer: String,
    decoder: String,
}

impl MutationOutput {
    fn new(buffer: impl Into<String>, decoder: impl Into<String>) -> Self {
        Self {
            buffer: buffer.into(),
            decoder: decoder.into(),
        }
    }

    pub fn buffer(&self) -> &str {
        &self.buffer
    }

    pub fn decoder(&self) -> &str {
        &self.decoder
    }
}
