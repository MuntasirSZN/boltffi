use boltffi_ast::PackageInfo;
use boltffi_backend::target::swift::SwiftHost;
use boltffi_binding::{Native, lower};

mod source;

use source::SourceFixture;

fn bindings(source: &str) -> boltffi_binding::Bindings<Native> {
    let file = syn::parse_str(source).expect("valid source fixture");
    let source =
        boltffi_scan::scan_file(file, PackageInfo::new("demo", None)).expect("fixture should scan");
    lower::<Native>(&source).expect("fixture should lower")
}

fn rendered_fixture(name: &str) -> String {
    rendered_source(SourceFixture::one(name))
}

fn rendered_source(fixture: SourceFixture) -> String {
    let host = SwiftHost::new("DemoFFI").expect("Swift host");
    let bindings = bindings(&fixture.read());
    let target = host.into_target().expect("Swift target");
    let output = target.render(&bindings).expect("Swift target renders");
    let swift_file = output
        .files()
        .iter()
        .find(|file| {
            file.path()
                .as_path()
                .extension()
                .is_some_and(|extension| extension == "swift")
        })
        .expect("Swift target should render a Swift source file");
    format!(
        "===== {} =====\n{}",
        swift_file.path().as_path().display(),
        swift_file.contents()
    )
}

#[test]
fn swift_target_renders_primitive_function_stack() {
    insta::assert_snapshot!(rendered_fixture("exports/single_function"));
}

#[test]
fn swift_target_renders_string_function_stack() {
    insta::assert_snapshot!(rendered_fixture("exports/string_functions"));
}

#[test]
fn swift_target_renders_encoded_function_stack() {
    insta::assert_snapshot!(rendered_source(SourceFixture::many([
        "records/person",
        "enums/shape",
        "enums/message",
        "exports/encoded_functions",
    ])));
}

#[test]
fn swift_target_renders_encoded_record_stack() {
    insta::assert_snapshot!(rendered_source(SourceFixture::many([
        "enums/role",
        "records/encoded_user",
        "exports/encoded_record_functions",
    ])));
}

#[test]
fn swift_target_renders_nullable_primitive_function_stack() {
    insta::assert_snapshot!(rendered_fixture("exports/nullable_primitive_functions"));
}

#[test]
fn swift_target_renders_fallible_functions_as_throwing_functions() {
    insta::assert_snapshot!(rendered_fixture("exports/fallible_returns"));
}

#[test]
fn swift_target_renders_direct_records_and_c_style_enums() {
    insta::assert_snapshot!(rendered_fixture("exports/direct_records_and_c_style_enums"));
}

#[test]
fn swift_target_renders_documented_record_and_enum_methods() {
    insta::assert_snapshot!(rendered_fixture(
        "associated/direct_record_and_enum_callables"
    ));
}

#[test]
fn swift_target_renders_class_handles_and_methods() {
    insta::assert_snapshot!(rendered_fixture("exports/kotlin_class_handles"));
}
