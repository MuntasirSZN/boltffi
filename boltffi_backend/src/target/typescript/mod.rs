mod codec;
mod name_style;
mod primitive;
mod render;
mod syntax;

use boltffi_binding::{
    Bindings, CallbackDecl, ClassDecl, ConstantDecl, CustomTypeDecl, EnumDecl, FunctionDecl,
    RecordDecl, StreamDecl, Wasm32,
};

use crate::{
    bridge::wasm::{WasmBridge, WasmBridgeContract},
    core::{
        BindingCapability, BridgeCapability, CapabilityRequirements, Emitted, Error,
        GeneratedOutput, HostCapabilities, RenderContext, RenderedDeclaration, Result, Target,
        contract::sealed, host,
    },
};

use name_style::ModuleName;
use render::{CustomType, Enumeration, Function, Module, Record};
use syntax::{StringLiteral, Syntax};

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
#[non_exhaustive]
pub struct TypeScriptHost {
    module: ModuleName,
    runtime_package: StringLiteral,
}

impl TypeScriptHost {
    pub fn new(module: impl Into<String>) -> Result<Self> {
        Ok(Self {
            module: ModuleName::parse(module)?,
            runtime_package: StringLiteral::new("@boltffi/runtime"),
        })
    }

    pub fn runtime_package(mut self, package: impl AsRef<str>) -> Self {
        self.runtime_package = StringLiteral::new(package.as_ref());
        self
    }

    pub fn into_target(self) -> Target<Self, WasmBridge> {
        Target::new(self, WasmBridge)
    }

    fn unsupported(shape: &'static str) -> Error {
        Error::UnsupportedTarget {
            target: "typescript",
            shape,
        }
    }
}

impl host::HostBackend for TypeScriptHost {
    type Surface = Wasm32;
    type Bridge = WasmBridgeContract;
    type Syntax = Syntax;

    fn name(&self) -> &'static str {
        "typescript"
    }

    fn binding_capabilities(&self) -> HostCapabilities {
        HostCapabilities::new()
            .in_progress(
                BindingCapability::Records,
                "TypeScript records are being migrated",
            )
            .in_progress(
                BindingCapability::Enums,
                "TypeScript enums are being migrated",
            )
            .stable(BindingCapability::Functions)
            .in_progress(
                BindingCapability::Classes,
                "TypeScript classes are being migrated",
            )
            .in_progress(
                BindingCapability::Callbacks,
                "TypeScript callbacks are being migrated",
            )
            .in_progress(
                BindingCapability::Streams,
                "TypeScript streams are being migrated",
            )
            .in_progress(
                BindingCapability::Constants,
                "TypeScript constants are being migrated",
            )
            .in_progress(
                BindingCapability::CustomTypes,
                "TypeScript custom types are being migrated",
            )
    }

    fn bridge_capabilities(&self) -> CapabilityRequirements<BridgeCapability> {
        CapabilityRequirements::new().require(BridgeCapability::Wasm)
    }

    fn record(
        &self,
        decl: &RecordDecl<Self::Surface>,
        _bridge: &Self::Bridge,
        context: &RenderContext<Self::Surface>,
    ) -> Result<Emitted> {
        Record::from_declaration(decl, context)?.render()
    }

    fn enumeration(
        &self,
        decl: &EnumDecl<Self::Surface>,
        _bridge: &Self::Bridge,
        context: &RenderContext<Self::Surface>,
    ) -> Result<Emitted> {
        Enumeration::from_declaration(decl, context)?.render()
    }

    fn function(
        &self,
        decl: &FunctionDecl<Self::Surface>,
        _bridge: &Self::Bridge,
        context: &RenderContext<Self::Surface>,
    ) -> Result<Emitted> {
        Function::from_declaration(decl, context)?.render()
    }

    fn class(
        &self,
        _decl: &ClassDecl<Self::Surface>,
        _bridge: &Self::Bridge,
        _context: &RenderContext<Self::Surface>,
    ) -> Result<Emitted> {
        Err(Self::unsupported("class"))
    }

    fn callback(
        &self,
        _decl: &CallbackDecl<Self::Surface>,
        _bridge: &Self::Bridge,
        _context: &RenderContext<Self::Surface>,
    ) -> Result<Emitted> {
        Err(Self::unsupported("callback"))
    }

    fn stream(
        &self,
        _decl: &StreamDecl<Self::Surface>,
        _bridge: &Self::Bridge,
        _context: &RenderContext<Self::Surface>,
    ) -> Result<Emitted> {
        Err(Self::unsupported("stream"))
    }

    fn constant(
        &self,
        _decl: &ConstantDecl<Self::Surface>,
        _bridge: &Self::Bridge,
        _context: &RenderContext<Self::Surface>,
    ) -> Result<Emitted> {
        Err(Self::unsupported("constant"))
    }

    fn custom_type(
        &self,
        decl: &CustomTypeDecl,
        _bridge: &Self::Bridge,
        context: &RenderContext<Self::Surface>,
    ) -> Result<Emitted> {
        CustomType::from_declaration(decl, context)?.render()
    }

    fn assemble<'decl>(
        &self,
        _bindings: &Bindings<Self::Surface>,
        _bridge: &Self::Bridge,
        _context: &RenderContext<Self::Surface>,
        declarations: Vec<RenderedDeclaration<'decl, Self::Surface>>,
    ) -> Result<GeneratedOutput> {
        Module::new(&self.module, &self.runtime_package).render(declarations)
    }
}

impl sealed::HostBackend for TypeScriptHost {}

#[cfg(test)]
mod tests {
    use boltffi_ast::PackageInfo;
    use boltffi_binding::{Bindings, Wasm32, lower};

    use super::TypeScriptHost;

    fn bindings() -> Bindings<Wasm32> {
        let source = boltffi_scan::scan_file(
            syn::parse_str(
                r#"
                #[export]
                pub fn noop() {}

                #[export]
                pub fn echo_bool(value: bool) -> bool { value }

                #[export]
                pub fn add(left: i32, right: i32) -> i32 { left + right }

                #[export]
                pub fn echo_u64(value: u64) -> u64 { value }

                #[export]
                pub fn echo_string(value: String) -> String { value }

                #[export]
                pub fn echo_bytes(value: Vec<u8>) -> Vec<u8> { value }

                #[export]
                pub fn echo_vec_i32(value: Vec<i32>) -> Vec<i32> { value }

                #[export]
                pub fn echo_vec_bool(value: Vec<bool>) -> Vec<bool> { value }

                #[export]
                pub fn increment_u64(value: &mut [u64]) {
                    if let Some(first) = value.first_mut() {
                        *first += 1;
                    }
                }

                #[export]
                pub fn echo_optional_i32(value: Option<i32>) -> Option<i32> { value }

                #[export]
                pub fn echo_optional_i64(value: Option<i64>) -> Option<i64> { value }

                #[export]
                pub fn echo_optional_f64(value: Option<f64>) -> Option<f64> { value }

                #[export]
                pub fn echo_optional_vec_i32(value: Option<Vec<i32>>) -> Option<Vec<i32>> { value }

                #[export]
                pub fn echo_vec_string(value: Vec<String>) -> Vec<String> { value }

                #[export]
                pub fn echo_vec_vec_i32(value: Vec<Vec<i32>>) -> Vec<Vec<i32>> { value }
                "#,
            )
            .expect("valid source"),
            PackageInfo::new("demo", None),
        )
        .expect("source scans");
        lower::<Wasm32>(&source).expect("source lowers")
    }

    fn record_bindings() -> Bindings<Wasm32> {
        let source = boltffi_scan::scan_file(
            syn::parse_str(
                r#"
                #[data]
                #[repr(C)]
                pub struct Point {
                    pub x: f64,
                    pub active: bool,
                    pub y: f64,
                }

                #[data]
                pub struct User {
                    pub name: String,
                    pub scores: Vec<i32>,
                }

                #[data]
                #[repr(i8)]
                pub enum Status {
                    Inactive = -1,
                    Active = 1,
                }

                #[data]
                pub enum Filter {
                    None,
                    ByName { name: String },
                    ByRange(i32, i32),
                }

                #[data]
                pub struct Task {
                    pub title: String,
                    pub status: Status,
                }

                #[export]
                pub fn echo_user(value: User) -> User { value }

                #[export]
                pub fn echo_status(value: Status) -> Status { value }

                #[export]
                pub fn echo_task(value: Task) -> Task { value }

                #[export]
                pub fn echo_filter(value: Filter) -> Filter { value }
                "#,
            )
            .expect("valid source"),
            PackageInfo::new("demo", None),
        )
        .expect("source scans");
        lower::<Wasm32>(&source).expect("source lowers")
    }

    #[test]
    fn renders_primitive_functions_through_the_wasm_surface() {
        let output = TypeScriptHost::new("demo")
            .expect("host constructs")
            .into_target()
            .render(&bindings())
            .expect("target renders");

        assert_eq!(output.files().len(), 2);
        let browser = output
            .files()
            .iter()
            .find(|file| file.path().as_path().ends_with("demo.ts"))
            .expect("browser module");
        assert!(browser.contents().contains("export function noop(): void"));
        assert!(
            browser
                .contents()
                .contains("export function echoBool(value: boolean): boolean")
        );
        assert!(browser.contents().contains(
            "return (_exports.boltffi_function_demo_echo_bool as Function)(value) !== 0;"
        ));
        assert!(
            browser
                .contents()
                .contains("export function add(left: number, right: number): number")
        );
        assert!(
            browser
                .contents()
                .contains("export function echoU64(value: bigint): bigint")
        );
        assert!(
            browser
                .contents()
                .contains("const __boltffi_value_allocation = _module.allocString(value);")
        );
        assert!(browser.contents().contains(
            "return _module.takePackedUtf8String((_exports.boltffi_function_demo_echo_string as Function)(__boltffi_value_allocation.ptr, __boltffi_value_allocation.len) as bigint);"
        ));
        assert!(
            browser
                .contents()
                .contains("_module.freeAlloc(__boltffi_value_allocation);")
        );
        assert!(
            browser
                .contents()
                .contains("const __boltffi_value_allocation = _module.allocBytes(value);")
        );
        assert!(browser.contents().contains(
            "return _module.takePackedU8Array((_exports.boltffi_function_demo_echo_bytes as Function)(__boltffi_value_allocation.ptr, __boltffi_value_allocation.len) as bigint);"
        ));
        assert!(browser.contents().contains(
            "export function echoVecI32(value: readonly number[] | Int32Array): Int32Array"
        ));
        assert!(
            browser
                .contents()
                .contains("const __boltffi_value_allocation = _module.allocI32Array(value);")
        );
        assert!(
            browser
                .contents()
                .contains("return _module.takeSlotI32Array();")
        );
        assert!(
            browser
                .contents()
                .contains("export function echoVecBool(value: readonly boolean[]): boolean[]")
        );
        assert!(
            browser
                .contents()
                .contains("export function incrementU64(value: BigUint64Array): void")
        );
        assert!(browser.contents().contains(
            "_module.copyPrimitiveBufferInto(__boltffi_value_allocation, value, \"u64\");"
        ));
        assert!(
            browser
                .contents()
                .contains("export function echoOptionalI32(value: number | null): number | null")
        );
        assert!(browser.contents().contains(
            "(_exports.boltffi_function_demo_echo_optional_i32 as Function)((value === null ? Number.NaN : value))"
        ));
        assert!(browser.contents().contains("_module.unpackOptionI32("));
        assert!(
            browser
                .contents()
                .contains("export function echoOptionalI64(value: bigint | null): bigint | null")
        );
        assert!(browser.contents().contains(
            "const __boltffi_value_writer = _module.allocWriter(wireOptionalSize(value, (__boltffiValue0) => 8));"
        ));
        assert!(
            browser
                .contents()
                .contains("__boltffi_value_writer.writeOptional(value, (__boltffiValue0) => {")
        );
        assert!(
            browser
                .contents()
                .contains("__boltffi_value_writer.writeI64(__boltffiValue0);")
        );
        assert!(
            browser
                .contents()
                .contains("return __boltffiReader.readOptional(() => __boltffiReader.readI64());")
        );
        assert!(
            browser
                .contents()
                .contains("export function echoOptionalF64(value: number | null): number | null")
        );
        assert!(browser.contents().contains(
            "export function echoOptionalVecI32(value: Array<number> | Int32Array | null): Array<number> | Int32Array | null"
        ));
        assert!(
            browser
                .contents()
                .contains("__boltffiReader.readOptional(() => __boltffiReader.readI32Array())")
        );
        assert!(
            browser
                .contents()
                .contains("export function echoVecString(value: Array<string>): Array<string>")
        );
        assert!(browser.contents().contains(
            "wireArraySize(value, (__boltffiValue0) => wireStringSize(__boltffiValue0))"
        ));
        assert!(
            browser
                .contents()
                .contains("__boltffiReader.readArray(() => __boltffiReader.readString())")
        );
        assert!(browser.contents().contains(
            "export function echoVecVecI32(value: Array<Array<number> | Int32Array>): Array<Array<number> | Int32Array>"
        ));
    }

    #[test]
    fn renders_record_codecs_from_shared_field_plans() {
        let output = TypeScriptHost::new("demo")
            .expect("host constructs")
            .into_target()
            .render_partial(&record_bindings())
            .expect("target renders");
        let browser = output
            .files()
            .iter()
            .find(|file| file.path().as_path().ends_with("demo.ts"))
            .expect("browser module");

        assert!(browser.contents().contains("export interface Point"));
        assert!(browser.contents().contains("size: (value) => 24"));
        assert!(browser.contents().contains("writer.skip(7);"));
        assert!(browser.contents().contains("reader.skip(7);"));
        assert!(browser.contents().contains("export interface User"));
        assert!(browser.contents().contains(
            "size: (value) => (wireStringSize(value.name) + (4 + (value.scores.length * 4)))"
        ));
        assert!(
            browser
                .contents()
                .contains("writer.writeString(value.name);")
        );
        assert!(
            browser
                .contents()
                .contains("UserCodec.encode(__boltffi_value_writer, value);")
        );
        assert!(
            browser
                .contents()
                .contains("UserCodec.decode(__boltffiReader)")
        );
        assert!(
            browser
                .contents()
                .contains("export function echoUser(value: User): User")
        );
        assert!(browser.contents().contains("export const Status ="));
        assert!(browser.contents().contains("Inactive: -1"));
        assert!(browser.contents().contains("writer.writeI8(value);"));
        assert!(
            browser
                .contents()
                .contains("case -1: return Status.Inactive;")
        );
        assert!(
            browser
                .contents()
                .contains("export function echoStatus(value: Status): Status")
        );
        assert!(browser.contents().contains("readonly status: Status;"));
        assert!(
            browser
                .contents()
                .contains("StatusCodec.encode(writer, value.status);")
        );
        assert!(browser.contents().contains("StatusCodec.decode(reader)"));
        assert!(browser.contents().contains("export type Filter ="));
        assert!(
            browser
                .contents()
                .contains("| { readonly tag: \"ByName\"; readonly name: string }")
        );
        assert!(
            browser.contents().contains(
                "| { readonly tag: \"ByRange\"; readonly 0: number; readonly 1: number };"
            )
        );
        assert!(browser.contents().contains("case \"ByName\": return"));
        assert!(
            browser
                .contents()
                .contains("case 1: return { tag: \"ByName\", name: reader.readString() };")
        );
        assert!(
            browser
                .contents()
                .contains("export function echoFilter(value: Filter): Filter")
        );
    }
}
