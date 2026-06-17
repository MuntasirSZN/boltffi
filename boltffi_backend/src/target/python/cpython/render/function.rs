use askama::Template as AskamaTemplate;
use boltffi_binding::{
    ErrorDecl, ExecutionDecl, ExportedCallable, FunctionDecl, Native, NativeSymbol, OutOfRust,
    ReturnPlan, TypeRef, native,
};

use crate::{
    bridge::{
        c::{self, syntax::TypeSyntax},
        python_cext::{ExtensionMethod, LoadedFunction, MethodFlags, PythonCExtBridgeContract},
    },
    core::{Emitted, Error, RenderContext, Result},
    target::python::{
        cpython::render::{argument, direct_vector, primitive, result},
        name_style::Name,
    },
};

#[derive(AskamaTemplate)]
#[template(path = "target/python/function.c", escape = "none")]
struct FunctionTemplate {
    python_name: String,
    wrapper: String,
    storage: String,
    params: Vec<argument::Conversion>,
    call_args: Vec<String>,
    returns: result::Conversion,
    fallible: Option<FallibleResult>,
}

pub struct Function {
    pub python_name: String,
    pub wrapper: String,
    pub storage: String,
    pub params: Vec<argument::Conversion>,
    pub call_args: Vec<String>,
    pub returns: result::Conversion,
    fallible: Option<FallibleResult>,
    method: ExtensionMethod,
}

impl Function {
    pub fn supports(callable: &ExportedCallable<Native>) -> bool {
        Self::unsupported(callable).is_none()
    }

    pub fn unsupported(callable: &ExportedCallable<Native>) -> Option<&'static str> {
        if !matches!(callable.execution(), ExecutionDecl::Synchronous(_)) {
            return Some("async function");
        }
        if !callable.params().iter().all(argument::Conversion::supports) {
            return Some("function parameter");
        }
        match callable.error() {
            ErrorDecl::None(_) if result::Conversion::supports(callable.returns().plan()) => None,
            ErrorDecl::None(_) => Some("function return"),
            ErrorDecl::EncodedViaReturnSlot {
                ty,
                shape: native::BufferShape::Buffer,
                ..
            } if result::Conversion::supports_out(callable.returns().plan())
                && result::Conversion::supports_encoded(ty) =>
            {
                None
            }
            ErrorDecl::EncodedViaReturnSlot { .. } => Some("fallible function"),
            _ => Some("function error channel"),
        }
    }

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
        match callable.execution() {
            ExecutionDecl::Synchronous(_) => {}
            ExecutionDecl::Asynchronous(_) => {
                return Err(Error::UnsupportedTarget {
                    target: "python",
                    shape: "async function",
                });
            }
            _ => {
                return Err(Error::UnsupportedTarget {
                    target: "python",
                    shape: "unknown function execution",
                });
            }
        }
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
            .map(|(offset, parameter)| {
                let index = receiver_args.len() + offset;
                argument::Conversion::from_parameter(index, parameter, bridge, context)
            })
            .collect::<Result<Vec<_>>>()?;
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
        Ok(Self {
            python_name,
            wrapper,
            storage: loaded.storage_name().to_owned(),
            params,
            call_args,
            returns,
            fallible,
            method,
        })
    }

    pub fn method(&self) -> &ExtensionMethod {
        &self.method
    }

    pub fn render(self) -> Result<Emitted> {
        let source = FunctionTemplate {
            python_name: self.python_name,
            wrapper: self.wrapper,
            storage: self.storage,
            params: self.params,
            call_args: self.call_args,
            returns: self.returns,
            fallible: self.fallible,
        }
        .render()?;
        Ok(Emitted::primary(source))
    }

    pub fn primitives(&self) -> Vec<primitive::Runtime> {
        let params = self
            .params
            .iter()
            .filter_map(argument::Conversion::primitive)
            .collect::<Vec<_>>();
        params
            .into_iter()
            .chain(self.returns.primitive())
            .chain(self.fallible.iter().filter_map(FallibleResult::primitive))
            .collect()
    }

    pub fn wire_primitives(&self) -> impl Iterator<Item = primitive::Runtime> + '_ {
        self.params
            .iter()
            .filter_map(argument::Conversion::wire_primitive)
    }

    pub fn direct_vector_elements(&self) -> impl Iterator<Item = direct_vector::Element> + '_ {
        self.params
            .iter()
            .filter_map(argument::Conversion::direct_vector_element)
            .chain(self.returns.direct_vector_element())
            .chain(
                self.fallible
                    .iter()
                    .filter_map(FallibleResult::direct_vector_element),
            )
    }

    pub fn owned_buffer(&self) -> Option<result::OwnedBuffer> {
        self.owned_buffers().next()
    }

    pub fn owned_buffers(&self) -> impl Iterator<Item = result::OwnedBuffer> + '_ {
        self.returns.owned_buffer().into_iter().chain(
            self.fallible
                .iter()
                .filter_map(FallibleResult::owned_buffer),
        )
    }

    pub fn has_string_argument(&self) -> bool {
        self.params.iter().any(argument::Conversion::is_string)
    }

    pub fn has_bytes_argument(&self) -> bool {
        self.params.iter().any(argument::Conversion::is_bytes)
    }

    pub fn has_raw_wire_argument(&self) -> bool {
        self.params.iter().any(argument::Conversion::is_raw_wire)
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
