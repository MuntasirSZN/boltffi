use askama::Template as AskamaTemplate;
use boltffi_binding::{
    ErrorDecl, ExecutionDecl, ExportedCallable, FunctionDecl, IncomingParam, Native, NativeSymbol,
    OutOfRust, ParamPlan, Receive, ReturnPlan, TypeRef, native,
};

use crate::{
    bridge::{
        c::{self, syntax::TypeSyntax},
        python_cext::{ExtensionMethod, LoadedFunction, MethodFlags, PythonCExtBridgeContract},
    },
    core::{Diagnostic, Emitted, Error, RenderContext, Result},
    target::python::{
        cpython::render::{argument, direct_vector, primitive, result},
        name_style::Name,
    },
};

#[derive(AskamaTemplate)]
#[template(path = "target/python/function.c", escape = "none")]
struct SyncTemplate {
    python_name: String,
    wrapper: String,
    storage: String,
    params: Vec<argument::Conversion>,
    call_args: Vec<String>,
    returns: result::Conversion,
    mutation: Option<argument::MutationOutput>,
    fallible: Option<FallibleResult>,
}

#[derive(AskamaTemplate)]
#[template(path = "target/python/async_function.c", escape = "none")]
struct AsyncTemplate {
    python_name: String,
    start_wrapper: String,
    start_storage: String,
    poll_python_name: String,
    poll_wrapper: String,
    poll_storage: String,
    complete_wrapper: String,
    complete_storage: String,
    panic_message_wrapper: String,
    panic_storage: String,
    cancel_wrapper: String,
    cancel_storage: String,
    free_wrapper: String,
    free_storage: String,
    params: Vec<argument::Conversion>,
    call_args: Vec<String>,
    complete_call_args: Vec<String>,
    returns: result::Conversion,
    fallible: Option<FallibleResult>,
}

pub struct Function {
    body: Body,
}

struct SyncFunction {
    pub python_name: String,
    pub wrapper: String,
    pub storage: String,
    pub params: Vec<argument::Conversion>,
    pub call_args: Vec<String>,
    pub returns: result::Conversion,
    mutation: Option<argument::MutationOutput>,
    fallible: Option<FallibleResult>,
    methods: Vec<ExtensionMethod>,
}

struct AsyncFunction {
    python_name: String,
    start_wrapper: String,
    start_storage: String,
    poll_python_name: String,
    poll_wrapper: String,
    poll_storage: String,
    complete_wrapper: String,
    complete_storage: String,
    panic_message_wrapper: String,
    panic_storage: String,
    cancel_wrapper: String,
    cancel_storage: String,
    free_wrapper: String,
    free_storage: String,
    params: Vec<argument::Conversion>,
    call_args: Vec<String>,
    complete_call_args: Vec<String>,
    returns: result::Conversion,
    fallible: Option<FallibleResult>,
    methods: Vec<ExtensionMethod>,
}

enum Body {
    Sync(Box<SyncFunction>),
    Async(Box<AsyncFunction>),
    Skipped(SkippedFunction),
}

impl Function {
    pub fn from_declaration(
        declaration: &FunctionDecl<Native>,
        bridge: &PythonCExtBridgeContract,
        context: &RenderContext<Native>,
    ) -> Result<Self> {
        Self::from_export(
            Name::new(declaration.name()).function(),
            declaration.symbol(),
            declaration.callable(),
            Vec::new(),
            bridge,
            context,
        )
    }

    pub fn from_export(
        python_name: String,
        symbol: &NativeSymbol,
        callable: &ExportedCallable<Native>,
        receiver_args: Vec<argument::Conversion>,
        bridge: &PythonCExtBridgeContract,
        context: &RenderContext<Native>,
    ) -> Result<Self> {
        if let Some(unsupported) = UnsupportedCallable::from_callable(callable) {
            return Ok(Self {
                body: Body::Skipped(SkippedFunction::new(python_name, unsupported)),
            });
        }
        match callable.execution() {
            ExecutionDecl::Synchronous(_) => SyncFunction::from_export(
                python_name,
                symbol,
                callable,
                receiver_args,
                bridge,
                context,
            )
            .map(Box::new)
            .map(Body::Sync)
            .map(|body| Self { body }),
            ExecutionDecl::Asynchronous(native::AsyncProtocol::PollHandle {
                poll,
                complete,
                cancel,
                free,
                panic_message,
                ..
            }) => AsyncFunction::from_export(
                python_name,
                AsyncSymbols {
                    start: symbol,
                    poll,
                    complete,
                    cancel,
                    free,
                    panic_message,
                },
                callable,
                receiver_args,
                bridge,
                context,
            )
            .map(Box::new)
            .map(Body::Async)
            .map(|body| Self { body }),
            _ => Err(Error::UnsupportedTarget {
                target: "python",
                shape: "unknown function execution",
            }),
        }
    }

    pub fn methods(&self) -> impl Iterator<Item = &ExtensionMethod> {
        self.body.methods().iter()
    }

    pub fn render(self) -> Result<Emitted> {
        match self.body {
            Body::Sync(function) => function.render(),
            Body::Async(function) => function.render(),
            Body::Skipped(function) => Ok(function.render()),
        }
    }

    pub fn primitives(&self) -> Vec<primitive::Runtime> {
        match &self.body {
            Body::Sync(function) => function.primitives(),
            Body::Async(function) => function.primitives(),
            Body::Skipped(_) => Vec::new(),
        }
    }

    pub fn wire_primitives(&self) -> impl Iterator<Item = primitive::Runtime> {
        match &self.body {
            Body::Sync(function) => function.wire_primitives().collect::<Vec<_>>(),
            Body::Async(function) => function.wire_primitives().collect::<Vec<_>>(),
            Body::Skipped(_) => Vec::new(),
        }
        .into_iter()
    }

    pub fn direct_vector_elements(&self) -> impl Iterator<Item = direct_vector::Element> {
        match &self.body {
            Body::Sync(function) => function.direct_vector_elements().collect::<Vec<_>>(),
            Body::Async(function) => function.direct_vector_elements().collect::<Vec<_>>(),
            Body::Skipped(_) => Vec::new(),
        }
        .into_iter()
    }

    pub fn owned_buffer(&self) -> Option<result::OwnedBuffer> {
        self.owned_buffers().next()
    }

    pub fn owned_buffers(&self) -> impl Iterator<Item = result::OwnedBuffer> {
        match &self.body {
            Body::Sync(function) => function.owned_buffers().collect::<Vec<_>>(),
            Body::Async(function) => function.owned_buffers().collect::<Vec<_>>(),
            Body::Skipped(_) => Vec::new(),
        }
        .into_iter()
    }

    pub fn has_string_argument(&self) -> bool {
        match &self.body {
            Body::Sync(function) => function.has_string_argument(),
            Body::Async(function) => function.has_string_argument(),
            Body::Skipped(_) => false,
        }
    }

    pub fn has_bytes_argument(&self) -> bool {
        match &self.body {
            Body::Sync(function) => function.has_bytes_argument(),
            Body::Async(function) => function.has_bytes_argument(),
            Body::Skipped(_) => false,
        }
    }

    pub fn has_raw_wire_argument(&self) -> bool {
        match &self.body {
            Body::Sync(function) => function.has_raw_wire_argument(),
            Body::Async(function) => function.has_raw_wire_argument(),
            Body::Skipped(_) => false,
        }
    }

    pub fn uses_async_protocol(&self) -> bool {
        matches!(self.body, Body::Async(_))
    }

    pub fn can_render(callable: &ExportedCallable<Native>) -> bool {
        UnsupportedCallable::from_callable(callable).is_none()
    }
}

impl Body {
    fn methods(&self) -> &[ExtensionMethod] {
        match self {
            Self::Sync(function) => &function.methods,
            Self::Async(function) => &function.methods,
            Self::Skipped(function) => &function.methods,
        }
    }
}

struct SkippedFunction {
    python_name: String,
    shape: &'static str,
    methods: Vec<ExtensionMethod>,
}

impl SkippedFunction {
    fn new(python_name: String, unsupported: UnsupportedCallable) -> Self {
        Self {
            python_name,
            shape: unsupported.shape(),
            methods: Vec::new(),
        }
    }

    fn render(self) -> Emitted {
        Emitted::diagnostic(Diagnostic::new(format!(
            "python target skipped unsupported callable {}: {}",
            self.python_name, self.shape
        )))
    }
}

#[derive(Clone, Copy)]
struct UnsupportedCallable {
    shape: &'static str,
}

impl UnsupportedCallable {
    fn from_callable(callable: &ExportedCallable<Native>) -> Option<Self> {
        callable
            .params()
            .iter()
            .find_map(|parameter| match parameter.payload() {
                IncomingParam::Value(ParamPlan::Encoded {
                    receive: Receive::ByMutRef,
                    ..
                }) => Some(Self {
                    shape: "mutable encoded parameter",
                }),
                IncomingParam::Value(_) | IncomingParam::Closure(_) => None,
            })
    }

    fn shape(self) -> &'static str {
        self.shape
    }
}

impl SyncFunction {
    fn from_export(
        python_name: String,
        symbol: &NativeSymbol,
        callable: &ExportedCallable<Native>,
        receiver_args: Vec<argument::Conversion>,
        bridge: &PythonCExtBridgeContract,
        context: &RenderContext<Native>,
    ) -> Result<Self> {
        let loaded = bridge
            .loaded_function(symbol)
            .ok_or(Error::UnsupportedTarget {
                target: "python",
                shape: "function without C bridge symbol",
            })?;
        let wrapper = format!("boltffi_python_callable_wrapper_{}", symbol.name().as_str());
        let method =
            ExtensionMethod::new(python_name.clone(), wrapper.clone(), MethodFlags::FastCall)?;
        let value_args = callable
            .params()
            .iter()
            .enumerate()
            .try_fold(
                (
                    receiver_args
                        .iter()
                        .map(argument::Conversion::c_arity)
                        .sum::<usize>(),
                    Vec::new(),
                ),
                |(c_offset, mut conversions), (offset, parameter)| {
                    let index = receiver_args.len() + offset;
                    let conversion = argument::Conversion::from_parameter(
                        symbol.name().as_str(),
                        index,
                        parameter,
                        &loaded.function().params()[c_offset..],
                        bridge,
                        context,
                    )?;
                    let c_offset = c_offset + conversion.c_arity();
                    conversions.push(conversion);
                    Ok::<_, Error>((c_offset, conversions))
                },
            )?
            .1;
        let params = receiver_args
            .into_iter()
            .chain(value_args)
            .collect::<Vec<_>>();
        let base_call_args = params
            .iter()
            .flat_map(argument::Conversion::call_args)
            .collect::<Vec<_>>();
        let fallible =
            FallibleResult::new(callable, loaded, base_call_args.len(), bridge, context)?;
        let call_args = base_call_args
            .into_iter()
            .chain(fallible.iter().filter_map(FallibleResult::success_argument))
            .collect();
        let returns = match &fallible {
            Some(_) => {
                result::Conversion::from_out_plan(callable.returns().plan(), bridge, context)
            }
            None => result::Conversion::from_plan(callable.returns().plan(), bridge, context),
        }?;
        let mutation = Self::mutation(&params, &returns, fallible.is_some())?;
        Ok(Self {
            python_name,
            wrapper,
            storage: loaded.storage_name().to_owned(),
            params,
            call_args,
            returns,
            mutation,
            fallible,
            methods: vec![method],
        })
    }

    fn render(self) -> Result<Emitted> {
        let source = SyncTemplate {
            python_name: self.python_name,
            wrapper: self.wrapper,
            storage: self.storage,
            params: self.params,
            call_args: self.call_args,
            returns: self.returns,
            mutation: self.mutation,
            fallible: self.fallible,
        }
        .render()?;
        Ok(Emitted::primary(source))
    }

    fn mutation(
        params: &[argument::Conversion],
        returns: &result::Conversion,
        fallible: bool,
    ) -> Result<Option<argument::MutationOutput>> {
        let mut mutations = params.iter().filter_map(argument::Conversion::mutation);
        let mutation = mutations.next();
        if mutations.next().is_some() {
            return Err(Error::UnsupportedTarget {
                target: "python",
                shape: "multiple mutable encoded parameters",
            });
        }
        if mutation.is_some() && (fallible || !returns.is_void()) {
            return Err(Error::UnsupportedTarget {
                target: "python",
                shape: "mutable encoded parameter with non-void return",
            });
        }
        Ok(mutation)
    }

    fn primitives(&self) -> Vec<primitive::Runtime> {
        let params = self
            .params
            .iter()
            .filter_map(argument::Conversion::primitive)
            .chain(
                self.params
                    .iter()
                    .flat_map(argument::Conversion::closure_primitives),
            )
            .collect::<Vec<_>>();
        params
            .into_iter()
            .chain(self.returns.primitive())
            .chain(self.fallible.iter().filter_map(FallibleResult::primitive))
            .collect()
    }

    fn wire_primitives(&self) -> impl Iterator<Item = primitive::Runtime> + '_ {
        self.params
            .iter()
            .filter_map(argument::Conversion::wire_primitive)
            .chain(
                self.params
                    .iter()
                    .flat_map(argument::Conversion::closure_wire_primitives),
            )
    }

    fn direct_vector_elements(&self) -> impl Iterator<Item = direct_vector::Element> + '_ {
        self.params
            .iter()
            .filter_map(argument::Conversion::direct_vector_element)
            .chain(
                self.params
                    .iter()
                    .flat_map(argument::Conversion::closure_direct_vector_elements),
            )
            .chain(self.returns.direct_vector_element())
            .chain(
                self.fallible
                    .iter()
                    .filter_map(FallibleResult::direct_vector_element),
            )
    }

    fn owned_buffers(&self) -> impl Iterator<Item = result::OwnedBuffer> + '_ {
        self.returns.owned_buffer().into_iter().chain(
            self.fallible
                .iter()
                .filter_map(FallibleResult::owned_buffer),
        )
    }

    fn has_string_argument(&self) -> bool {
        self.params.iter().any(argument::Conversion::is_string)
            || self
                .params
                .iter()
                .any(|param| param.has_closure_string_argument())
    }

    fn has_bytes_argument(&self) -> bool {
        self.params.iter().any(argument::Conversion::is_bytes)
            || self
                .params
                .iter()
                .any(|param| param.has_closure_bytes_argument())
    }

    fn has_raw_wire_argument(&self) -> bool {
        self.params.iter().any(argument::Conversion::is_raw_wire)
            || self
                .params
                .iter()
                .any(|param| param.has_closure_raw_wire_argument())
    }
}

#[derive(Clone, Copy)]
struct AsyncSymbols<'symbol> {
    start: &'symbol NativeSymbol,
    poll: &'symbol NativeSymbol,
    complete: &'symbol NativeSymbol,
    cancel: &'symbol NativeSymbol,
    free: &'symbol NativeSymbol,
    panic_message: &'symbol NativeSymbol,
}

impl AsyncFunction {
    fn from_export(
        python_name: String,
        symbols: AsyncSymbols<'_>,
        callable: &ExportedCallable<Native>,
        receiver_args: Vec<argument::Conversion>,
        bridge: &PythonCExtBridgeContract,
        context: &RenderContext<Native>,
    ) -> Result<Self> {
        let start = Self::loaded(symbols.start, bridge, "async start symbol")?;
        let poll = Self::loaded(symbols.poll, bridge, "async poll symbol")?;
        let complete = Self::loaded(symbols.complete, bridge, "async complete symbol")?;
        let cancel = Self::loaded(symbols.cancel, bridge, "async cancel symbol")?;
        let free = Self::loaded(symbols.free, bridge, "async free symbol")?;
        let panic_message = Self::loaded(symbols.panic_message, bridge, "async panic symbol")?;
        let start_wrapper = Self::wrapper(symbols.start);
        let poll_python_name = format!("{python_name}__poll");
        let poll_wrapper = Self::wrapper(symbols.poll);
        let complete_wrapper = Self::wrapper(symbols.complete);
        let panic_message_wrapper = Self::wrapper(symbols.panic_message);
        let cancel_wrapper = Self::wrapper(symbols.cancel);
        let free_wrapper = Self::wrapper(symbols.free);
        let value_args = callable
            .params()
            .iter()
            .enumerate()
            .try_fold(
                (
                    receiver_args
                        .iter()
                        .map(argument::Conversion::c_arity)
                        .sum::<usize>(),
                    Vec::new(),
                ),
                |(c_offset, mut conversions), (offset, parameter)| {
                    let index = receiver_args.len() + offset;
                    let conversion = argument::Conversion::from_parameter(
                        symbols.start.name().as_str(),
                        index,
                        parameter,
                        &start.function().params()[c_offset..],
                        bridge,
                        context,
                    )?;
                    let c_offset = c_offset + conversion.c_arity();
                    conversions.push(conversion);
                    Ok::<_, Error>((c_offset, conversions))
                },
            )?
            .1;
        let params = receiver_args
            .into_iter()
            .chain(value_args)
            .collect::<Vec<_>>();
        let call_args = params
            .iter()
            .flat_map(argument::Conversion::call_args)
            .collect::<Vec<_>>();
        let fallible = FallibleResult::new(callable, complete, 2, bridge, context)?;
        let complete_call_args = fallible
            .iter()
            .filter_map(FallibleResult::success_argument)
            .collect::<Vec<_>>();
        let returns = match &fallible {
            Some(_) => {
                result::Conversion::from_out_plan(callable.returns().plan(), bridge, context)
            }
            None => Self::completion_return(callable.returns().plan(), bridge, context),
        }?;
        let methods = [
            ExtensionMethod::new(
                python_name.clone(),
                start_wrapper.clone(),
                MethodFlags::FastCall,
            ),
            ExtensionMethod::new(
                poll_python_name.clone(),
                poll_wrapper.clone(),
                MethodFlags::FastCall,
            ),
            ExtensionMethod::new(
                format!("{python_name}__complete"),
                complete_wrapper.clone(),
                MethodFlags::OneObject,
            ),
            ExtensionMethod::new(
                format!("{python_name}__panic_message"),
                panic_message_wrapper.clone(),
                MethodFlags::OneObject,
            ),
            ExtensionMethod::new(
                format!("{python_name}__cancel"),
                cancel_wrapper.clone(),
                MethodFlags::OneObject,
            ),
            ExtensionMethod::new(
                format!("{python_name}__free"),
                free_wrapper.clone(),
                MethodFlags::OneObject,
            ),
        ]
        .into_iter()
        .collect::<Result<Vec<_>>>()?;
        Ok(Self {
            python_name,
            start_wrapper,
            start_storage: start.storage_name().to_owned(),
            poll_python_name,
            poll_wrapper,
            poll_storage: poll.storage_name().to_owned(),
            complete_wrapper,
            complete_storage: complete.storage_name().to_owned(),
            panic_message_wrapper,
            panic_storage: panic_message.storage_name().to_owned(),
            cancel_wrapper,
            cancel_storage: cancel.storage_name().to_owned(),
            free_wrapper,
            free_storage: free.storage_name().to_owned(),
            params,
            call_args,
            complete_call_args,
            returns,
            fallible,
            methods,
        })
    }

    fn render(self) -> Result<Emitted> {
        let source = AsyncTemplate {
            python_name: self.python_name,
            start_wrapper: self.start_wrapper,
            start_storage: self.start_storage,
            poll_python_name: self.poll_python_name,
            poll_wrapper: self.poll_wrapper,
            poll_storage: self.poll_storage,
            complete_wrapper: self.complete_wrapper,
            complete_storage: self.complete_storage,
            panic_message_wrapper: self.panic_message_wrapper,
            panic_storage: self.panic_storage,
            cancel_wrapper: self.cancel_wrapper,
            cancel_storage: self.cancel_storage,
            free_wrapper: self.free_wrapper,
            free_storage: self.free_storage,
            params: self.params,
            call_args: self.call_args,
            complete_call_args: self.complete_call_args,
            returns: self.returns,
            fallible: self.fallible,
        }
        .render()?;
        Ok(Emitted::primary(source))
    }

    fn primitives(&self) -> Vec<primitive::Runtime> {
        self.params
            .iter()
            .filter_map(argument::Conversion::primitive)
            .chain(
                self.params
                    .iter()
                    .flat_map(argument::Conversion::closure_primitives),
            )
            .chain(self.returns.primitive())
            .chain(self.fallible.iter().filter_map(FallibleResult::primitive))
            .collect()
    }

    fn wire_primitives(&self) -> impl Iterator<Item = primitive::Runtime> + '_ {
        self.params
            .iter()
            .filter_map(argument::Conversion::wire_primitive)
            .chain(
                self.params
                    .iter()
                    .flat_map(argument::Conversion::closure_wire_primitives),
            )
    }

    fn direct_vector_elements(&self) -> impl Iterator<Item = direct_vector::Element> + '_ {
        self.params
            .iter()
            .filter_map(argument::Conversion::direct_vector_element)
            .chain(
                self.params
                    .iter()
                    .flat_map(argument::Conversion::closure_direct_vector_elements),
            )
            .chain(self.returns.direct_vector_element())
            .chain(
                self.fallible
                    .iter()
                    .filter_map(FallibleResult::direct_vector_element),
            )
    }

    fn owned_buffers(&self) -> impl Iterator<Item = result::OwnedBuffer> + '_ {
        self.returns.owned_buffer().into_iter().chain(
            self.fallible
                .iter()
                .filter_map(FallibleResult::owned_buffer),
        )
    }

    fn has_string_argument(&self) -> bool {
        self.params.iter().any(argument::Conversion::is_string)
            || self
                .params
                .iter()
                .any(|param| param.has_closure_string_argument())
    }

    fn has_bytes_argument(&self) -> bool {
        self.params.iter().any(argument::Conversion::is_bytes)
            || self
                .params
                .iter()
                .any(|param| param.has_closure_bytes_argument())
    }

    fn has_raw_wire_argument(&self) -> bool {
        self.params.iter().any(argument::Conversion::is_raw_wire)
            || self
                .params
                .iter()
                .any(|param| param.has_closure_raw_wire_argument())
    }

    fn loaded<'bridge>(
        symbol: &NativeSymbol,
        bridge: &'bridge PythonCExtBridgeContract,
        shape: &'static str,
    ) -> Result<&'bridge LoadedFunction> {
        bridge
            .loaded_function(symbol)
            .ok_or(Error::UnsupportedTarget {
                target: "python",
                shape,
            })
    }

    fn wrapper(symbol: &NativeSymbol) -> String {
        format!("boltffi_python_callable_wrapper_{}", symbol.name().as_str())
    }

    fn completion_return(
        plan: &ReturnPlan<Native, OutOfRust>,
        bridge: &PythonCExtBridgeContract,
        context: &RenderContext<Native>,
    ) -> Result<result::Conversion> {
        match plan {
            ReturnPlan::DirectViaOutPointer { .. }
            | ReturnPlan::EncodedViaOutPointer { .. }
            | ReturnPlan::HandleViaOutPointer { .. } => {
                result::Conversion::from_out_plan(plan, bridge, context)
            }
            ReturnPlan::Void
            | ReturnPlan::DirectViaReturnSlot { .. }
            | ReturnPlan::EncodedViaReturnSlot { .. }
            | ReturnPlan::HandleViaReturnSlot { .. }
            | ReturnPlan::ScalarOptionViaReturnSlot { .. }
            | ReturnPlan::DirectVecViaReturnSlot { .. } => {
                result::Conversion::from_plan(plan, bridge, context)
            }
            _ => Err(Error::UnsupportedTarget {
                target: "python",
                shape: "async completion return",
            }),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct FallibleResult {
    success_declaration: Option<String>,
    success_argument: Option<String>,
    success_value: String,
    error_type: String,
    error_value: String,
    error: result::Conversion,
}

impl FallibleResult {
    fn new(
        callable: &ExportedCallable<Native>,
        loaded: &LoadedFunction,
        argument_count: usize,
        bridge: &PythonCExtBridgeContract,
        context: &RenderContext<Native>,
    ) -> Result<Option<Self>> {
        match callable.error() {
            ErrorDecl::None(_) => Ok(None),
            ErrorDecl::EncodedViaReturnSlot {
                ty,
                shape: native::BufferShape::Buffer,
                ..
            } => Self::encoded(
                ty,
                callable.returns().plan(),
                loaded,
                argument_count,
                bridge,
                context,
            )
            .map(Some),
            ErrorDecl::EncodedViaReturnSlot { .. } => Err(Error::UnsupportedTarget {
                target: "python",
                shape: "fallible error buffer shape",
            }),
            _ => Err(Error::UnsupportedTarget {
                target: "python",
                shape: "fallible function",
            }),
        }
    }

    fn success_argument(&self) -> Option<String> {
        self.success_argument.clone()
    }

    fn primitive(&self) -> Option<primitive::Runtime> {
        self.error.primitive()
    }

    fn direct_vector_element(&self) -> Option<direct_vector::Element> {
        self.error.direct_vector_element()
    }

    fn owned_buffer(&self) -> Option<result::OwnedBuffer> {
        self.error.owned_buffer()
    }

    fn encoded(
        error: &TypeRef,
        success: &ReturnPlan<Native, OutOfRust>,
        loaded: &LoadedFunction,
        argument_count: usize,
        bridge: &PythonCExtBridgeContract,
        context: &RenderContext<Native>,
    ) -> Result<Self> {
        if !matches!(loaded.function().returns(), c::Type::Buffer) {
            return Err(Error::UnsupportedTarget {
                target: "python",
                shape: "fallible error return",
            });
        }
        let error = result::Conversion::from_encoded_type(error, bridge, context)?;
        let (success_declaration, success_argument, success_value) =
            Self::success_binding(success, loaded.function(), argument_count)?;
        Ok(Self {
            success_declaration,
            success_argument,
            success_value,
            error_type: TypeSyntax::new(loaded.function().returns()).anonymous()?,
            error_value: "return_error".to_owned(),
            error,
        })
    }

    fn success_binding(
        success: &ReturnPlan<Native, OutOfRust>,
        function: &c::Function,
        argument_count: usize,
    ) -> Result<(Option<String>, Option<String>, String)> {
        match success {
            ReturnPlan::Void => Ok((None, None, String::new())),
            ReturnPlan::DirectViaOutPointer { .. }
            | ReturnPlan::EncodedViaOutPointer { .. }
            | ReturnPlan::HandleViaOutPointer { .. } => {
                let parameter =
                    function
                        .params()
                        .get(argument_count)
                        .ok_or(Error::UnsupportedTarget {
                            target: "python",
                            shape: "missing fallible success out parameter",
                        })?;
                let c::Type::MutPointer(success_type) = parameter.ty() else {
                    return Err(Error::UnsupportedTarget {
                        target: "python",
                        shape: "fallible success parameter",
                    });
                };
                let value = "return_success".to_owned();
                Ok((
                    Some(TypeSyntax::new(success_type.as_ref()).declaration(&value)?),
                    Some(format!("&{value}")),
                    value,
                ))
            }
            _ => Err(Error::UnsupportedTarget {
                target: "python",
                shape: "fallible success return",
            }),
        }
    }
}
