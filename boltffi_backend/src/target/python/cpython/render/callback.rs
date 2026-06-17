use askama::Template as AskamaTemplate;
use boltffi_binding::{
    CallbackDecl, CallbackId, DeclarationRef, ErrorDecl, ExecutionDecl, HandlePresence,
    ImportedMethodDecl, IntoRust, Native, OutOfRust, OutgoingParam, ParamDecl, ParamPlan,
    Primitive, ReturnPlan, TypeRef, VTableSlot, native,
};

use crate::{
    bridge::{
        c::{self, identifier::Identifier, syntax::TypeSyntax},
        python_cext::PythonCExtBridgeContract,
    },
    core::{Emitted, Error, RenderContext, Result},
    target::python::{
        cpython::render::{direct_vector, enumeration, primitive, record},
        name_style::Name,
    },
};

#[derive(AskamaTemplate)]
#[template(path = "target/python/callback.c", escape = "none")]
struct Template {
    vtable_type: String,
    vtable: String,
    register: String,
    register_storage: String,
    create_handle_storage: String,
    copy_buffer_storage: String,
    parser: String,
    optional_parser: String,
    free: String,
    clone: String,
    slots: Vec<Slot>,
    methods: Vec<Method>,
}

pub struct Callback {
    symbols: Symbols,
    vtable_type: String,
    register_storage: String,
    create_handle_storage: String,
    copy_buffer_storage: String,
    slots: Vec<Slot>,
    methods: Vec<Method>,
}

impl Callback {
    pub fn supports(declaration: &CallbackDecl<Native>, bridge: &PythonCExtBridgeContract) -> bool {
        bridge
            .source_callback(declaration.id())
            .is_some_and(|callback| {
                declaration
                    .protocol()
                    .vtable()
                    .methods()
                    .iter()
                    .all(|method| Method::supports(method, callback))
            })
    }

    pub fn from_declaration(
        declaration: &CallbackDecl<Native>,
        bridge: &PythonCExtBridgeContract,
        context: &RenderContext<Native>,
    ) -> Result<Self> {
        let c_callback =
            bridge
                .source_callback(declaration.id())
                .ok_or(Error::UnsupportedTarget {
                    target: "python",
                    shape: "callback without C bridge vtable",
                })?;
        let register = bridge
            .loaded_function(declaration.protocol().register())
            .ok_or(Error::UnsupportedTarget {
                target: "python",
                shape: "callback register symbol not loaded",
            })?;
        let create_handle = bridge
            .loaded_function(declaration.protocol().create_handle())
            .ok_or(Error::UnsupportedTarget {
                target: "python",
                shape: "callback handle constructor symbol not loaded",
            })?;
        let copy_buffer = bridge
            .functions()
            .iter()
            .find(|function| function.function().name() == "boltffi_buf_from_bytes")
            .ok_or(Error::UnsupportedTarget {
                target: "python",
                shape: "missing CPython buffer copy support symbol",
            })?;
        let symbols = Symbols::from_declaration(declaration)?;
        let methods = declaration
            .protocol()
            .vtable()
            .methods()
            .iter()
            .map(|method| Method::new(method, c_callback, &symbols, bridge, context))
            .collect::<Result<Vec<_>>>()?;
        let slots = std::iter::once(Slot::new(
            declaration.protocol().vtable().free_slot().as_str(),
            symbols.free(),
        ))
        .chain(std::iter::once(Slot::new(
            declaration.protocol().vtable().clone_slot().as_str(),
            symbols.clone(),
        )))
        .chain(
            methods
                .iter()
                .map(|method| Slot::new(method.slot.as_str(), method.function.as_str())),
        )
        .collect();
        Ok(Self {
            symbols,
            vtable_type: TypeSyntax::new(&c::Type::Named(c_callback.vtable().name().to_owned()))
                .anonymous()?,
            register_storage: register.storage_name().to_owned(),
            create_handle_storage: create_handle.storage_name().to_owned(),
            copy_buffer_storage: copy_buffer.storage_name().to_owned(),
            slots,
            methods,
        })
    }

    pub fn render(self) -> Result<Emitted> {
        let source = Template {
            vtable_type: self.vtable_type,
            vtable: self.symbols.vtable,
            register: self.symbols.register,
            register_storage: self.register_storage,
            create_handle_storage: self.create_handle_storage,
            copy_buffer_storage: self.copy_buffer_storage,
            parser: self.symbols.parser,
            optional_parser: self.symbols.optional_parser,
            free: self.symbols.free,
            clone: self.symbols.clone,
            slots: self.slots,
            methods: self.methods,
        }
        .render()?;
        Ok(Emitted::primary(source))
    }

    pub fn binding(&self) -> &str {
        &self.symbols.register
    }

    pub fn primitives(&self) -> impl Iterator<Item = primitive::Runtime> + '_ {
        self.methods.iter().flat_map(Method::primitives)
    }

    pub fn wire_primitives(&self) -> impl Iterator<Item = primitive::Runtime> + '_ {
        self.methods.iter().flat_map(Method::wire_primitives)
    }

    pub fn direct_vector_elements(&self) -> impl Iterator<Item = direct_vector::Element> + '_ {
        self.methods.iter().flat_map(Method::direct_vector_elements)
    }

    pub fn has_string_argument(&self) -> bool {
        self.methods.iter().any(Method::has_string_argument)
    }

    pub fn has_bytes_argument(&self) -> bool {
        self.methods.iter().any(Method::has_bytes_argument)
    }

    pub fn has_raw_wire_argument(&self) -> bool {
        self.methods.iter().any(Method::has_raw_wire_argument)
    }
}

pub struct Symbols {
    parser: String,
    optional_parser: String,
    vtable: String,
    register: String,
    free: String,
    clone: String,
    method_prefix: String,
}

impl Symbols {
    pub fn from_callback_id(
        callback_id: CallbackId,
        bridge: &PythonCExtBridgeContract,
        context: &RenderContext<Native>,
    ) -> Result<Self> {
        let callback = context
            .bindings()
            .decls()
            .iter()
            .find_map(|decl| match DeclarationRef::from(decl) {
                DeclarationRef::Callback(callback) if callback.id() == callback_id => {
                    Some(callback)
                }
                DeclarationRef::Record(_)
                | DeclarationRef::Enum(_)
                | DeclarationRef::Function(_)
                | DeclarationRef::Class(_)
                | DeclarationRef::Callback(_)
                | DeclarationRef::Stream(_)
                | DeclarationRef::Constant(_)
                | DeclarationRef::CustomType(_) => None,
            })
            .ok_or(Error::UnsupportedTarget {
                target: "python",
                shape: "callback id without declaration",
            })?;
        bridge
            .source_callback(callback_id)
            .ok_or(Error::UnsupportedTarget {
                target: "python",
                shape: "callback id without C bridge vtable",
            })?;
        Self::from_declaration(callback)
    }

    pub fn parser(&self, presence: HandlePresence) -> &str {
        match presence {
            HandlePresence::Required => &self.parser,
            HandlePresence::Nullable => &self.optional_parser,
            _ => &self.parser,
        }
    }

    fn from_declaration(callback: &CallbackDecl<Native>) -> Result<Self> {
        let stem = Identifier::escape(Name::new(callback.name()).function())?.to_string();
        let stem = format!("callback_{stem}");
        Ok(Self {
            parser: format!("boltffi_python_parse_{stem}"),
            optional_parser: format!("boltffi_python_parse_optional_{stem}"),
            vtable: format!("boltffi_python_{stem}_vtable"),
            register: format!("boltffi_python_bind_{stem}"),
            free: format!("boltffi_python_{stem}_free"),
            clone: format!("boltffi_python_{stem}_clone"),
            method_prefix: format!("boltffi_python_{stem}"),
        })
    }

    fn free(&self) -> &str {
        &self.free
    }

    fn clone(&self) -> &str {
        &self.clone
    }

    fn method(&self, name: &boltffi_binding::CanonicalName) -> Result<String> {
        Ok(format!(
            "{}_{}",
            self.method_prefix,
            Identifier::escape(Name::new(name).function())?
        ))
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct Slot {
    name: String,
    function: String,
}

impl Slot {
    fn new(name: impl Into<String>, function: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            function: function.into(),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct Method {
    slot: String,
    function: String,
    python_name: String,
    returns: MethodReturn,
    params: Vec<MethodParam>,
}

impl Method {
    fn supports(method: &ImportedMethodDecl<Native, VTableSlot>, c_callback: &c::Callback) -> bool {
        if matches!(
            method.callable().execution(),
            ExecutionDecl::Asynchronous(_)
        ) || !matches!(method.callable().error(), ErrorDecl::None(_))
        {
            return false;
        }
        let Some(c_field) = c_callback
            .vtable()
            .fields()
            .iter()
            .find(|field| field.name() == method.target().as_str())
        else {
            return false;
        };
        let Ok(signature) = MethodSignature::from_field(c_field) else {
            return false;
        };
        method
            .callable()
            .params()
            .iter()
            .map(MethodParam::arity)
            .collect::<Result<Vec<_>>>()
            .and_then(|arity| signature.value_params(&arity).map(|_| ()))
            .is_ok()
            && MethodReturn::supports(method.callable().returns().plan())
    }

    fn new(
        method: &ImportedMethodDecl<Native, VTableSlot>,
        c_callback: &c::Callback,
        symbols: &Symbols,
        bridge: &PythonCExtBridgeContract,
        context: &RenderContext<Native>,
    ) -> Result<Self> {
        if matches!(
            method.callable().execution(),
            ExecutionDecl::Asynchronous(_)
        ) {
            return Err(Error::UnsupportedTarget {
                target: "python",
                shape: "async callback method",
            });
        }
        if !matches!(method.callable().error(), ErrorDecl::None(_)) {
            return Err(Error::UnsupportedTarget {
                target: "python",
                shape: "fallible callback method",
            });
        }
        let c_field = c_callback
            .vtable()
            .fields()
            .iter()
            .find(|field| field.name() == method.target().as_str())
            .ok_or(Error::UnsupportedTarget {
                target: "python",
                shape: "callback method without C vtable slot",
            })?;
        let signature = MethodSignature::from_field(c_field)?;
        let arity = method
            .callable()
            .params()
            .iter()
            .map(MethodParam::arity)
            .collect::<Result<Vec<_>>>()?;
        let c_params = signature.value_params(&arity)?;
        let params = method
            .callable()
            .params()
            .iter()
            .zip(c_params)
            .map(|(parameter, c_types)| MethodParam::new(parameter, c_types, bridge, context))
            .collect::<Result<Vec<_>>>()?;
        Ok(Self {
            slot: method.target().as_str().to_owned(),
            function: symbols.method(method.name())?,
            python_name: Name::new(method.name()).function(),
            returns: MethodReturn::new(
                method.callable().returns().plan(),
                signature.returns(),
                bridge,
                context,
            )?,
            params,
        })
    }

    fn primitives(&self) -> impl Iterator<Item = primitive::Runtime> + '_ {
        self.params
            .iter()
            .filter_map(MethodParam::primitive)
            .chain(self.returns.primitive())
    }

    fn wire_primitives(&self) -> impl Iterator<Item = primitive::Runtime> + '_ {
        self.params
            .iter()
            .filter_map(MethodParam::wire_primitive)
            .chain(self.returns.wire_primitive())
    }

    fn direct_vector_elements(&self) -> impl Iterator<Item = direct_vector::Element> + '_ {
        self.params
            .iter()
            .filter_map(MethodParam::direct_vector_element)
            .chain(self.returns.direct_vector())
    }

    fn has_string_argument(&self) -> bool {
        self.params.iter().any(MethodParam::has_string) || self.returns.has_string()
    }

    fn has_bytes_argument(&self) -> bool {
        self.params.iter().any(MethodParam::has_bytes) || self.returns.has_bytes()
    }

    fn has_raw_wire_argument(&self) -> bool {
        self.params.iter().any(MethodParam::has_raw_wire) || self.returns.has_raw_wire()
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct MethodSignature<'field> {
    returns: &'field c::Type,
    params: &'field [c::Type],
}

impl<'field> MethodSignature<'field> {
    fn from_field(field: &'field c::Field) -> Result<Self> {
        match field.ty() {
            c::Type::FunctionPointer { returns, params } => Ok(Self { returns, params }),
            _ => Err(Error::UnsupportedTarget {
                target: "python",
                shape: "callback vtable slot is not a function pointer",
            }),
        }
    }

    fn returns(&self) -> &c::Type {
        self.returns
    }

    fn value_params(&self, arity: &[usize]) -> Result<Vec<&'field [c::Type]>> {
        let value_param_count = arity.iter().sum::<usize>();
        let value_start =
            self.params
                .len()
                .checked_sub(value_param_count)
                .ok_or(Error::UnsupportedTarget {
                    target: "python",
                    shape: "callback method parameter ABI mismatch",
                })?;
        if value_start == 0 {
            return Err(Error::UnsupportedTarget {
                target: "python",
                shape: "callback method handle ABI mismatch",
            });
        }
        Ok(arity
            .iter()
            .scan(value_start, |offset, count| {
                let start = *offset;
                *offset += *count;
                Some(&self.params[start..*offset])
            })
            .collect())
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct MethodParam {
    declarations: Vec<String>,
    name: String,
    object: String,
    expression: String,
    primitive: Option<primitive::Runtime>,
    wire_primitive: Option<primitive::Runtime>,
    direct_vector: Option<direct_vector::Element>,
    string: bool,
    bytes: bool,
    raw_wire: bool,
}

impl MethodParam {
    fn arity(parameter: &ParamDecl<Native, OutOfRust>) -> Result<usize> {
        match parameter.payload() {
            OutgoingParam::Value(ParamPlan::Direct { .. }) => Ok(1),
            OutgoingParam::Value(ParamPlan::Encoded {
                shape: native::BufferShape::Slice,
                ..
            })
            | OutgoingParam::Value(ParamPlan::DirectVec { .. }) => Ok(2),
            OutgoingParam::Value(ParamPlan::Handle { .. }) => Ok(1),
            OutgoingParam::Value(ParamPlan::Encoded { .. })
            | OutgoingParam::Value(ParamPlan::ScalarOption { .. }) => {
                Err(Error::UnsupportedTarget {
                    target: "python",
                    shape: "unsupported callback method parameter",
                })
            }
            OutgoingParam::Closure(_) | OutgoingParam::Value(_) => Err(Error::UnsupportedTarget {
                target: "python",
                shape: "unknown callback method parameter",
            }),
        }
    }

    fn new(
        parameter: &ParamDecl<Native, OutOfRust>,
        c_types: &[c::Type],
        bridge: &PythonCExtBridgeContract,
        context: &RenderContext<Native>,
    ) -> Result<Self> {
        let name = Identifier::escape(Name::new(parameter.name()).function())?.to_string();
        let object = format!("{name}_object");
        match parameter.payload() {
            OutgoingParam::Value(ParamPlan::Direct { ty, .. }) => {
                Self::direct(name, object, ty, c_types, bridge, context)
            }
            OutgoingParam::Value(ParamPlan::Encoded {
                ty,
                shape: native::BufferShape::Slice,
                ..
            }) => Self::encoded(name, object, ty, c_types, bridge, context),
            OutgoingParam::Value(ParamPlan::DirectVec { element }) => {
                Self::direct_vector_param(name, object, element, c_types, bridge, context)
            }
            OutgoingParam::Value(ParamPlan::Encoded { .. }) => Err(Error::UnsupportedTarget {
                target: "python",
                shape: "unsupported callback method encoded parameter",
            }),
            OutgoingParam::Value(ParamPlan::Handle { .. }) => Err(Error::UnsupportedTarget {
                target: "python",
                shape: "callback handle method parameter",
            }),
            OutgoingParam::Closure(_) | OutgoingParam::Value(_) => Err(Error::UnsupportedTarget {
                target: "python",
                shape: "unknown callback method parameter",
            }),
        }
    }

    fn primitive(&self) -> Option<primitive::Runtime> {
        self.primitive
    }

    fn wire_primitive(&self) -> Option<primitive::Runtime> {
        self.wire_primitive
    }

    fn direct_vector_element(&self) -> Option<direct_vector::Element> {
        self.direct_vector.clone()
    }

    fn has_string(&self) -> bool {
        self.string
    }

    fn has_bytes(&self) -> bool {
        self.bytes
    }

    fn has_raw_wire(&self) -> bool {
        self.raw_wire
    }

    fn direct(
        name: String,
        object: String,
        ty: &TypeRef,
        c_types: &[c::Type],
        bridge: &PythonCExtBridgeContract,
        context: &RenderContext<Native>,
    ) -> Result<Self> {
        if c_types.len() != 1 {
            return Err(Error::UnsupportedTarget {
                target: "python",
                shape: "callback direct parameter ABI mismatch",
            });
        }
        let (expression, primitive) = match ty {
            TypeRef::Primitive(primitive) => {
                let primitive = primitive::Runtime::new(*primitive);
                (format!("{}({name})", primitive.boxer()?), Some(primitive))
            }
            TypeRef::Record(record_id) => {
                let symbols = record::Symbols::from_record_id(*record_id, bridge, context)?;
                (format!("{}({name})", symbols.boxer()), None)
            }
            TypeRef::Enum(enum_id) => {
                let symbols = enumeration::Symbols::from_enum_id(*enum_id, bridge, context)?;
                (format!("{}({name})", symbols.boxer()), None)
            }
            _ => {
                return Err(Error::UnsupportedTarget {
                    target: "python",
                    shape: "unsupported direct callback parameter",
                });
            }
        };
        Ok(Self {
            declarations: vec![TypeSyntax::new(&c_types[0]).declaration(&name)?],
            name,
            object,
            expression,
            primitive,
            wire_primitive: None,
            direct_vector: None,
            string: false,
            bytes: false,
            raw_wire: false,
        })
    }

    fn encoded(
        name: String,
        object: String,
        ty: &TypeRef,
        c_types: &[c::Type],
        bridge: &PythonCExtBridgeContract,
        context: &RenderContext<Native>,
    ) -> Result<Self> {
        let [pointer, length] = c_types else {
            return Err(Error::UnsupportedTarget {
                target: "python",
                shape: "callback encoded parameter ABI mismatch",
            });
        };
        let pointer_name = format!("{name}_ptr");
        let length_name = format!("{name}_len");
        let (expression, wire_primitive, string, bytes, raw_wire) = match ty {
            TypeRef::String => (
                format!("boltffi_python_decode_borrowed_utf8({pointer_name}, {length_name})"),
                None,
                true,
                false,
                false,
            ),
            TypeRef::Bytes => (
                format!("boltffi_python_decode_borrowed_bytes({pointer_name}, {length_name})"),
                None,
                false,
                true,
                false,
            ),
            TypeRef::Record(record_id) => {
                let decoder = record::Symbols::from_record_id(*record_id, bridge, context)?
                    .borrowed_decoder()
                    .to_owned();
                (
                    format!("{decoder}({pointer_name}, {length_name})"),
                    None,
                    false,
                    false,
                    false,
                )
            }
            TypeRef::Enum(enum_id) => {
                let decoder = enumeration::Symbols::from_enum_id(*enum_id, bridge, context)?
                    .borrowed_decoder()
                    .to_owned();
                (
                    format!("{decoder}({pointer_name}, {length_name})"),
                    None,
                    false,
                    false,
                    false,
                )
            }
            TypeRef::Primitive(primitive) => {
                let primitive = primitive::Runtime::new(*primitive);
                (
                    format!(
                        "boltffi_python_decode_borrowed_raw_wire({pointer_name}, {length_name})"
                    ),
                    Some(primitive),
                    false,
                    false,
                    true,
                )
            }
            _ => (
                format!("boltffi_python_decode_borrowed_raw_wire({pointer_name}, {length_name})"),
                None,
                false,
                false,
                true,
            ),
        };
        Ok(Self {
            declarations: vec![
                TypeSyntax::new(pointer).declaration(&pointer_name)?,
                TypeSyntax::new(length).declaration(&length_name)?,
            ],
            name,
            object,
            expression,
            primitive: None,
            wire_primitive,
            direct_vector: None,
            string,
            bytes,
            raw_wire,
        })
    }

    fn direct_vector_param(
        name: String,
        object: String,
        element: &TypeRef,
        c_types: &[c::Type],
        bridge: &PythonCExtBridgeContract,
        context: &RenderContext<Native>,
    ) -> Result<Self> {
        let [pointer, length] = c_types else {
            return Err(Error::UnsupportedTarget {
                target: "python",
                shape: "callback vector parameter ABI mismatch",
            });
        };
        let pointer_name = format!("{name}_ptr");
        let length_name = format!("{name}_len");
        let element = direct_vector::Element::from_type_ref(element, bridge, context)?;
        Ok(Self {
            declarations: vec![
                TypeSyntax::new(pointer).declaration(&pointer_name)?,
                TypeSyntax::new(length).declaration(&length_name)?,
            ],
            name,
            object,
            expression: format!("{}({pointer_name}, {length_name})", element.vector_boxer()),
            primitive: None,
            wire_primitive: None,
            direct_vector: Some(element),
            string: false,
            bytes: false,
            raw_wire: false,
        })
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct MethodReturn {
    c_type: String,
    parser: String,
    default_value: String,
    value: String,
    primitive: Option<primitive::Runtime>,
    wire_primitive: Option<primitive::Runtime>,
    direct_vector: Option<direct_vector::Element>,
    wire: bool,
    string: bool,
    bytes: bool,
    raw_wire: bool,
    void: bool,
}

impl MethodReturn {
    fn supports(plan: &ReturnPlan<Native, IntoRust>) -> bool {
        matches!(
            plan,
            ReturnPlan::Void
                | ReturnPlan::DirectViaReturnSlot {
                    ty: TypeRef::Primitive(_) | TypeRef::Record(_) | TypeRef::Enum(_),
                }
                | ReturnPlan::EncodedViaReturnSlot {
                    shape: native::BufferShape::Buffer,
                    ..
                }
                | ReturnPlan::DirectVecViaReturnSlot { .. }
        )
    }

    fn new(
        plan: &ReturnPlan<Native, IntoRust>,
        c_type: &c::Type,
        bridge: &PythonCExtBridgeContract,
        context: &RenderContext<Native>,
    ) -> Result<Self> {
        match plan {
            ReturnPlan::Void => Ok(Self {
                c_type: TypeSyntax::new(c_type).anonymous()?,
                parser: String::new(),
                default_value: String::new(),
                value: String::new(),
                primitive: None,
                wire_primitive: None,
                direct_vector: None,
                wire: false,
                string: false,
                bytes: false,
                raw_wire: false,
                void: true,
            }),
            ReturnPlan::DirectViaReturnSlot {
                ty: TypeRef::Primitive(primitive),
            } => {
                let source_primitive = *primitive;
                let primitive = primitive::Runtime::new(source_primitive);
                Ok(Self {
                    c_type: TypeSyntax::new(c_type).anonymous()?,
                    parser: primitive.parser()?.to_owned(),
                    default_value: Self::default_value(source_primitive),
                    value: "return_value".to_owned(),
                    primitive: Some(primitive),
                    wire_primitive: None,
                    direct_vector: None,
                    wire: false,
                    string: false,
                    bytes: false,
                    raw_wire: false,
                    void: false,
                })
            }
            ReturnPlan::DirectViaReturnSlot {
                ty: TypeRef::Record(record_id),
            } => {
                let symbols = record::Symbols::from_record_id(*record_id, bridge, context)?;
                Ok(Self::direct(
                    c_type,
                    symbols.parser().to_owned(),
                    "{0}".to_owned(),
                )?)
            }
            ReturnPlan::DirectViaReturnSlot {
                ty: TypeRef::Enum(enum_id),
            } => {
                let symbols = enumeration::Symbols::from_enum_id(*enum_id, bridge, context)?;
                Ok(Self::direct(
                    c_type,
                    symbols.parser().to_owned(),
                    "0".to_owned(),
                )?)
            }
            ReturnPlan::EncodedViaReturnSlot {
                ty,
                shape: native::BufferShape::Buffer,
                ..
            } => Self::encoded(c_type, ty, bridge, context),
            ReturnPlan::DirectVecViaReturnSlot { element } => {
                let element = direct_vector::Element::from_type_ref(element, bridge, context)?;
                Ok(Self::wire(
                    c_type,
                    element.vector_encoder().to_owned(),
                    Some(element),
                    None,
                    false,
                    false,
                    false,
                )?)
            }
            ReturnPlan::DirectViaReturnSlot { .. }
            | ReturnPlan::EncodedViaReturnSlot { .. }
            | ReturnPlan::HandleViaReturnSlot { .. }
            | ReturnPlan::ScalarOptionViaReturnSlot { .. }
            | ReturnPlan::DirectViaOutPointer { .. }
            | ReturnPlan::EncodedViaOutPointer { .. }
            | ReturnPlan::HandleViaOutPointer { .. }
            | ReturnPlan::ClosureViaOutPointer(_) => Err(Error::UnsupportedTarget {
                target: "python",
                shape: "unsupported callback method return",
            }),
            _ => Err(Error::UnsupportedTarget {
                target: "python",
                shape: "unknown callback method return",
            }),
        }
    }

    fn primitive(&self) -> Option<primitive::Runtime> {
        self.primitive
    }

    fn wire_primitive(&self) -> Option<primitive::Runtime> {
        self.wire_primitive
    }

    fn direct_vector(&self) -> Option<direct_vector::Element> {
        self.direct_vector.clone()
    }

    fn has_string(&self) -> bool {
        self.string
    }

    fn has_bytes(&self) -> bool {
        self.bytes
    }

    fn has_raw_wire(&self) -> bool {
        self.raw_wire
    }

    fn default_value(primitive: Primitive) -> String {
        match primitive {
            Primitive::Bool => "false".to_owned(),
            Primitive::F32 => "0.0f".to_owned(),
            Primitive::F64 => "0.0".to_owned(),
            _ => "0".to_owned(),
        }
    }

    fn has_value(&self) -> bool {
        !self.void
    }

    fn direct(c_type: &c::Type, parser: String, default_value: String) -> Result<Self> {
        Ok(Self {
            c_type: TypeSyntax::new(c_type).anonymous()?,
            parser,
            default_value,
            value: "return_value".to_owned(),
            primitive: None,
            wire_primitive: None,
            direct_vector: None,
            wire: false,
            string: false,
            bytes: false,
            raw_wire: false,
            void: false,
        })
    }

    fn encoded(
        c_type: &c::Type,
        ty: &TypeRef,
        bridge: &PythonCExtBridgeContract,
        context: &RenderContext<Native>,
    ) -> Result<Self> {
        match ty {
            TypeRef::String => Self::wire(
                c_type,
                "boltffi_python_wire_string".to_owned(),
                None,
                None,
                true,
                false,
                false,
            ),
            TypeRef::Bytes => Self::wire(
                c_type,
                "boltffi_python_wire_bytes".to_owned(),
                None,
                None,
                false,
                true,
                false,
            ),
            TypeRef::Primitive(primitive) => Self::wire(
                c_type,
                primitive::Runtime::new(*primitive).wire_encoder()?,
                None,
                Some(primitive::Runtime::new(*primitive)),
                false,
                false,
                false,
            ),
            TypeRef::Record(record_id) => Self::wire(
                c_type,
                record::Symbols::from_record_id(*record_id, bridge, context)?
                    .parser()
                    .to_owned(),
                None,
                None,
                false,
                false,
                false,
            ),
            TypeRef::Enum(enum_id) => Self::wire(
                c_type,
                enumeration::Symbols::from_enum_id(*enum_id, bridge, context)?
                    .parser()
                    .to_owned(),
                None,
                None,
                false,
                false,
                false,
            ),
            _ => Self::wire(
                c_type,
                "boltffi_python_wire_raw".to_owned(),
                None,
                None,
                false,
                false,
                true,
            ),
        }
    }

    fn wire(
        c_type: &c::Type,
        parser: String,
        direct_vector: Option<direct_vector::Element>,
        wire_primitive: Option<primitive::Runtime>,
        string: bool,
        bytes: bool,
        raw_wire: bool,
    ) -> Result<Self> {
        Ok(Self {
            c_type: TypeSyntax::new(c_type).anonymous()?,
            parser,
            default_value: "{0}".to_owned(),
            value: "return_value".to_owned(),
            primitive: None,
            wire_primitive,
            direct_vector,
            wire: true,
            string,
            bytes,
            raw_wire,
            void: false,
        })
    }
}
