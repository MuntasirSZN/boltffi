use crate::bridge::{
    c::{ArgumentList, Expression, Identifier, Literal, Statement, TypeFragment},
    jni::{ClosureArgument, ClosureBytesArgument, ClosureCParameter, ClosureRegistration},
};

pub struct ClosureRegistrationView {
    pub class: Literal,
    pub global_class: Identifier,
    pub call_method: Identifier,
    pub free_method: Identifier,
    pub load: Identifier,
    pub unload: Identifier,
    pub call: Identifier,
    pub release: Identifier,
    pub c_return_type: TypeFragment,
    pub returns_void: bool,
    pub returns_byte_array: bool,
    pub returns_bytes: bool,
    pub returns_record: bool,
    pub method_signature: Literal,
    pub call_method_suffix: String,
    pub failure_value: Expression,
    pub c_parameters: Vec<ClosureCParameterView>,
    pub byte_arrays: Vec<ClosureBytesArgumentView>,
    pub jni_arguments: ArgumentList,
    pub has_jni_arguments: bool,
    pub handle_parameters: Vec<ClosureCParameterView>,
    pub handle_byte_arrays: Vec<ClosureBytesArgumentView>,
    pub rust_arguments: ArgumentList,
    pub has_rust_arguments: bool,
}

pub struct CallbackClosureHandleView {
    pub ty: Identifier,
    pub new: Identifier,
    pub ref_: Identifier,
    pub release: Identifier,
    pub call_symbol: Identifier,
    pub release_symbol: Identifier,
    pub call_field: Statement,
    pub jni_return_type: TypeFragment,
    pub failure_value: Expression,
    pub closure: ClosureRegistrationView,
}

pub struct ClosureCParameterView {
    pub declaration: Statement,
}

pub struct ClosureBytesArgumentView {
    pub name: Identifier,
    pub pointer: Identifier,
    pub length: Identifier,
    pub buffer: Identifier,
}

impl ClosureRegistrationView {
    pub fn from_registration(registration: &ClosureRegistration) -> Self {
        let arguments = registration.arguments();
        Self {
            class: Literal::string(&registration.class().as_jni_class_name()),
            global_class: registration.global_class().clone(),
            call_method: registration.call_method().clone(),
            free_method: registration.free_method().clone(),
            load: registration.load().clone(),
            unload: registration.unload().clone(),
            call: registration.call().clone(),
            release: registration.release().clone(),
            c_return_type: registration.c_return_type().clone(),
            returns_void: registration.returns_void(),
            returns_byte_array: registration.returns_byte_array(),
            returns_bytes: registration.returns_bytes(),
            returns_record: registration.returns_record(),
            method_signature: Literal::string(&registration.method_signature()),
            call_method_suffix: registration
                .call_method_suffix()
                .unwrap_or_default()
                .to_owned(),
            failure_value: registration
                .failure_value()
                .unwrap_or_else(|| Expression::literal(Literal::integer_zero())),
            c_parameters: arguments
                .iter()
                .flat_map(ClosureArgument::c_parameters)
                .map(ClosureCParameterView::from_parameter)
                .collect(),
            byte_arrays: arguments
                .iter()
                .filter_map(ClosureArgument::call_bytes)
                .map(ClosureBytesArgumentView::from_argument)
                .collect(),
            jni_arguments: ClosureArgument::jvm_argument_list(arguments),
            has_jni_arguments: !arguments.is_empty(),
            handle_parameters: arguments
                .iter()
                .flat_map(ClosureArgument::handle_parameters)
                .map(ClosureCParameterView::from_parameter)
                .collect(),
            handle_byte_arrays: arguments
                .iter()
                .filter_map(ClosureArgument::handle_bytes)
                .map(ClosureBytesArgumentView::from_argument)
                .collect(),
            rust_arguments: ClosureArgument::rust_argument_list(arguments),
            has_rust_arguments: !arguments.is_empty(),
        }
    }
}

impl CallbackClosureHandleView {
    pub fn from_registration(registration: &ClosureRegistration) -> Option<Self> {
        registration.callback_handle().map(|handle| Self {
            ty: handle.ty().clone(),
            new: handle.new_function().clone(),
            ref_: handle.ref_function().clone(),
            release: handle.release_function().clone(),
            call_symbol: handle.call_symbol().as_identifier().clone(),
            release_symbol: handle.release_symbol().as_identifier().clone(),
            call_field: handle.call_field().clone(),
            jni_return_type: registration.callback_return_type(),
            failure_value: registration
                .callback_failure_value()
                .unwrap_or_else(|| Expression::literal(Literal::integer_zero())),
            closure: ClosureRegistrationView::from_registration(registration),
        })
    }
}

impl ClosureCParameterView {
    fn from_parameter(parameter: ClosureCParameter) -> Self {
        Self {
            declaration: parameter.declaration().clone(),
        }
    }
}

impl ClosureBytesArgumentView {
    fn from_argument(argument: &ClosureBytesArgument) -> Self {
        Self {
            name: argument.name().clone(),
            pointer: argument.pointer().clone(),
            length: argument.length().clone(),
            buffer: argument.buffer().clone(),
        }
    }
}
