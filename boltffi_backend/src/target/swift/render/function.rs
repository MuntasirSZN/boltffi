use askama::Template;

use boltffi_binding::{
    CanonicalName, ClassId, ClosureReturn, DirectValueType, DirectVectorElementType, Direction,
    ErrorChannel, ErrorPlacement, ExecutionDecl, ExportedCallable, ExportedMethodDecl,
    FunctionDecl, HandlePresence, HandleTarget, InitializerDecl, IntoRust, Native, NativeSymbol,
    OutOfRust, ParamDecl, ParamPlanRender, Primitive, Receive, ReturnPlanRender, ReturnValueSlot,
    Surface, TypeRef, WritePlan, native,
};

use crate::{
    bridge::c::{CBridgeContract, Function as CFunction, ReturnChannel},
    core::{AuxChunk, Emitted, Error, HelperId, RenderContext, Result, TextChunk},
    target::swift::{
        SwiftHost,
        codec::{ArgumentBuffer, OwnedBuffer, Reader, ScalarOption, Writer},
        name_style::{GeneratedLocal, Name},
        primitive::SwiftPrimitive,
        render::{Documentation, SwiftType},
        syntax::{ArgumentList, Expression, Identifier, Statement, TypeName},
    },
};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Function {
    documentation: Documentation,
    name: Identifier,
    parameters: Vec<Parameter>,
    body: String,
    returns: ReturnSignature,
    requires_wire_runtime: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AssociatedFunction {
    documentation: Documentation,
    is_static: bool,
    name: Identifier,
    parameters: Vec<Parameter>,
    body: String,
    returns: ReturnSignature,
    requires_wire_runtime: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Initializer {
    documentation: Documentation,
    parameters: Vec<Parameter>,
    body: String,
    fallible: bool,
    requires_wire_runtime: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Receiver {
    argument: Argument,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Parameter {
    name: Identifier,
    ty: TypeName,
    argument: Argument,
}

#[derive(Clone, Debug, Eq, PartialEq)]
enum Argument {
    Direct(Expression),
    Encoded(EncodedArgument),
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct EncodedArgument {
    buffer: ArgumentBuffer,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum BodyExit {
    ReturnValue,
    CompleteEffect,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct Return {
    ty: Option<TypeName>,
    conversion: ReturnConversion,
    success: Option<SuccessSlot>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
enum ReturnConversion {
    Direct,
    FromC(TypeName),
    Encoded(EncodedReturn),
    ClassHandle(ClassHandle),
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct EncodedReturn {
    buffer: OwnedBuffer,
    reader: Identifier,
    decode: Expression,
    free: Identifier,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct ClassHandle {
    ty: TypeName,
    presence: HandlePresence,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct SuccessSlot {
    binding: Identifier,
    ty: TypeName,
}

#[derive(Clone, Debug, Eq, PartialEq)]
enum ErrorConversion {
    None,
    Encoded(EncodedError),
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct EncodedError {
    buffer: OwnedBuffer,
    reader: Identifier,
    decode: Expression,
    free: Identifier,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct Invocation {
    symbol: String,
    parameters: Vec<Parameter>,
    arguments: Vec<Argument>,
    returns: Return,
    error: ErrorConversion,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ReturnSignature {
    ty: Option<TypeName>,
    fallible: bool,
}

#[derive(Template)]
#[template(path = "target/swift/function.swift", escape = "none")]
struct FunctionTemplate<'a> {
    function: &'a Function,
}

#[derive(Template)]
#[template(path = "target/swift/wire.swift", escape = "none")]
struct WireTemplate;

impl Function {
    pub fn from_declaration(
        decl: &FunctionDecl<Native>,
        bridge: &CBridgeContract,
        context: &RenderContext<Native>,
    ) -> Result<Self> {
        let invocation =
            Invocation::from_callable(decl.symbol(), decl.callable(), None, bridge, context)?;
        let requires_wire_runtime = invocation.requires_wire_runtime();
        let (parameters, body, returns) = invocation.into_rendered("    ")?;
        Ok(Self {
            documentation: Documentation::new(decl.meta().doc(), ""),
            name: Name::new(decl.name()).function()?,
            parameters,
            body,
            returns,
            requires_wire_runtime,
        })
    }

    pub fn render(&self) -> Result<Emitted> {
        let mut source = FunctionTemplate { function: self }.render()?;
        source.push_str("\n\n");
        let emitted = Emitted::primary(source);
        match self.requires_wire_runtime {
            true => Ok(emitted.with_aux(Self::wire_helper()?)),
            false => Ok(emitted),
        }
    }

    fn name(&self) -> &Identifier {
        &self.name
    }

    fn documentation(&self) -> &Documentation {
        &self.documentation
    }

    fn parameters(&self) -> &[Parameter] {
        &self.parameters
    }

    fn body(&self) -> &str {
        &self.body
    }

    fn returns(&self) -> &ReturnSignature {
        &self.returns
    }

    fn wire_helper() -> Result<AuxChunk> {
        let mut text = WireTemplate.render()?;
        text.push_str("\n\n");
        Ok(AuxChunk::Helper {
            id: HelperId::new(CanonicalName::single("swift_wire")),
            text: TextChunk::new(text),
        })
    }
}

impl AssociatedFunction {
    pub fn from_initializer(
        initializer: &boltffi_binding::InitializerDecl<Native>,
        bridge: &CBridgeContract,
        context: &RenderContext<Native>,
    ) -> Result<Self> {
        Self::from_parts(
            Documentation::new(initializer.meta().doc(), "    "),
            true,
            Name::new(initializer.name()).function()?,
            Invocation::from_callable(
                initializer.symbol(),
                initializer.callable(),
                None,
                bridge,
                context,
            )?,
        )
    }

    pub fn from_method(
        method: &ExportedMethodDecl<Native, NativeSymbol>,
        receiver: Option<Receiver>,
        bridge: &CBridgeContract,
        context: &RenderContext<Native>,
    ) -> Result<Self> {
        let is_static = receiver.is_none();
        Self::from_parts(
            Documentation::new(method.meta().doc(), "    "),
            is_static,
            Name::new(method.name()).function()?,
            Invocation::from_callable(
                method.target(),
                method.callable(),
                receiver,
                bridge,
                context,
            )?,
        )
    }

    pub fn documentation(&self) -> &Documentation {
        &self.documentation
    }

    pub fn static_keyword(&self) -> &str {
        match self.is_static {
            true => "static ",
            false => "",
        }
    }

    pub fn name(&self) -> &Identifier {
        &self.name
    }

    pub fn parameters(&self) -> &[Parameter] {
        &self.parameters
    }

    pub fn body(&self) -> &str {
        &self.body
    }

    pub fn returns(&self) -> &ReturnSignature {
        &self.returns
    }

    pub fn requires_wire_runtime(&self) -> bool {
        self.requires_wire_runtime
    }

    pub fn wire_helper() -> Result<AuxChunk> {
        Function::wire_helper()
    }

    fn from_parts(
        documentation: Documentation,
        is_static: bool,
        name: Identifier,
        invocation: Invocation,
    ) -> Result<Self> {
        let requires_wire_runtime = invocation.requires_wire_runtime();
        let (parameters, body, returns) = invocation.into_rendered("        ")?;
        Ok(Self {
            documentation,
            is_static,
            name,
            parameters,
            body,
            returns,
            requires_wire_runtime,
        })
    }
}

impl Initializer {
    pub fn from_declaration(
        initializer: &InitializerDecl<Native>,
        bridge: &CBridgeContract,
        context: &RenderContext<Native>,
    ) -> Result<Self> {
        let invocation = Invocation::from_callable(
            initializer.symbol(),
            initializer.callable(),
            None,
            bridge,
            context,
        )?;
        let fallible = invocation.error.fallible();
        let requires_wire_runtime = invocation.requires_wire_runtime();
        let (parameters, body) = invocation.into_initializer_rendered("        ")?;
        Ok(Self {
            documentation: Documentation::new(initializer.meta().doc(), "    "),
            parameters,
            body,
            fallible,
            requires_wire_runtime,
        })
    }

    pub fn documentation(&self) -> &Documentation {
        &self.documentation
    }

    pub fn parameters(&self) -> &[Parameter] {
        &self.parameters
    }

    pub fn throwing_keyword(&self) -> &str {
        match self.fallible {
            true => " throws",
            false => "",
        }
    }

    pub fn body(&self) -> &str {
        &self.body
    }

    pub fn requires_wire_runtime(&self) -> bool {
        self.requires_wire_runtime
    }
}

impl Receiver {
    pub fn direct() -> Self {
        Self {
            argument: Argument::Direct(Expression::member("self", "cValue")),
        }
    }

    pub fn encoded(
        name: &CanonicalName,
        plan: &WritePlan,
        context: &RenderContext<Native>,
    ) -> Result<Self> {
        Ok(Self {
            argument: Argument::Encoded(EncodedArgument::new(
                &Name::new(name),
                plan,
                Expression::new("self"),
                context,
            )?),
        })
    }

    pub fn class_handle() -> Self {
        Self {
            argument: Argument::Direct(Expression::member("self", "handle")),
        }
    }

    fn argument(self) -> Argument {
        self.argument
    }
}

impl Invocation {
    fn from_callable(
        symbol: &NativeSymbol,
        callable: &ExportedCallable<Native>,
        receiver: Option<Receiver>,
        bridge: &CBridgeContract,
        context: &RenderContext<Native>,
    ) -> Result<Self> {
        Self::check_execution(callable)?;
        Self::check_receiver(callable, receiver.as_ref())?;
        let c_function = Self::c_function(symbol, bridge)?;
        let error =
            ErrorConversion::from_channel(callable.error().channel(), c_function, bridge, context)?;
        let parameters = callable
            .params()
            .iter()
            .map(|parameter| Parameter::from_decl(parameter, context))
            .collect::<Result<Vec<_>>>()?;
        let arguments = receiver
            .into_iter()
            .map(Receiver::argument)
            .chain(parameters.iter().map(Parameter::argument))
            .collect::<Vec<_>>();
        let returns = callable.returns().plan().render_with(&mut ReturnPlan {
            bridge,
            context,
            c_return_channel: c_function.return_channel(),
        })?;
        Ok(Self {
            symbol: c_function.name().to_owned(),
            parameters,
            arguments,
            returns,
            error,
        })
    }

    fn into_rendered(self, indent: &str) -> Result<(Vec<Parameter>, String, ReturnSignature)> {
        let body = self.render_body(indent)?;
        let returns = self.returns.signature(self.error.fallible());
        Ok((self.parameters, body, returns))
    }

    fn into_initializer_rendered(self, indent: &str) -> Result<(Vec<Parameter>, String)> {
        let body = self.render_initializer_body(indent)?;
        Ok((self.parameters, body))
    }

    fn requires_wire_runtime(&self) -> bool {
        self.arguments.iter().any(Argument::requires_wire_runtime)
            || self.returns.requires_wire_runtime()
            || self.error.requires_wire_runtime()
    }

    fn render_body(&self, indent: &str) -> Result<String> {
        let encoded_arguments = self
            .arguments
            .iter()
            .filter_map(Argument::encoded_argument)
            .collect::<Vec<_>>();
        Self::render_scoped_body(
            &encoded_arguments,
            &self.returns,
            &self.error,
            self.call(),
            indent,
            self.returns.exit(),
        )
    }

    fn render_initializer_body(&self, indent: &str) -> Result<String> {
        let encoded_arguments = self
            .arguments
            .iter()
            .filter_map(Argument::encoded_argument)
            .collect::<Vec<_>>();
        Self::render_scoped_initializer_body(
            &encoded_arguments,
            &self.returns,
            &self.error,
            self.call(),
            indent,
        )
    }

    fn render_scoped_body(
        encoded_arguments: &[&EncodedArgument],
        returns: &Return,
        error: &ErrorConversion,
        call: Expression,
        indent: &str,
        exit: BodyExit,
    ) -> Result<String> {
        match encoded_arguments.split_first() {
            Some((argument, rest)) => Ok(argument.wrap(
                Self::render_scoped_body(
                    rest,
                    returns,
                    error,
                    call,
                    &format!("{indent}    "),
                    exit,
                )?,
                indent,
                exit,
            )),
            None => returns.body(call, error, indent),
        }
    }

    fn render_scoped_initializer_body(
        encoded_arguments: &[&EncodedArgument],
        returns: &Return,
        error: &ErrorConversion,
        call: Expression,
        indent: &str,
    ) -> Result<String> {
        match encoded_arguments.split_first() {
            Some((argument, rest)) => Ok(argument.wrap(
                Self::render_scoped_initializer_body(
                    rest,
                    returns,
                    error,
                    call,
                    &format!("{indent}    "),
                )?,
                indent,
                BodyExit::CompleteEffect,
            )),
            None => returns.initializer_body(call, error, indent),
        }
    }

    fn call(&self) -> Expression {
        Expression::call(
            &self.symbol,
            self.arguments
                .iter()
                .flat_map(Argument::arguments)
                .chain(self.returns.arguments())
                .collect::<ArgumentList>(),
        )
    }

    fn c_function<'bridge>(
        symbol: &NativeSymbol,
        bridge: &'bridge CBridgeContract,
    ) -> Result<&'bridge CFunction> {
        bridge
            .functions()
            .iter()
            .find(|function| function.name() == symbol.name().as_str())
            .ok_or(Error::BrokenBridgeContract {
                bridge: SwiftHost::TARGET,
                invariant: "missing C function for Swift function",
            })
    }

    fn check_execution(callable: &ExportedCallable<Native>) -> Result<()> {
        match callable.execution() {
            ExecutionDecl::Synchronous(_) => Ok(()),
            ExecutionDecl::Asynchronous(_) => Err(SwiftHost::unsupported("async function")),
            _ => Err(SwiftHost::unsupported("unknown function execution")),
        }
    }

    fn check_receiver(
        callable: &ExportedCallable<Native>,
        receiver: Option<&Receiver>,
    ) -> Result<()> {
        match (callable.receiver(), receiver) {
            (None, None) => Ok(()),
            (Some(Receive::ByValue | Receive::ByRef), Some(_)) => Ok(()),
            (Some(Receive::ByMutRef), Some(_)) => {
                Err(SwiftHost::unsupported("mutable value receiver"))
            }
            _ => Err(SwiftHost::unsupported("method receiver mismatch")),
        }
    }
}

impl Parameter {
    fn from_decl(
        decl: &ParamDecl<Native, IntoRust>,
        context: &RenderContext<Native>,
    ) -> Result<Self> {
        let source_name = Name::new(decl.name());
        let name = source_name.parameter()?;
        decl.payload()
            .as_value()
            .ok_or(SwiftHost::unsupported("closure parameter"))?
            .render_with(&mut ParameterPlan {
                source_name,
                name,
                context,
            })
    }

    pub fn signature(&self) -> String {
        format!("{}: {}", self.name, self.ty)
    }

    fn argument(&self) -> Argument {
        self.argument.clone()
    }
}

impl Argument {
    fn arguments(&self) -> Vec<Expression> {
        match self {
            Self::Direct(argument) => vec![argument.clone()],
            Self::Encoded(argument) => argument.arguments(),
        }
    }

    fn encoded_argument(&self) -> Option<&EncodedArgument> {
        match self {
            Self::Encoded(argument) => Some(argument),
            Self::Direct(_) => None,
        }
    }

    fn requires_wire_runtime(&self) -> bool {
        matches!(self, Self::Encoded(_))
    }
}

impl EncodedArgument {
    fn new(
        source_name: &Name,
        plan: &WritePlan,
        current: Expression,
        context: &RenderContext<Native>,
    ) -> Result<Self> {
        let buffer = ArgumentBuffer::new(source_name)?;
        let write = plan
            .render_with(&mut Writer::new(buffer.writer().clone(), current, context))
            .into_iter()
            .collect::<Result<Vec<_>>>()?;
        Ok(Self {
            buffer: buffer.with_statements(write),
        })
    }

    fn scalar_option(source_name: &Name, primitive: Primitive, value: Expression) -> Result<Self> {
        ScalarOption::new(primitive)
            .write(source_name, value)
            .map(|buffer| Self { buffer })
    }

    fn arguments(&self) -> Vec<Expression> {
        self.buffer.arguments()
    }

    fn wrap(&self, body: String, indent: &str, exit: BodyExit) -> String {
        format!(
            "{}\n{}",
            self.buffer.bytes_statement().indented(indent),
            self.buffer
                .with_buffer_scope(body, indent, exit.returns_value())
        )
    }
}

impl BodyExit {
    fn returns_value(self) -> bool {
        matches!(self, Self::ReturnValue)
    }
}

struct ParameterPlan<'context, 'bindings> {
    source_name: Name,
    name: Identifier,
    context: &'context RenderContext<'bindings, Native>,
}

impl<'plan, 'context, 'bindings> ParamPlanRender<'plan, Native, IntoRust>
    for ParameterPlan<'context, 'bindings>
{
    type Output = Result<Parameter>;

    fn direct(&mut self, ty: &'plan DirectValueType, receive: Receive) -> Self::Output {
        if receive != Receive::ByValue {
            return Err(SwiftHost::unsupported("borrowed direct parameter"));
        }
        let ty = match ty {
            DirectValueType::Primitive(primitive) => SwiftType::primitive(*primitive)?,
            DirectValueType::Record(record) => {
                return Ok(Parameter {
                    name: self.name.clone(),
                    ty: SwiftType::record(*record, self.context)?,
                    argument: Argument::Direct(Expression::member(&self.name, "cValue")),
                });
            }
            DirectValueType::Enum(enumeration) => {
                return Ok(Parameter {
                    name: self.name.clone(),
                    ty: SwiftType::enumeration(*enumeration, self.context)?,
                    argument: Argument::Direct(Expression::member(&self.name, "cValue")),
                });
            }
            _ => return Err(SwiftHost::unsupported("unknown direct parameter")),
        };
        Ok(Parameter {
            name: self.name.clone(),
            ty,
            argument: Argument::Direct(Expression::new(self.name.to_string())),
        })
    }

    fn encoded(
        &mut self,
        ty: &'plan TypeRef,
        codec: &'plan <IntoRust as Direction>::Codec,
        shape: <Native as Surface>::BufferShape,
        receive: Receive,
    ) -> Self::Output {
        if shape != native::BufferShape::Slice {
            return Err(SwiftHost::unsupported("encoded parameter shape"));
        }
        if receive == Receive::ByMutRef {
            return Err(SwiftHost::unsupported("mutable encoded parameter"));
        }
        Ok(Parameter {
            name: self.name.clone(),
            ty: SwiftType::type_ref(ty, self.context)?,
            argument: Argument::Encoded(EncodedArgument::new(
                &self.source_name,
                codec,
                Expression::identifier(self.name.clone()),
                self.context,
            )?),
        })
    }

    fn handle(
        &mut self,
        target: &'plan HandleTarget,
        _carrier: <Native as Surface>::HandleCarrier,
        presence: HandlePresence,
        _receive: Receive,
    ) -> Self::Output {
        match target {
            HandleTarget::Class(class) => {
                let handle = ClassHandle::new(*class, presence, self.context)?;
                Ok(Parameter {
                    name: self.name.clone(),
                    ty: handle.api_type(),
                    argument: Argument::Direct(
                        handle.parameter_argument(Expression::identifier(self.name.clone())),
                    ),
                })
            }
            HandleTarget::Callback(_) => Err(SwiftHost::unsupported("callback handle parameter")),
            HandleTarget::Stream(_) => Err(SwiftHost::unsupported("stream handle parameter")),
            _ => Err(SwiftHost::unsupported("unknown handle parameter")),
        }
    }

    fn scalar_option(&mut self, primitive: Primitive) -> Self::Output {
        Ok(Parameter {
            name: self.name.clone(),
            ty: ScalarOption::new(primitive).ty()?,
            argument: Argument::Encoded(EncodedArgument::scalar_option(
                &self.source_name,
                primitive,
                Expression::identifier(self.name.clone()),
            )?),
        })
    }

    fn direct_vector(&mut self, _element: &'plan DirectVectorElementType) -> Self::Output {
        Err(SwiftHost::unsupported("direct vector parameter"))
    }
}

impl Return {
    fn signature(&self, fallible: bool) -> ReturnSignature {
        ReturnSignature {
            ty: self.ty.clone(),
            fallible,
        }
    }

    fn exit(&self) -> BodyExit {
        match self.ty {
            Some(_) => BodyExit::ReturnValue,
            None => BodyExit::CompleteEffect,
        }
    }

    fn requires_wire_runtime(&self) -> bool {
        matches!(self.conversion, ReturnConversion::Encoded(_))
    }

    fn arguments(&self) -> impl Iterator<Item = Expression> + '_ {
        self.success.iter().map(SuccessSlot::argument)
    }

    fn body(&self, call: Expression, error: &ErrorConversion, indent: &str) -> Result<String> {
        let setup = self
            .success
            .as_ref()
            .map(|success| success.statement().indented(indent));
        let error = error.body(call.clone(), indent)?;
        let success = self.success.as_ref().map(SuccessSlot::expression);
        let result = match (success, error.consumes_call()) {
            (Some(value), _) => Some(self.body_for_success(value, indent)?),
            (None, false) => Some(self.body_for_value(call, indent)?),
            (None, true) => None,
        };
        Ok([setup, Some(error.text), result]
            .into_iter()
            .flatten()
            .filter(|line| !line.is_empty())
            .collect::<Vec<_>>()
            .join("\n"))
    }

    fn body_for_value(&self, value: Expression, indent: &str) -> Result<String> {
        match &self.conversion {
            ReturnConversion::Encoded(encoded) => encoded.body(value, indent),
            ReturnConversion::ClassHandle(handle) => handle.body(value, indent),
            ReturnConversion::Direct | ReturnConversion::FromC(_) => {
                Ok(self.statement(value).indented(indent))
            }
        }
    }

    fn body_for_success(&self, value: Expression, indent: &str) -> Result<String> {
        match &self.conversion {
            ReturnConversion::Encoded(encoded)
                if value == Expression::identifier(encoded.buffer.binding().clone()) =>
            {
                encoded.body_from_buffer(indent)
            }
            _ => self.body_for_value(value, indent),
        }
    }

    fn initializer_body(
        &self,
        call: Expression,
        error: &ErrorConversion,
        indent: &str,
    ) -> Result<String> {
        let ReturnConversion::ClassHandle(_) = &self.conversion else {
            return Err(SwiftHost::unsupported("class initializer return"));
        };
        let setup = self
            .success
            .as_ref()
            .map(|success| success.statement().indented(indent));
        let error = error.body(call.clone(), indent)?;
        let value = match (
            self.success.as_ref().map(SuccessSlot::expression),
            error.consumes_call(),
        ) {
            (Some(value), _) => value,
            (None, false) => call,
            (None, true) => return Err(SwiftHost::unsupported("class initializer result")),
        };
        let assign =
            Statement::assign(Expression::member("self", "handle"), value).indented(indent);
        Ok([setup, Some(error.text), Some(assign)]
            .into_iter()
            .flatten()
            .filter(|line| !line.is_empty())
            .collect::<Vec<_>>()
            .join("\n"))
    }

    fn statement(&self, call: Expression) -> Statement {
        match &self.ty {
            Some(_) => Statement::returns(self.expression(call)),
            None => Statement::expression(call),
        }
    }

    fn expression(&self, call: Expression) -> Expression {
        match &self.conversion {
            ReturnConversion::Direct => call,
            ReturnConversion::FromC(ty) => Expression::call(
                ty,
                [Expression::labeled("fromC", call)]
                    .into_iter()
                    .collect::<ArgumentList>(),
            ),
            ReturnConversion::Encoded(_) => call,
            ReturnConversion::ClassHandle(handle) => handle.wrap(call),
        }
    }
}

impl ClassHandle {
    fn new(id: ClassId, presence: HandlePresence, context: &RenderContext<Native>) -> Result<Self> {
        Ok(Self {
            ty: SwiftType::class(id, context)?,
            presence,
        })
    }

    fn api_type(&self) -> TypeName {
        match self.presence {
            HandlePresence::Required => self.ty.clone(),
            HandlePresence::Nullable => self.ty.clone().optional(),
            _ => self.ty.clone(),
        }
    }

    fn parameter_argument(&self, value: Expression) -> Expression {
        match self.presence {
            HandlePresence::Required => Expression::member(value, "handle"),
            HandlePresence::Nullable => Expression::nil_coalescing(
                Expression::member(Expression::new(format!("{value}?")), "handle"),
                Self::empty(),
            ),
            _ => value,
        }
    }

    fn wrap(&self, handle: Expression) -> Expression {
        let wrapped = Expression::call(
            &self.ty,
            [Expression::labeled("handle", handle.clone())]
                .into_iter()
                .collect::<ArgumentList>(),
        );
        match self.presence {
            HandlePresence::Required => wrapped,
            HandlePresence::Nullable => Expression::conditional(
                Expression::equal(&handle, Self::empty()),
                Expression::nil(),
                wrapped,
            ),
            _ => wrapped,
        }
    }

    fn body(&self, handle: Expression, indent: &str) -> Result<String> {
        match self.presence {
            HandlePresence::Required => Ok(Statement::returns(self.wrap(handle)).indented(indent)),
            HandlePresence::Nullable => {
                let binding = GeneratedLocal::ReturnHandle.identifier()?;
                let value = Expression::identifier(binding.clone());
                Ok([
                    Statement::let_value(&binding, handle).indented(indent),
                    Statement::returns(self.wrap(value)).indented(indent),
                ]
                .join("\n"))
            }
            _ => Ok(Statement::returns(self.wrap(handle)).indented(indent)),
        }
    }

    fn empty() -> Expression {
        Expression::new("0")
    }
}

impl EncodedReturn {
    fn new(decode: Expression, bridge: &CBridgeContract) -> Result<Self> {
        Ok(Self {
            buffer: OwnedBuffer::new(GeneratedLocal::ReturnBuffer.identifier()?),
            reader: GeneratedLocal::WireReader.identifier()?,
            decode,
            free: Identifier::parse(bridge.support().buffer_free()?.name())?,
        })
    }

    fn body(&self, call: Expression, indent: &str) -> Result<String> {
        Ok([
            Statement::let_value(self.buffer.binding(), call).indented(indent),
            self.body_from_buffer(indent)?,
        ]
        .join("\n"))
    }

    fn body_from_buffer(&self, indent: &str) -> Result<String> {
        let decode_call = self.buffer.decode(&self.reader, &self.decode)?;
        Ok([
            Statement::defer(Expression::call(
                &self.free,
                [Expression::identifier(self.buffer.binding().clone())]
                    .into_iter()
                    .collect::<ArgumentList>(),
            ))
            .indented(indent),
            Statement::returns(decode_call).indented(indent),
        ]
        .join("\n"))
    }
}

impl SuccessSlot {
    fn new(ty: TypeName) -> Result<Self> {
        Ok(Self {
            binding: GeneratedLocal::ReturnBuffer.identifier()?,
            ty,
        })
    }

    fn statement(&self) -> Statement {
        Statement::var_value(
            &self.binding,
            &self.ty,
            Expression::call(&self.ty, ArgumentList::default()),
        )
    }

    fn argument(&self) -> Expression {
        Expression::address(&self.binding)
    }

    fn expression(&self) -> Expression {
        Expression::identifier(self.binding.clone())
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct ErrorBody {
    text: String,
    consumes_call: bool,
}

impl ErrorBody {
    fn empty() -> Self {
        Self {
            text: String::new(),
            consumes_call: false,
        }
    }

    fn consumes_call(&self) -> bool {
        self.consumes_call
    }
}

impl ReturnSignature {
    pub fn signature(&self) -> String {
        match (&self.ty, self.fallible) {
            (Some(ty), true) => format!(" throws -> {ty}"),
            (Some(ty), false) => format!(" -> {ty}"),
            (None, true) => " throws".to_owned(),
            (None, false) => String::new(),
        }
    }
}

impl ErrorConversion {
    fn from_channel(
        channel: ErrorChannel<'_, Native, OutOfRust>,
        function: &CFunction,
        bridge: &CBridgeContract,
        context: &RenderContext<Native>,
    ) -> Result<Self> {
        match channel {
            ErrorChannel::None => {
                if function.return_channel() == ReturnChannel::EncodedError {
                    Err(Error::BrokenBridgeContract {
                        bridge: SwiftHost::TARGET,
                        invariant: "C return channel carries an error for an infallible callable",
                    })
                } else {
                    Ok(Self::None)
                }
            }
            ErrorChannel::Status => Err(SwiftHost::unsupported("status error channel")),
            ErrorChannel::Encoded {
                placement,
                ty,
                codec,
                shape,
            } => {
                if placement != ErrorPlacement::ReturnSlot {
                    return Err(SwiftHost::unsupported("error out pointer"));
                }
                if function.return_channel() != ReturnChannel::EncodedError {
                    return Err(Error::BrokenBridgeContract {
                        bridge: SwiftHost::TARGET,
                        invariant: "encoded error does not use the C return slot",
                    });
                }
                if shape != native::BufferShape::Buffer {
                    return Err(SwiftHost::unsupported("encoded error shape"));
                }
                let reader = GeneratedLocal::ErrorReader.identifier()?;
                let decode = codec.render_with(&mut Reader::new(reader.clone(), context))?;
                Ok(Self::Encoded(EncodedError::new(
                    ty, decode, reader, bridge,
                )?))
            }
            _ => Err(SwiftHost::unsupported("unknown error channel")),
        }
    }

    fn fallible(&self) -> bool {
        !matches!(self, Self::None)
    }

    fn requires_wire_runtime(&self) -> bool {
        matches!(self, Self::Encoded(_))
    }

    fn body(&self, call: Expression, indent: &str) -> Result<ErrorBody> {
        match self {
            Self::None => Ok(ErrorBody::empty()),
            Self::Encoded(encoded) => encoded.body(call, indent),
        }
    }
}

impl EncodedError {
    fn new(
        ty: &TypeRef,
        decode: Expression,
        reader: Identifier,
        bridge: &CBridgeContract,
    ) -> Result<Self> {
        Ok(Self {
            buffer: OwnedBuffer::new(GeneratedLocal::ErrorBuffer.identifier()?),
            reader,
            decode: Self::throw_expression(ty, decode)?,
            free: Identifier::parse(bridge.support().buffer_free()?.name())?,
        })
    }

    fn body(&self, call: Expression, indent: &str) -> Result<ErrorBody> {
        let decode = self.buffer.decode(&self.reader, &self.decode)?;
        let block = Statement::if_then(
            self.buffer.is_present(),
            [
                Statement::defer(self.buffer.free_call(&self.free)),
                Statement::throwing(decode),
            ],
        );
        Ok(ErrorBody {
            text: [
                Statement::let_value(self.buffer.binding(), call).indented(indent),
                block.indented(indent),
            ]
            .join("\n"),
            consumes_call: true,
        })
    }

    fn throw_expression(ty: &TypeRef, decode: Expression) -> Result<Expression> {
        match ty {
            TypeRef::String => Ok(Expression::call(
                "FfiError",
                [Expression::labeled("message", decode)]
                    .into_iter()
                    .collect::<ArgumentList>(),
            )),
            TypeRef::Record(_) | TypeRef::Enum(_) => Ok(decode),
            _ => Err(SwiftHost::unsupported("encoded error type")),
        }
    }
}

struct ReturnPlan<'context, 'bindings> {
    bridge: &'context CBridgeContract,
    context: &'context RenderContext<'bindings, Native>,
    c_return_channel: ReturnChannel,
}

impl<'plan, 'context, 'bindings> ReturnPlanRender<'plan, Native, OutOfRust>
    for ReturnPlan<'context, 'bindings>
{
    type Output = Result<Return>;

    fn void(&mut self) -> Self::Output {
        Ok(Return {
            ty: None,
            conversion: ReturnConversion::Direct,
            success: None,
        })
    }

    fn direct(&mut self, slot: ReturnValueSlot, ty: &'plan DirectValueType) -> Self::Output {
        let ty = match ty {
            DirectValueType::Primitive(primitive) => SwiftType::primitive(*primitive)?,
            DirectValueType::Record(record) => {
                let api_ty = SwiftType::record(*record, self.context)?;
                let storage_ty = self.direct_record_storage(*record)?;
                return Ok(Return {
                    ty: Some(api_ty.clone()),
                    conversion: ReturnConversion::FromC(api_ty),
                    success: self.success_slot(slot, storage_ty)?,
                });
            }
            DirectValueType::Enum(enumeration) => {
                let api_ty = SwiftType::enumeration(*enumeration, self.context)?;
                let storage_ty = self.c_style_enum_storage(*enumeration)?;
                return Ok(Return {
                    ty: Some(api_ty.clone()),
                    conversion: ReturnConversion::FromC(api_ty),
                    success: self.success_slot(slot, storage_ty)?,
                });
            }
            _ => return Err(SwiftHost::unsupported("unknown direct return")),
        };
        Ok(Return {
            ty: Some(ty.clone()),
            conversion: ReturnConversion::Direct,
            success: self.success_slot(slot, ty)?,
        })
    }

    fn encoded(
        &mut self,
        slot: ReturnValueSlot,
        ty: &'plan TypeRef,
        codec: &'plan <OutOfRust as Direction>::Codec,
        shape: <Native as Surface>::BufferShape,
    ) -> Self::Output {
        if shape != native::BufferShape::Buffer {
            return Err(SwiftHost::unsupported("encoded return shape"));
        }
        let reader = GeneratedLocal::WireReader.identifier()?;
        let decode = codec.render_with(&mut Reader::new(reader.clone(), self.context))?;
        Ok(Return {
            ty: Some(SwiftType::type_ref(ty, self.context)?),
            conversion: ReturnConversion::Encoded(EncodedReturn::new(decode, self.bridge)?),
            success: self.success_slot(slot, TypeName::new("FfiBuf_u8"))?,
        })
    }

    fn handle(
        &mut self,
        slot: ReturnValueSlot,
        target: &'plan HandleTarget,
        carrier: <Native as Surface>::HandleCarrier,
        presence: HandlePresence,
    ) -> Self::Output {
        match target {
            HandleTarget::Class(class) => {
                let handle = ClassHandle::new(*class, presence, self.context)?;
                Ok(Return {
                    ty: Some(handle.api_type()),
                    conversion: ReturnConversion::ClassHandle(handle),
                    success: self.success_slot(slot, SwiftType::handle_carrier(carrier)?)?,
                })
            }
            HandleTarget::Callback(_) => Err(SwiftHost::unsupported("callback handle return")),
            HandleTarget::Stream(_) => Err(SwiftHost::unsupported("stream handle return")),
            _ => Err(SwiftHost::unsupported("unknown handle return")),
        }
    }

    fn scalar_option(&mut self, primitive: Primitive) -> Self::Output {
        let reader = GeneratedLocal::WireReader.identifier()?;
        Ok(Return {
            ty: Some(SwiftPrimitive::new(primitive).api_type()?.optional()),
            conversion: ReturnConversion::Encoded(EncodedReturn::new(
                ScalarOption::new(primitive).read(reader)?,
                self.bridge,
            )?),
            success: None,
        })
    }

    fn direct_vector(&mut self, _element: &'plan DirectVectorElementType) -> Self::Output {
        Err(SwiftHost::unsupported("direct vector return"))
    }

    fn closure(&mut self, _closure: &'plan ClosureReturn<Native, OutOfRust>) -> Self::Output {
        Err(SwiftHost::unsupported("closure return"))
    }
}

impl<'context, 'bindings> ReturnPlan<'context, 'bindings> {
    fn success_slot(&self, slot: ReturnValueSlot, ty: TypeName) -> Result<Option<SuccessSlot>> {
        match (slot, self.c_return_channel) {
            (ReturnValueSlot::ReturnSlot, ReturnChannel::Value) => Ok(None),
            (ReturnValueSlot::OutPointer, ReturnChannel::EncodedError) => {
                SuccessSlot::new(ty).map(Some)
            }
            (ReturnValueSlot::OutPointer, ReturnChannel::Value) => {
                Err(SwiftHost::unsupported("out pointer return"))
            }
            (ReturnValueSlot::ReturnSlot, ReturnChannel::EncodedError) => {
                Err(Error::BrokenBridgeContract {
                    bridge: SwiftHost::TARGET,
                    invariant: "error return channel without success out pointer",
                })
            }
            _ => Err(SwiftHost::unsupported("unknown return slot")),
        }
    }

    fn direct_record_storage(&self, id: boltffi_binding::RecordId) -> Result<TypeName> {
        self.bridge
            .source_direct_record(id)
            .map(|record| TypeName::new(record.name()))
            .ok_or(Error::BrokenBridgeContract {
                bridge: SwiftHost::TARGET,
                invariant: "missing C direct record for Swift return",
            })
    }

    fn c_style_enum_storage(&self, id: boltffi_binding::EnumId) -> Result<TypeName> {
        self.bridge
            .source_c_style_enum(id)
            .map(|enumeration| TypeName::new(enumeration.name()))
            .ok_or(Error::BrokenBridgeContract {
                bridge: SwiftHost::TARGET,
                invariant: "missing C enum for Swift return",
            })
    }
}
