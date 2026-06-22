use std::path::Path;

use super::{bridge_fixture, rendered_fixture};

#[test]
fn jni_bridge_layers_primitive_functions_on_c_bridge() {
    insta::assert_snapshot!(rendered_fixture(
        "native_methods/jni_bridge_layers_primitive_functions_on_c_bridge"
    ));
}

#[test]
fn jni_bridge_contract_records_class_and_source_path() {
    let output = bridge_fixture("native_methods/jni_bridge_contract_records_class_and_source_path");
    let contract = output.contract();

    assert_eq!(contract.class().as_java_path(), "com.boltffi.demo.Native");
    assert_eq!(
        contract.source_path().as_path(),
        Path::new("jni/jni_glue.c")
    );
    assert_eq!(contract.c_header().as_str(), "demo.h");
    assert_eq!(contract.methods().len(), 1);
    assert_eq!(
        contract.methods()[0].symbol().to_string(),
        "Java_com_boltffi_demo_Native_boltffi_1function_1demo_1add"
    );
}

#[test]
fn jni_bridge_renders_direct_records_and_c_style_enums() {
    insta::assert_snapshot!(rendered_fixture(
        "native_methods/jni_bridge_renders_direct_records_and_c_style_enums"
    ));
}

#[test]
fn jni_bridge_renders_encoded_functions_as_byte_arrays() {
    insta::assert_snapshot!(rendered_fixture(
        "native_methods/jni_bridge_renders_encoded_functions_as_byte_arrays"
    ));
}

#[test]
fn jni_bridge_renders_custom_type_functions_as_byte_arrays() {
    insta::assert_snapshot!(rendered_fixture(
        "native_methods/jni_bridge_renders_custom_type_functions_as_byte_arrays"
    ));
}

#[test]
fn jni_bridge_renders_class_handles_and_methods() {
    insta::assert_snapshot!(rendered_fixture(
        "native_methods/jni_bridge_renders_class_handles_and_methods"
    ));
}

#[test]
fn jni_bridge_renders_async_class_methods() {
    insta::assert_snapshot!(rendered_fixture(
        "native_methods/jni_bridge_renders_async_class_methods"
    ));
}

#[test]
fn jni_bridge_casts_async_handles_and_callbacks_to_c_abi_types() {
    insta::assert_snapshot!(rendered_fixture(
        "native_methods/jni_bridge_casts_async_handles_and_callbacks_to_c_abi_types"
    ));
}

#[test]
fn jni_bridge_renders_async_complete_return_shapes() {
    insta::assert_snapshot!(rendered_fixture(
        "native_methods/jni_bridge_renders_async_complete_return_shapes"
    ));
}

#[test]
fn jni_bridge_renders_closure_parameters_from_contract_group() {
    insta::assert_snapshot!(rendered_fixture(
        "native_methods/jni_bridge_renders_closure_parameters_from_contract_group"
    ));
}

#[test]
fn jni_bridge_preserves_multi_argument_closure_signature_names() {
    insta::assert_snapshot!(rendered_fixture(
        "native_methods/jni_bridge_preserves_multi_argument_closure_signature_names"
    ));
}

#[test]
fn jni_bridge_renders_encoded_closure_parameters_from_contract_group() {
    insta::assert_snapshot!(rendered_fixture(
        "native_methods/jni_bridge_renders_encoded_closure_parameters_from_contract_group"
    ));
}

#[test]
fn jni_bridge_renders_encoded_closure_return_shapes_as_byte_arrays() {
    insta::assert_snapshot!(rendered_fixture(
        "native_methods/jni_bridge_renders_encoded_closure_return_shapes_as_byte_arrays"
    ));
}

#[test]
fn jni_bridge_renders_c_style_enum_closure_returns_as_scalars() {
    insta::assert_snapshot!(rendered_fixture(
        "native_methods/jni_bridge_renders_c_style_enum_closure_returns_as_scalars"
    ));
}

#[test]
fn jni_bridge_renders_direct_vector_closure_parameters_from_contract_group() {
    insta::assert_snapshot!(rendered_fixture(
        "native_methods/jni_bridge_renders_direct_vector_closure_parameters_from_contract_group"
    ));
}

#[test]
fn jni_bridge_renders_nested_closure_parameters_from_contract_group() {
    insta::assert_snapshot!(rendered_fixture(
        "native_methods/jni_bridge_renders_nested_closure_parameters_from_contract_group"
    ));
}

#[test]
fn jni_bridge_renders_nested_closure_parameters_for_callback_owned_closures() {
    insta::assert_snapshot!(rendered_fixture(
        "native_methods/jni_bridge_renders_nested_closure_parameters_for_callback_owned_closures"
    ));
}

#[test]
fn jni_bridge_renders_closure_callback_handle_returns() {
    insta::assert_snapshot!(rendered_fixture(
        "native_methods/jni_bridge_renders_closure_callback_handle_returns"
    ));
}

#[test]
fn jni_bridge_renders_closure_direct_record_returns() {
    insta::assert_snapshot!(rendered_fixture(
        "native_methods/jni_bridge_renders_closure_direct_record_returns"
    ));
}

#[test]
fn jni_bridge_renders_closure_class_handle_returns() {
    insta::assert_snapshot!(rendered_fixture(
        "native_methods/jni_bridge_renders_closure_class_handle_returns"
    ));
}
