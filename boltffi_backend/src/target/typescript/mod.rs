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
use render::{Function, Module};
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
        _decl: &RecordDecl<Self::Surface>,
        _bridge: &Self::Bridge,
        _context: &RenderContext<Self::Surface>,
    ) -> Result<Emitted> {
        Err(Self::unsupported("record"))
    }

    fn enumeration(
        &self,
        _decl: &EnumDecl<Self::Surface>,
        _bridge: &Self::Bridge,
        _context: &RenderContext<Self::Surface>,
    ) -> Result<Emitted> {
        Err(Self::unsupported("enum"))
    }

    fn function(
        &self,
        decl: &FunctionDecl<Self::Surface>,
        _bridge: &Self::Bridge,
        _context: &RenderContext<Self::Surface>,
    ) -> Result<Emitted> {
        Function::from_declaration(decl)?.render()
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
        _decl: &CustomTypeDecl,
        _bridge: &Self::Bridge,
        _context: &RenderContext<Self::Surface>,
    ) -> Result<Emitted> {
        Err(Self::unsupported("custom type"))
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
        assert!(browser.contents().contains(
            "__boltffi_value_writer.writeOptional(value, (__boltffiValue0) => { __boltffi_value_writer.writeI64(__boltffiValue0); });"
        ));
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
    }
}
