use super::rendered_fixture;

#[test]
fn jni_bridge_renders_callback_handle_parameters() {
    insta::assert_snapshot!(rendered_fixture(
        "callback/jni_bridge_renders_callback_handle_parameters"
    ));
}

#[test]
fn jni_bridge_caches_android_callback_threads_with_local_frames() {
    insta::assert_snapshot!(rendered_fixture(
        "callback/jni_bridge_caches_android_callback_threads_with_local_frames"
    ));
}

#[test]
fn jni_bridge_renders_callback_byte_slice_parameters() {
    insta::assert_snapshot!(rendered_fixture(
        "callback/jni_bridge_renders_callback_byte_slice_parameters"
    ));
}

#[test]
fn jni_bridge_renders_callback_handle_method_parameters() {
    insta::assert_snapshot!(rendered_fixture(
        "callback/jni_bridge_renders_callback_handle_method_parameters"
    ));
}

#[test]
fn jni_bridge_renders_callback_record_parameters() {
    insta::assert_snapshot!(rendered_fixture(
        "callback/jni_bridge_renders_callback_record_parameters"
    ));
}

#[test]
fn jni_bridge_renders_callback_closure_parameters() {
    insta::assert_snapshot!(rendered_fixture(
        "callback/jni_bridge_renders_callback_closure_parameters"
    ));
}

#[test]
fn jni_bridge_renders_callback_encoded_closure_parameters() {
    insta::assert_snapshot!(rendered_fixture(
        "callback/jni_bridge_renders_callback_encoded_closure_parameters"
    ));
}

#[test]
fn jni_bridge_renders_callback_direct_vector_closure_parameters() {
    insta::assert_snapshot!(rendered_fixture(
        "callback/jni_bridge_renders_callback_direct_vector_closure_parameters"
    ));
}

#[test]
fn jni_bridge_renders_callback_closure_returns() {
    insta::assert_snapshot!(rendered_fixture(
        "callback/jni_bridge_renders_callback_closure_returns"
    ));
}

#[test]
fn jni_bridge_renders_callback_handle_method_closure_returns() {
    insta::assert_snapshot!(rendered_fixture(
        "callback/jni_bridge_renders_callback_handle_method_closure_returns"
    ));
}

#[test]
fn jni_bridge_renders_callback_encoded_closure_returns() {
    insta::assert_snapshot!(rendered_fixture(
        "callback/jni_bridge_renders_callback_encoded_closure_returns"
    ));
}

#[test]
fn jni_bridge_renders_callback_encoded_returns() {
    insta::assert_snapshot!(rendered_fixture(
        "callback/jni_bridge_renders_callback_encoded_returns"
    ));
}

#[test]
fn jni_bridge_keeps_callback_status_param_separate_from_generated_status() {
    insta::assert_snapshot!(rendered_fixture(
        "callback/jni_bridge_keeps_callback_status_param_separate_from_generated_status"
    ));
}

#[test]
fn jni_bridge_renders_callback_record_returns() {
    insta::assert_snapshot!(rendered_fixture(
        "callback/jni_bridge_renders_callback_record_returns"
    ));
}

#[test]
fn jni_bridge_renders_async_callback_completions() {
    insta::assert_snapshot!(rendered_fixture(
        "callback/jni_bridge_renders_async_callback_completions"
    ));
}

#[test]
fn jni_bridge_renders_async_callback_completion_shapes() {
    insta::assert_snapshot!(rendered_fixture(
        "callback/jni_bridge_renders_async_callback_completion_shapes"
    ));
}

#[test]
fn jni_bridge_renders_c_style_enum_async_callback_completion_payloads() {
    insta::assert_snapshot!(rendered_fixture(
        "callback/jni_bridge_renders_c_style_enum_async_callback_completion_payloads"
    ));
}

#[test]
fn jni_bridge_renders_async_callback_handle_completion_payloads() {
    insta::assert_snapshot!(rendered_fixture(
        "callback/jni_bridge_renders_async_callback_handle_completion_payloads"
    ));
}

#[test]
fn jni_bridge_renders_callback_handle_returns() {
    insta::assert_snapshot!(rendered_fixture(
        "callback/jni_bridge_renders_callback_handle_returns"
    ));
}

#[test]
fn jni_bridge_renders_async_callback_handle_methods() {
    insta::assert_snapshot!(rendered_fixture(
        "callback/jni_bridge_renders_async_callback_handle_methods"
    ));
}

#[test]
fn jni_bridge_renders_async_callback_handle_method_payloads() {
    insta::assert_snapshot!(rendered_fixture(
        "callback/jni_bridge_renders_async_callback_handle_method_payloads"
    ));
}

#[test]
fn jni_bridge_renders_callback_method_callback_handle_returns() {
    insta::assert_snapshot!(rendered_fixture(
        "callback/jni_bridge_renders_callback_method_callback_handle_returns"
    ));
}

#[test]
fn jni_bridge_renders_nullable_callback_handle_returns() {
    insta::assert_snapshot!(rendered_fixture(
        "callback/jni_bridge_renders_nullable_callback_handle_returns"
    ));
}
