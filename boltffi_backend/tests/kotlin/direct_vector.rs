use super::rendered_fixture;

#[test]
fn kotlin_target_passes_signed_primitive_vectors_as_jni_arrays() {
    insta::assert_snapshot!(rendered_fixture("direct_vector/primitive_vector_parameter"));
}

#[test]
fn kotlin_target_passes_direct_record_vectors_as_packed_bytes() {
    insta::assert_snapshot!(rendered_fixture("direct_vector/record_vector_parameter"));
}

#[test]
fn kotlin_target_returns_primitive_vectors_from_native_buffers() {
    insta::assert_snapshot!(rendered_fixture("direct_vector/primitive_vector_return"));
}

#[test]
fn kotlin_target_returns_direct_record_vectors_from_native_buffers() {
    insta::assert_snapshot!(rendered_fixture("direct_vector/record_vector_return"));
}
