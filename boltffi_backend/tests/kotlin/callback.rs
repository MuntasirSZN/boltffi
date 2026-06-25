use super::rendered_fixture;

#[test]
fn kotlin_target_renders_callback_handle_parameters() {
    insta::assert_snapshot!(rendered_fixture("callback/foreign_callback_parameter"));
}

#[test]
fn kotlin_target_renders_callback_enum_parameters() {
    insta::assert_snapshot!(rendered_fixture("callback/callback_enum_parameter"));
}
