use boltffi_binding::{
    CallbackId, EnumId, HandlePresence, HandleTarget, IncomingParam, IntoRust, Native, ParamDecl,
    ParamPlan, Receive, RecordId, TypeRef, native,
};

use crate::{
    bridge::{
        c::{Type, identifier::Identifier, syntax::TypeSyntax},
        python_cext::PythonCExtBridgeContract,
    },
    core::{Error, RenderContext, Result},
    target::python::{
        cpython::render::{callback, custom, enumeration, primitive, record},
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
        index: usize,
        parameter: &ParamDecl<Native, IntoRust>,
        bridge: &PythonCExtBridgeContract,
        context: &RenderContext<Native>,
    ) -> Result<Self> {
        match parameter.payload() {
            IncomingParam::Value(ParamPlan::Direct {
                ty: TypeRef::Primitive(primitive),
                receive: Receive::ByValue,
            }) => Self::from_primitive(index, parameter, primitive::Runtime::new(*primitive)),
            IncomingParam::Value(ParamPlan::Direct {
                ty: TypeRef::Record(record),
                receive: Receive::ByValue,
            }) => Self::from_record(index, parameter, *record, bridge, context),
            IncomingParam::Value(ParamPlan::Direct {
                ty: TypeRef::Enum(enumeration),
                receive: Receive::ByValue,
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
                )
            }
            IncomingParam::Value(ParamPlan::Handle {
                target: HandleTarget::Class(_),
                carrier,
                presence: HandlePresence::Required,
                receive: Receive::ByValue,
            }) => Self::from_handle(index, parameter, *carrier),
            IncomingParam::Value(ParamPlan::Handle {
                target: HandleTarget::Callback(callback),
                carrier: native::HandleCarrier::CallbackHandle,
                presence,
                receive: Receive::ByValue,
            }) => Self::from_callback(index, parameter, *callback, *presence, bridge, context),
            IncomingParam::Closure(_) => Err(Error::UnsupportedTarget {
                target: "python",
                shape: "closure parameter",
            }),
            IncomingParam::Value(ParamPlan::Direct { .. }) => Err(Error::UnsupportedTarget {
                target: "python",
                shape: "borrowed direct parameter",
            }),
            IncomingParam::Value(ParamPlan::Encoded { .. }) => Err(Error::UnsupportedTarget {
                target: "python",
                shape: "unsupported encoded parameter",
            }),
            IncomingParam::Value(
                ParamPlan::Handle { .. }
                | ParamPlan::ScalarOption { .. }
                | ParamPlan::DirectVec { .. },
            ) => Err(Error::UnsupportedTarget {
                target: "python",
                shape: "unsupported parameter",
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

    pub fn call_args(&self) -> Vec<String> {
        match &self.kind {
            Kind::Direct(_) => vec![self.name.clone()],
            Kind::Encoded(encoded) => vec![encoded.pointer.clone(), encoded.length.clone()],
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

    pub fn is_string(&self) -> bool {
        matches!(
            self.kind,
            Kind::Encoded(EncodedParam {
                value: Encoded::String,
                ..
            })
        )
    }

    pub fn is_bytes(&self) -> bool {
        matches!(
            self.kind,
            Kind::Encoded(EncodedParam {
                value: Encoded::Bytes,
                ..
            })
        )
    }

    pub fn wire_primitive(&self) -> Option<primitive::Runtime> {
        match self.kind {
            Kind::Encoded(EncodedParam {
                value: Encoded::Primitive(primitive),
                ..
            }) => Some(primitive),
            Kind::Direct(_) | Kind::Encoded(_) => None,
        }
    }

    pub fn c_type(&self) -> &str {
        match &self.kind {
            Kind::Direct(direct) => direct.c_type.as_str(),
            Kind::Encoded(_) => "",
        }
    }

    pub fn parser(&self) -> &str {
        match &self.kind {
            Kind::Direct(direct) => direct.parser.as_str(),
            Kind::Encoded(encoded) => encoded.parser.as_str(),
        }
    }

    pub fn wire(&self) -> &str {
        match &self.kind {
            Kind::Direct(_) => "",
            Kind::Encoded(encoded) => encoded.wire.as_str(),
        }
    }

    pub fn pointer(&self) -> &str {
        match &self.kind {
            Kind::Direct(_) => "",
            Kind::Encoded(encoded) => encoded.pointer.as_str(),
        }
    }

    pub fn length(&self) -> &str {
        match &self.kind {
            Kind::Direct(_) => "",
            Kind::Encoded(encoded) => encoded.length.as_str(),
        }
    }

    fn from_primitive(
        index: usize,
        parameter: &ParamDecl<Native, IntoRust>,
        primitive: primitive::Runtime,
    ) -> Result<Self> {
        let name = Identifier::escape(Name::new(parameter.name()).function())?.to_string();
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

    fn from_record(
        index: usize,
        parameter: &ParamDecl<Native, IntoRust>,
        record: RecordId,
        bridge: &PythonCExtBridgeContract,
        context: &RenderContext<Native>,
    ) -> Result<Self> {
        let symbols = record::Symbols::from_record_id(record, bridge, context)?;
        let name = Identifier::escape(Name::new(parameter.name()).function())?.to_string();
        Ok(Self {
            index,
            name,
            kind: Kind::Direct(Direct {
                c_type: symbols.c_type().to_owned(),
                parser: symbols.parser().to_owned(),
            }),
            primitive: None,
        })
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

    fn from_enum(
        index: usize,
        parameter: &ParamDecl<Native, IntoRust>,
        enumeration: EnumId,
        bridge: &PythonCExtBridgeContract,
        context: &RenderContext<Native>,
    ) -> Result<Self> {
        let symbols = enumeration::Symbols::from_enum_id(enumeration, bridge, context)?;
        let name = Identifier::escape(Name::new(parameter.name()).function())?.to_string();
        Ok(Self {
            index,
            name,
            kind: Kind::Direct(Direct {
                c_type: symbols.c_type().to_owned(),
                parser: symbols.parser().to_owned(),
            }),
            primitive: None,
        })
    }

    fn encoded(
        index: usize,
        parameter: &ParamDecl<Native, IntoRust>,
        receive: Receive,
        encoded: Encoded,
    ) -> Result<Self> {
        if matches!(receive, Receive::ByMutRef) {
            Err(Error::UnsupportedTarget {
                target: "python",
                shape: "mutable encoded parameter",
            })
        } else {
            let name = Identifier::escape(Name::new(parameter.name()).function())?.to_string();
            let wire = format!("{name}_wire");
            let pointer = format!("{name}_ptr");
            let length = format!("{name}_len");
            Ok(Self {
                index,
                name,
                kind: Kind::Encoded(EncodedParam {
                    value: encoded,
                    parser: encoded.parser()?,
                    wire,
                    pointer,
                    length,
                }),
                primitive: encoded.primitive(),
            })
        }
    }

    fn from_encoded_type(
        index: usize,
        parameter: &ParamDecl<Native, IntoRust>,
        receive: Receive,
        ty: &TypeRef,
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
            _ => Err(Error::UnsupportedTarget {
                target: "python",
                shape: "unsupported custom representation parameter",
            }),
        }
    }
}

enum Kind {
    Direct(Direct),
    Encoded(EncodedParam),
}

struct Direct {
    c_type: String,
    parser: String,
}

struct EncodedParam {
    value: Encoded,
    parser: String,
    wire: String,
    pointer: String,
    length: String,
}

#[derive(Clone, Copy)]
enum Encoded {
    String,
    Bytes,
    Primitive(primitive::Runtime),
}

impl Encoded {
    fn parser(self) -> Result<String> {
        match self {
            Self::String => Ok("boltffi_python_wire_string".to_owned()),
            Self::Bytes => Ok("boltffi_python_wire_bytes".to_owned()),
            Self::Primitive(primitive) => primitive.wire_encoder(),
        }
    }

    fn primitive(self) -> Option<primitive::Runtime> {
        match self {
            Self::Primitive(primitive) => Some(primitive),
            Self::String | Self::Bytes => None,
        }
    }
}
