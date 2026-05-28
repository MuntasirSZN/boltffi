use boltffi_ast::{FunctionDef, FunctionId, ParameterDef};

use crate::registry::TypeRegistry;
use crate::ty::TypeScanner;
use crate::{ModulePath, ScanError, name, signature, visibility};

pub(crate) fn scan_function(
    item: &syn::ItemFn,
    module: &ModulePath,
    registry: &TypeRegistry,
) -> Result<FunctionDef, ScanError> {
    let ident = &item.sig.ident;
    let mut function = FunctionDef::new(
        FunctionId::new(module.qualified(&ident.to_string())),
        name::canonical(ident),
    );
    let scanner = TypeScanner::new(registry);
    function.source = visibility::scan(&item.vis);
    function.execution = signature::execution(&item.sig);
    function.parameters = parameters(&item.sig, &scanner)?;
    function.returns = scanner.scan_return(&item.sig.output)?;
    Ok(function)
}

fn parameters(
    sig: &syn::Signature,
    scanner: &TypeScanner<'_>,
) -> Result<Vec<ParameterDef>, ScanError> {
    sig.inputs
        .iter()
        .map(|argument| match argument {
            syn::FnArg::Typed(typed) => signature::parameter(typed, scanner),
            syn::FnArg::Receiver(_) => Err(ScanError::ReceiverOnFreeFunction),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use boltffi_ast::{
        CallableForm, CanonicalName, ClosureKind, ExecutionKind, HandlePresence, NamePart,
        ParameterPassing, Primitive, RecordId, ReturnDef, Source, TypeExpr, Visibility,
    };

    fn parse(source: &str) -> syn::ItemFn {
        syn::parse_str(source).expect("valid function source")
    }

    fn scan(source: &str) -> Result<FunctionDef, ScanError> {
        scan_function(
            &parse(source),
            &ModulePath::root("demo"),
            &TypeRegistry::new(),
        )
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
    fn non_primitive_parameter_is_rejected_without_registration() {
        let error = scan("pub fn make(point: Point) {}").expect_err("unregistered type rejects");

        assert!(matches!(
            error,
            ScanError::UnsupportedType { spelling } if spelling == "Point"
        ));
    }

    #[test]
    fn resolves_record_typed_parameter_and_return_against_registry() {
        let mut registry = TypeRegistry::new();
        registry.register_record("Point", RecordId::new("demo::Point"));
        let function = scan_function(
            &parse("pub fn translate(point: Point, dx: f64) -> Point { point }"),
            &ModulePath::root("demo"),
            &registry,
        )
        .expect("scan");

        assert_eq!(
            function.parameters[0].type_expr,
            TypeExpr::Record(RecordId::new("demo::Point"))
        );
        assert_eq!(
            function.returns,
            ReturnDef::Value(TypeExpr::Record(RecordId::new("demo::Point")))
        );
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
