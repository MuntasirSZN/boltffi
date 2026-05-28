use boltffi_ast::{CanonicalName, ExecutionKind, FunctionDef, FunctionId, ParameterDef};

use crate::{ModulePath, ScanError, name, ty, visibility};

pub fn scan_function(item: &syn::ItemFn, module: &ModulePath) -> Result<FunctionDef, ScanError> {
    let ident = &item.sig.ident;
    let mut function = FunctionDef::new(
        FunctionId::new(module.qualified(&ident.to_string())),
        name::canonical(ident),
    );
    function.source = visibility::scan(&item.vis);
    function.execution = execution(&item.sig);
    function.parameters = parameters(&item.sig)?;
    function.returns = ty::scan_return(&item.sig.output)?;
    Ok(function)
}

fn execution(signature: &syn::Signature) -> ExecutionKind {
    match signature.asyncness {
        Some(_) => ExecutionKind::Async,
        None => ExecutionKind::Sync,
    }
}

fn parameters(signature: &syn::Signature) -> Result<Vec<ParameterDef>, ScanError> {
    signature.inputs.iter().map(parameter).collect()
}

fn parameter(arg: &syn::FnArg) -> Result<ParameterDef, ScanError> {
    let syn::FnArg::Typed(typed) = arg else {
        return Err(ScanError::ReceiverOnFreeFunction);
    };
    let binding_name = parameter_name(&typed.pat)?;
    let type_expr = ty::scan_type(&typed.ty)?;
    Ok(ParameterDef::value(binding_name, type_expr))
}

fn parameter_name(pat: &syn::Pat) -> Result<CanonicalName, ScanError> {
    match pat {
        syn::Pat::Ident(binding) => Ok(name::canonical(&binding.ident)),
        _ => Err(ScanError::UnnamedParameter),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use boltffi_ast::{
        CallableForm, ClosureKind, HandlePresence, NamePart, ParameterPassing, Primitive,
        ReturnDef, Source, TypeExpr, Visibility,
    };

    fn parse(source: &str) -> syn::ItemFn {
        syn::parse_str(source).expect("valid function source")
    }

    fn scan(source: &str) -> Result<FunctionDef, ScanError> {
        scan_function(&parse(source), &ModulePath::root("demo"))
    }

    fn name(parts: &[&str]) -> CanonicalName {
        CanonicalName::new(parts.iter().copied().map(NamePart::new).collect())
    }

    #[test]
    fn scans_complete_primitive_free_function_contract() {
        let function = scan("pub fn add(a: i32, b: i32) -> i32 { a + b }").expect("scan");
        let mut expected = FunctionDef::new(FunctionId::new("demo::add"), name(&["add"]));
        expected.form = CallableForm::Function;
        expected.execution = ExecutionKind::Sync;
        expected.parameters = vec![
            ParameterDef::value(name(&["a"]), TypeExpr::Primitive(Primitive::I32)),
            ParameterDef::value(name(&["b"]), TypeExpr::Primitive(Primitive::I32)),
        ];
        expected.returns = ReturnDef::Value(TypeExpr::Primitive(Primitive::I32));
        expected.source = Source::new(Visibility::Public, None);

        assert_eq!(function, expected);
    }

    #[test]
    fn explicit_and_implicit_unit_returns_are_void() {
        let explicit = scan("pub fn explicit() -> () {}").expect("scan");
        let implicit = scan("pub fn implicit() {}").expect("scan");

        assert_eq!(explicit.returns, ReturnDef::Void);
        assert_eq!(implicit.returns, ReturnDef::Void);
    }

    #[test]
    fn async_function_records_only_execution_change() {
        let function = scan("pub async fn spin() {}").expect("scan");

        assert_eq!(function.execution, ExecutionKind::Async);
        assert_eq!(function.form, CallableForm::Function);
        assert!(function.parameters.is_empty());
        assert_eq!(function.returns, ReturnDef::Void);
    }

    #[test]
    fn closure_return_records_complete_signature_contract() {
        let function =
            scan("pub fn make_handler() -> impl Send + FnMut(u32, bool) -> i64 { todo!() }")
                .expect("scan");

        let ReturnDef::Value(TypeExpr::Closure {
            signature,
            presence,
        }) = function.returns
        else {
            panic!("expected closure return");
        };
        assert_eq!(presence, HandlePresence::Required);
        assert_eq!(signature.kind, ClosureKind::FnMut);
        assert_eq!(
            signature.parameters,
            vec![
                TypeExpr::Primitive(Primitive::U32),
                TypeExpr::Primitive(Primitive::Bool)
            ]
        );
        assert_eq!(
            signature.returns,
            ReturnDef::Value(TypeExpr::Primitive(Primitive::I64))
        );
    }

    #[test]
    fn closure_return_without_arrow_records_void_invoke_return() {
        let function = scan("pub fn make_handler() -> impl FnOnce(u32) { todo!() }").expect("scan");

        let ReturnDef::Value(TypeExpr::Closure { signature, .. }) = function.returns else {
            panic!("expected closure return");
        };

        assert_eq!(signature.kind, ClosureKind::FnOnce);
        assert_eq!(
            signature.parameters,
            vec![TypeExpr::Primitive(Primitive::U32)]
        );
        assert_eq!(signature.returns, ReturnDef::Void);
    }

    #[test]
    fn non_primitive_parameter_is_rejected_until_records_land() {
        let error = scan("pub fn make(point: Point) {}").expect_err("non-primitive must reject");

        assert!(matches!(
            error,
            ScanError::UnsupportedType { spelling } if spelling == "Point"
        ));
    }

    #[test]
    fn scans_restricted_function_visibility_without_touching_parameters() {
        let function = scan("pub(crate) fn add(a: i32) -> i32 { a }").expect("scan");

        assert_eq!(
            function.source.visibility,
            Visibility::Restricted("crate".to_owned())
        );
        assert_eq!(function.parameters[0].source.visibility, Visibility::Public);
        assert_eq!(function.parameters[0].passing, ParameterPassing::Value);
    }

    #[test]
    fn scans_multi_word_function_and_parameter_names_as_parts() {
        let function = scan("pub fn make_handler(user_id: i32) -> i32 { user_id }").expect("scan");

        assert_eq!(function.id, FunctionId::new("demo::make_handler"));
        assert_eq!(function.name, name(&["make", "handler"]));
        assert_eq!(function.parameters[0].name, name(&["user", "id"]));
    }

    #[test]
    fn rejects_receiver_on_free_function() {
        let error = scan("pub fn distance(&self) -> f64 { 0.0 }").expect_err("receiver rejected");

        assert_eq!(error, ScanError::ReceiverOnFreeFunction);
    }

    #[test]
    fn rejects_non_binding_parameter_pattern_before_type_scanning() {
        let error =
            scan("pub fn sum((x, y): (i32, i32)) -> i32 { x + y }").expect_err("pattern rejected");

        assert_eq!(error, ScanError::UnnamedParameter);
    }
}
