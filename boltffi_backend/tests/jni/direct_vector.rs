use super::rendered_fixture;

#[test]
fn jni_bridge_maps_primitive_direct_vectors_to_java_primitive_arrays() {
    insta::assert_snapshot!(rendered_fixture(
        "direct_vector/jni_bridge_maps_primitive_direct_vectors_to_java_primitive_arrays"
    ));
}

#[test]
fn jni_bridge_maps_direct_record_vectors_to_java_byte_arrays() {
    insta::assert_snapshot!(rendered_fixture(
        "direct_vector/jni_bridge_maps_direct_record_vectors_to_java_byte_arrays"
    ));
}

#[test]
fn jni_bridge_maps_callback_direct_vectors_to_java_primitive_arrays() {
    insta::assert_snapshot!(rendered_fixture(
        "direct_vector/jni_bridge_maps_callback_direct_vectors_to_java_primitive_arrays"
    ));
}
