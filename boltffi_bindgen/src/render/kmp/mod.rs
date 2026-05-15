use std::collections::HashSet;
use std::path::{Path, PathBuf};

use crate::ir::definitions::{EnumRepr, VariantPayload};
use crate::ir::{self, AbiContract, FfiContract};
use crate::render::c::CHeaderLowerer;
use crate::render::jni::{JniEmitter, JniLowerer, JvmBindingStyle};
use crate::render::kotlin::{
    FactoryStyle, KotlinEmitter, KotlinLowerer, KotlinOptions, NamingConvention,
};

#[derive(Debug, Clone)]
pub struct KMPOptions {
    pub package_name: String,
    pub module_name: String,
    pub min_sdk: u32,
    pub kotlin_options: KotlinOptions,
    pub native_library_name: String,
    pub apple_targets: Vec<KmpAppleTarget>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KMPOutputFile {
    pub relative_path: PathBuf,
    pub contents: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KMPOutput {
    pub files: Vec<KMPOutputFile>,
}

pub struct KMPEmitter;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum KmpAppleTarget {
    IosArm64,
    IosSimulatorArm64,
    IosSimulatorX64,
    MacosArm64,
    MacosX64,
}

impl KmpAppleTarget {
    fn gradle_target_function(self) -> &'static str {
        match self {
            Self::IosArm64 => "iosArm64",
            Self::IosSimulatorArm64 => "iosSimulatorArm64",
            Self::IosSimulatorX64 => "iosX64",
            Self::MacosArm64 => "macosArm64",
            Self::MacosX64 => "macosX64",
        }
    }

    fn main_source_set(self) -> &'static str {
        match self {
            Self::IosArm64 => "iosArm64Main",
            Self::IosSimulatorArm64 => "iosSimulatorArm64Main",
            Self::IosSimulatorX64 => "iosX64Main",
            Self::MacosArm64 => "macosArm64Main",
            Self::MacosX64 => "macosX64Main",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum KmpActualBackend {
    KotlinJvm,
    KotlinNativeApple,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct KmpPlatformAdapter {
    source_set: &'static str,
    actual_file_suffix: &'static str,
    backend: KmpActualBackend,
}

impl KmpPlatformAdapter {
    const fn jvm() -> Self {
        Self {
            source_set: "jvmMain",
            actual_file_suffix: "JvmActual",
            backend: KmpActualBackend::KotlinJvm,
        }
    }

    const fn android() -> Self {
        Self {
            source_set: "androidMain",
            actual_file_suffix: "AndroidActual",
            backend: KmpActualBackend::KotlinJvm,
        }
    }

    const fn apple() -> Self {
        Self {
            source_set: "appleMain",
            actual_file_suffix: "AppleActual",
            backend: KmpActualBackend::KotlinNativeApple,
        }
    }
}

struct KmpRender {
    common: String,
    platform_actuals: Vec<KmpPlatformActual>,
}

struct KmpPlatformActual {
    adapter: KmpPlatformAdapter,
    contents: String,
}

struct KmpSurfaceSupport {
    records: HashSet<String>,
    enums: HashSet<String>,
    custom_types: HashSet<String>,
    classes: HashSet<String>,
    callbacks: HashSet<String>,
    streams: HashSet<String>,
}

struct KmpEnumVariant {
    name: String,
    payload: VariantPayload,
    doc: Option<String>,
}

struct KmpEnumField {
    name: String,
    type_expr: ir::types::TypeExpr,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum KmpConstructorSurface {
    Constructor,
    CompanionFactory,
}

impl KmpEnumField {
    fn kotlin_name(&self) -> String {
        self.name.clone()
    }
}

impl KmpSurfaceSupport {
    fn for_contract(contract: &ir::FfiContract) -> Self {
        let mut records = HashSet::new();
        let mut enums = HashSet::new();
        let mut custom_types = HashSet::new();
        let classes = contract
            .catalog
            .all_classes()
            .map(|class| class.id.as_str().to_string())
            .collect::<HashSet<_>>();

        loop {
            let before = records.len() + enums.len() + custom_types.len();

            contract.catalog.all_enums().for_each(|enumeration| {
                if enum_supported_with_sets(
                    enumeration,
                    contract,
                    &records,
                    &enums,
                    &custom_types,
                    &classes,
                    &HashSet::new(),
                ) {
                    enums.insert(enumeration.id.as_str().to_string());
                }
            });

            contract.catalog.all_records().for_each(|record| {
                if record.fields.iter().all(|field| {
                    type_supported_with_sets(
                        &field.type_expr,
                        contract,
                        &records,
                        &enums,
                        &custom_types,
                        &classes,
                        &HashSet::new(),
                    )
                }) {
                    records.insert(record.id.as_str().to_string());
                }
            });

            contract.catalog.all_custom_types().for_each(|custom| {
                if type_supported_with_sets(
                    &custom.repr,
                    contract,
                    &records,
                    &enums,
                    &custom_types,
                    &classes,
                    &HashSet::new(),
                ) {
                    custom_types.insert(custom.id.as_str().to_string());
                }
            });

            if records.len() + enums.len() + custom_types.len() == before {
                break;
            }
        }

        let callbacks = contract
            .catalog
            .all_callbacks()
            .filter(|callback| {
                callback_supported_with_sets(
                    callback,
                    contract,
                    &records,
                    &enums,
                    &custom_types,
                    &classes,
                    &HashSet::new(),
                )
            })
            .map(|callback| callback.id.as_str().to_string())
            .collect();

        let streams = contract
            .catalog
            .all_classes()
            .flat_map(|class| {
                class.streams.iter().filter_map(|stream| {
                    if stream_supported_with_sets(
                        stream,
                        contract,
                        &records,
                        &enums,
                        &custom_types,
                        &classes,
                        &callbacks,
                    ) {
                        Some(stream_key(class.id.as_str(), stream.id.as_str()))
                    } else {
                        None
                    }
                })
            })
            .collect();

        Self {
            records,
            enums,
            custom_types,
            classes,
            callbacks,
            streams,
        }
    }

    fn has_streams(&self) -> bool {
        !self.streams.is_empty()
    }
}

impl KMPEmitter {
    fn package_path(package_name: &str) -> PathBuf {
        package_name.split('.').collect()
    }

    pub fn emit(contract: &FfiContract, abi: &AbiContract, options: KMPOptions) -> KMPOutput {
        let KMPOptions {
            package_name,
            module_name,
            min_sdk,
            kotlin_options,
            native_library_name,
            apple_targets,
        } = options;
        let internal_package = format!("{package_name}.jvm");
        let factory_style = kotlin_options.factory_style;
        let common_package_path = Self::package_path(&package_name);
        let internal_package_path = Self::package_path(&internal_package);
        let apple_targets = Self::deduplicate_apple_targets(&apple_targets);
        let platform_adapters = Self::platform_adapters(!apple_targets.is_empty());

        let rendered = Self::render_surfaces(
            contract,
            &package_name,
            &internal_package,
            factory_style,
            &platform_adapters,
        );

        let kotlin_module = KotlinLowerer::new(
            contract,
            abi,
            internal_package.clone(),
            module_name.clone(),
            kotlin_options,
        )
        .lower();
        let jvm_source = KotlinEmitter::emit(&kotlin_module);

        let jni_module = JniLowerer::new(contract, abi, internal_package, module_name.clone())
            .with_jvm_binding_style(JvmBindingStyle::Kotlin)
            .lower();
        let jni_source = JniEmitter::emit(&jni_module);

        let common_dir = PathBuf::from("src/commonMain/kotlin").join(&common_package_path);

        let mut files = vec![
            KMPOutputFile {
                relative_path: PathBuf::from("settings.gradle.kts"),
                contents: Self::render_settings_gradle(&module_name),
            },
            KMPOutputFile {
                relative_path: PathBuf::from("build.gradle.kts"),
                contents: Self::render_build_gradle(&package_name, min_sdk, &apple_targets),
            },
            KMPOutputFile {
                relative_path: common_dir.join(format!("{module_name}.kt")),
                contents: rendered.common,
            },
        ];

        rendered.platform_actuals.into_iter().for_each(|actual| {
            let actual_dir =
                Self::source_set_kotlin_dir(actual.adapter.source_set, &common_package_path);
            files.push(KMPOutputFile {
                relative_path: actual_dir.join(format!(
                    "{}{}.kt",
                    module_name, actual.adapter.actual_file_suffix
                )),
                contents: actual.contents,
            });
        });

        platform_adapters
            .iter()
            .filter(|adapter| matches!(adapter.backend, KmpActualBackend::KotlinJvm))
            .for_each(|adapter| {
                let internal_dir =
                    Self::source_set_kotlin_dir(adapter.source_set, &internal_package_path);
                files.push(KMPOutputFile {
                    relative_path: internal_dir.join(format!("{module_name}.kt")),
                    contents: jvm_source.clone(),
                });
            });

        platform_adapters
            .iter()
            .filter(|adapter| matches!(adapter.backend, KmpActualBackend::KotlinJvm))
            .for_each(|adapter| {
                files.push(KMPOutputFile {
                    relative_path: PathBuf::from(format!(
                        "src/{}/c/jni_glue.c",
                        adapter.source_set
                    )),
                    contents: jni_source.clone(),
                });
            });

        if !apple_targets.is_empty() {
            let header_name = format!("{native_library_name}.h");
            files.push(KMPOutputFile {
                relative_path: PathBuf::from("src/nativeInterop/cinterop/boltffi.def"),
                contents: Self::render_native_cinterop_def(&header_name),
            });
            files.push(KMPOutputFile {
                relative_path: PathBuf::from("src/nativeInterop/cinterop/include")
                    .join(&header_name),
                contents: CHeaderLowerer::new(contract, abi).generate(),
            });
        }

        KMPOutput { files }
    }

    fn default_platform_adapters() -> Vec<KmpPlatformAdapter> {
        vec![KmpPlatformAdapter::jvm(), KmpPlatformAdapter::android()]
    }

    fn platform_adapters(include_apple: bool) -> Vec<KmpPlatformAdapter> {
        let mut adapters = Self::default_platform_adapters();
        if include_apple {
            adapters.push(KmpPlatformAdapter::apple());
        }
        adapters
    }

    fn deduplicate_apple_targets(targets: &[KmpAppleTarget]) -> Vec<KmpAppleTarget> {
        let mut seen = HashSet::new();
        targets
            .iter()
            .copied()
            .filter(|target| seen.insert(*target))
            .collect()
    }

    fn source_set_kotlin_dir(source_set: &str, package_path: &Path) -> PathBuf {
        PathBuf::from(format!("src/{source_set}/kotlin")).join(package_path)
    }

    fn render_surfaces(
        contract: &ir::FfiContract,
        package_name: &str,
        internal_package: &str,
        factory_style: FactoryStyle,
        platform_adapters: &[KmpPlatformAdapter],
    ) -> KmpRender {
        let support = KmpSurfaceSupport::for_contract(contract);
        let common = Self::render_common_surface(contract, package_name, &support, factory_style);
        let platform_actuals = platform_adapters
            .iter()
            .map(|adapter| KmpPlatformActual {
                adapter: *adapter,
                contents: Self::render_platform_actual(
                    contract,
                    package_name,
                    internal_package,
                    &support,
                    factory_style,
                    *adapter,
                ),
            })
            .collect();

        KmpRender {
            common,
            platform_actuals,
        }
    }

    fn render_common_surface(
        contract: &ir::FfiContract,
        package_name: &str,
        support: &KmpSurfaceSupport,
        factory_style: FactoryStyle,
    ) -> String {
        let mut common_sections = Vec::new();
        common_sections.push("// Auto-generated by BoltFFI. Do not edit.".to_string());
        common_sections.push(format!("package {package_name}"));
        if support.has_streams() {
            common_sections.push("import kotlinx.coroutines.flow.Flow".to_string());
        }
        common_sections.push(Self::render_common_result_runtime());

        let mut unsupported = Vec::new();

        contract
            .catalog
            .all_custom_types()
            .filter(|custom| support.custom_types.contains(custom.id.as_str()))
            .map(Self::render_custom_type)
            .for_each(|section| common_sections.push(section));

        contract
            .catalog
            .all_records()
            .filter(|record| support.records.contains(record.id.as_str()))
            .map(Self::render_common_record)
            .for_each(|section| common_sections.push(section));

        contract
            .catalog
            .all_enums()
            .filter(|enumeration| support.enums.contains(enumeration.id.as_str()))
            .map(|enumeration| Self::render_common_enum(enumeration, package_name, support))
            .for_each(|section| common_sections.push(section));

        contract
            .catalog
            .all_callbacks()
            .filter(|callback| support.callbacks.contains(callback.id.as_str()))
            .map(Self::render_common_callback)
            .for_each(|section| common_sections.push(section));

        contract
            .catalog
            .all_classes()
            .filter(|class| support.classes.contains(class.id.as_str()))
            .map(|class| Self::render_common_class(class, contract, support, factory_style))
            .for_each(|section| common_sections.push(section));

        contract.functions.iter().for_each(|function| {
            if function_supported(function, contract, support) {
                common_sections.push(Self::render_common_function(function));
            } else {
                unsupported.push(function.id.as_str().to_string());
            }
        });

        if !unsupported.is_empty() {
            common_sections.push(format!(
                "// Unsupported in the initial KMP generator slice: {}",
                unsupported.join(", ")
            ));
        }

        join_kotlin_sections(common_sections)
    }

    fn render_platform_actual(
        contract: &ir::FfiContract,
        package_name: &str,
        internal_package: &str,
        support: &KmpSurfaceSupport,
        factory_style: FactoryStyle,
        adapter: KmpPlatformAdapter,
    ) -> String {
        match adapter.backend {
            KmpActualBackend::KotlinJvm => Self::render_kotlin_jvm_actual(
                contract,
                package_name,
                internal_package,
                support,
                factory_style,
            ),
            KmpActualBackend::KotlinNativeApple => Self::render_kotlin_native_apple_actual(
                contract,
                package_name,
                support,
                factory_style,
            ),
        }
    }

    fn render_kotlin_jvm_actual(
        contract: &ir::FfiContract,
        package_name: &str,
        internal_package: &str,
        support: &KmpSurfaceSupport,
        factory_style: FactoryStyle,
    ) -> String {
        let mut actual_sections = Vec::new();
        actual_sections.push("// Auto-generated by BoltFFI. Do not edit.".to_string());
        actual_sections.push(format!("package {package_name}"));
        if support.has_streams() {
            actual_sections.push(
                "import kotlinx.coroutines.flow.Flow\nimport kotlinx.coroutines.flow.catch\nimport kotlinx.coroutines.flow.map"
                    .to_string(),
            );
        }

        contract
            .catalog
            .all_records()
            .filter(|record| support.records.contains(record.id.as_str()))
            .map(|record| {
                Self::render_record_actual_conversions(record, internal_package, contract)
            })
            .for_each(|section| actual_sections.push(section));

        contract
            .catalog
            .all_enums()
            .filter(|enumeration| support.enums.contains(enumeration.id.as_str()))
            .filter(|enumeration| !common_enum_is_c_style_value(enumeration))
            .map(|enumeration| {
                Self::render_enum_actual_conversions(enumeration, internal_package, contract)
            })
            .for_each(|section| actual_sections.push(section));

        actual_sections.push(Self::render_ffi_exception_actual_conversion(
            internal_package,
        ));

        contract
            .catalog
            .all_classes()
            .filter(|class| support.classes.contains(class.id.as_str()))
            .map(|class| {
                Self::render_kotlin_jvm_class_actual(
                    class,
                    contract,
                    support,
                    internal_package,
                    factory_style,
                )
            })
            .for_each(|section| actual_sections.push(section));

        contract
            .functions
            .iter()
            .filter(|function| function_supported(function, contract, support))
            .map(|function| {
                Self::render_kotlin_jvm_function_actual(function, contract, internal_package)
            })
            .for_each(|section| actual_sections.push(section));

        join_kotlin_sections(actual_sections)
    }

    fn render_kotlin_native_apple_actual(
        contract: &ir::FfiContract,
        package_name: &str,
        support: &KmpSurfaceSupport,
        factory_style: FactoryStyle,
    ) -> String {
        let mut actual_sections = Vec::new();
        actual_sections.push("// Auto-generated by BoltFFI. Do not edit.".to_string());
        actual_sections.push(format!("package {package_name}"));
        if support.has_streams() {
            actual_sections.push("import kotlinx.coroutines.flow.Flow".to_string());
        }

        contract
            .catalog
            .all_classes()
            .filter(|class| support.classes.contains(class.id.as_str()))
            .map(|class| {
                Self::render_kotlin_native_apple_class_stub(class, contract, support, factory_style)
            })
            .for_each(|section| actual_sections.push(section));

        contract
            .functions
            .iter()
            .filter(|function| function_supported(function, contract, support))
            .map(Self::render_kotlin_native_apple_function_stub)
            .for_each(|section| actual_sections.push(section));

        join_kotlin_sections(actual_sections)
    }

    fn render_kotlin_native_apple_class_stub(
        class: &ir::definitions::ClassDef,
        contract: &ir::FfiContract,
        support: &KmpSurfaceSupport,
        factory_style: FactoryStyle,
    ) -> String {
        let class_name = NamingConvention::class_name(class.id.as_str());
        let constructor_surfaces = kmp_constructor_surfaces(&class.constructors, factory_style);
        let mut members = Vec::new();
        let mut companion_members = Vec::new();

        class
            .constructors
            .iter()
            .zip(constructor_surfaces.iter())
            .filter(|(constructor, _)| constructor_supported(constructor, contract, support))
            .for_each(|(constructor, surface)| match surface {
                KmpConstructorSurface::Constructor => {
                    members.push(Self::render_kotlin_native_apple_constructor_stub(
                        constructor,
                    ));
                }
                KmpConstructorSurface::CompanionFactory => {
                    companion_members.push(Self::render_kotlin_native_apple_factory_stub(
                        class,
                        constructor,
                    ));
                }
            });

        members.push(Self::render_kotlin_native_apple_close_stub());

        class
            .methods
            .iter()
            .filter(|method| method_supported(method, contract, support))
            .for_each(|method| {
                let rendered = Self::render_kotlin_native_apple_method_stub(method);
                if method.receiver == ir::definitions::Receiver::Static {
                    companion_members.push(rendered);
                } else {
                    members.push(indent_lines(&rendered, "    "));
                }
            });

        class
            .streams
            .iter()
            .filter(|stream| stream_supported(class, stream, contract, support))
            .map(Self::render_kotlin_native_apple_stream_stub)
            .map(|rendered| indent_lines(&rendered, "    "))
            .for_each(|section| members.push(section));

        if !companion_members.is_empty() {
            members.push(format!(
                "    actual companion object {{\n{}\n    }}",
                companion_members
                    .iter()
                    .map(|member| indent_lines(member, "        "))
                    .collect::<Vec<_>>()
                    .join("\n\n")
            ));
        }

        format!("actual class {class_name} {{\n{}\n}}", members.join("\n\n"))
    }

    fn render_kotlin_native_apple_constructor_stub(
        constructor: &ir::definitions::ConstructorDef,
    ) -> String {
        let params = constructor.params();
        let param_list = render_param_list(&params);
        format!(
            "    actual constructor({param_list}) {{\n        {}\n    }}",
            kotlin_native_apple_scaffold_throw()
        )
    }

    fn render_kotlin_native_apple_factory_stub(
        class: &ir::definitions::ClassDef,
        constructor: &ir::definitions::ConstructorDef,
    ) -> String {
        let class_name = NamingConvention::class_name(class.id.as_str());
        let name = constructor
            .name()
            .map(|name| NamingConvention::method_name(name.as_str()))
            .unwrap_or_else(|| "new".to_string());
        let params = constructor.params();
        let param_list = render_param_list(&params);

        format!(
            "actual fun {name}({param_list}): {class_name} = {}",
            kotlin_native_apple_scaffold_throw()
        )
    }

    fn render_kotlin_native_apple_close_stub() -> String {
        format!(
            "    actual fun close() {{\n        {}\n    }}",
            kotlin_native_apple_scaffold_throw()
        )
    }

    fn render_kotlin_native_apple_method_stub(method: &ir::definitions::MethodDef) -> String {
        let method_name = NamingConvention::method_name(method.id.as_str());
        let suspend_prefix = if method.is_async() { "suspend " } else { "" };
        let params = method.params.iter().collect::<Vec<_>>();
        let param_list = render_param_list(&params);
        let return_type = return_type_name(&method.returns);
        let return_suffix = return_type
            .as_ref()
            .map(|ty| format!(": {ty}"))
            .unwrap_or_default();
        let body = kotlin_native_apple_stub_body(&method.returns);

        format!("actual {suspend_prefix}fun {method_name}({param_list}){return_suffix} {body}")
    }

    fn render_kotlin_native_apple_stream_stub(stream: &ir::definitions::StreamDef) -> String {
        let method_name = NamingConvention::method_name(stream.id.as_str());
        let item_type = common_type_name(&stream.item_type);

        format!(
            "actual fun {method_name}(): Flow<{item_type}> = {}",
            kotlin_native_apple_scaffold_throw()
        )
    }

    fn render_kotlin_native_apple_function_stub(function: &ir::definitions::FunctionDef) -> String {
        let function_name = NamingConvention::method_name(function.id.as_str());
        let suspend_prefix = if function.is_async() { "suspend " } else { "" };
        let params = Self::render_common_function_params(function);
        let return_type = return_type_name(&function.returns);
        let return_suffix = return_type
            .as_ref()
            .map(|ty| format!(": {ty}"))
            .unwrap_or_default();
        let body = kotlin_native_apple_stub_body(&function.returns);

        format!(
            "{}actual {suspend_prefix}fun {function_name}({params}){return_suffix} {body}",
            kdoc_block(&function.doc)
        )
    }

    fn render_common_result_runtime() -> String {
        r#"class FfiException(val code: Int, message: String) : Exception(message)

sealed class BoltFFIResult<out T, out E> {
    data class Ok<T>(val value: T) : BoltFFIResult<T, Nothing>()
    data class Err<E>(val error: E) : BoltFFIResult<Nothing, E>()

    val isSuccess: Boolean get() = this is Ok
    val isFailure: Boolean get() = this is Err

    fun getOrThrow(): T = when (this) {
        is Ok -> value
        is Err -> throw when (error) {
            is Throwable -> error
            else -> FfiException(-1, error.toString())
        }
    }

    fun getOrNull(): T? = when (this) {
        is Ok -> value
        is Err -> null
    }

    fun exceptionOrNull(): Throwable? = when (this) {
        is Ok -> null
        is Err -> when (error) {
            is Throwable -> error
            else -> FfiException(-1, error.toString())
        }
    }

    inline fun <R> fold(onSuccess: (T) -> R, onFailure: (E) -> R): R = when (this) {
        is Ok -> onSuccess(value)
        is Err -> onFailure(error)
    }
}"#
        .to_string()
    }

    fn render_custom_type(custom: &ir::definitions::CustomTypeDef) -> String {
        format!(
            "typealias {} = {}",
            NamingConvention::class_name(custom.id.as_str()),
            common_type_name(&custom.repr)
        )
    }

    fn render_common_record(record: &ir::definitions::RecordDef) -> String {
        if record.fields.is_empty() {
            if record.is_error {
                return format!(
                    "{}object {} : Exception(\"\")",
                    kdoc_block(&record.doc),
                    NamingConvention::class_name(record.id.as_str())
                );
            }
            return format!(
                "{}object {}",
                kdoc_block(&record.doc),
                NamingConvention::class_name(record.id.as_str())
            );
        }

        let params = record
            .fields
            .iter()
            .map(|field| {
                let name = NamingConvention::property_name(field.name.as_str());
                let prefix = if record.is_error && name == "message" {
                    "override "
                } else {
                    ""
                };
                format!(
                    "{prefix}val {}: {}",
                    name,
                    common_type_name(&field.type_expr)
                )
            })
            .collect::<Vec<_>>()
            .join(", ");

        let class_name = NamingConvention::class_name(record.id.as_str());
        let error_suffix = if record.is_error {
            let message_field = record
                .fields
                .iter()
                .find(|field| NamingConvention::property_name(field.name.as_str()) == "message")
                .map(|field| NamingConvention::property_name(field.name.as_str()))
                .unwrap_or_else(|| "\"\"".to_string());
            format!(" : Exception({message_field})")
        } else {
            String::new()
        };

        format!(
            "{}data class {class_name}({params}){error_suffix}",
            kdoc_block(&record.doc)
        )
    }

    fn render_record_actual_conversions(
        record: &ir::definitions::RecordDef,
        internal_package: &str,
        contract: &ir::FfiContract,
    ) -> String {
        let class_name = NamingConvention::class_name(record.id.as_str());
        if record.fields.is_empty() {
            return format!(
                "private fun {class_name}.toBoltFfiJvm(): {internal_package}.{class_name} = {internal_package}.{class_name}\n\nprivate fun {internal_package}.{class_name}.toBoltFfiCommon(): {class_name} = {class_name}"
            );
        }

        let to_jvm_args = record
            .fields
            .iter()
            .map(|field| {
                let name = NamingConvention::property_name(field.name.as_str());
                format!(
                    "{} = {}",
                    name,
                    to_jvm_expr(&field.type_expr, &name, contract, internal_package)
                )
            })
            .collect::<Vec<_>>()
            .join(", ");
        let to_common_args = record
            .fields
            .iter()
            .map(|field| {
                let name = NamingConvention::property_name(field.name.as_str());
                format!(
                    "{} = {}",
                    name,
                    to_common_expr(&field.type_expr, &name, contract, internal_package)
                )
            })
            .collect::<Vec<_>>()
            .join(", ");

        format!(
            "private fun {class_name}.toBoltFfiJvm(): {internal_package}.{class_name} = {internal_package}.{class_name}({to_jvm_args})\n\nprivate fun {internal_package}.{class_name}.toBoltFfiCommon(): {class_name} = {class_name}({to_common_args})"
        )
    }

    fn render_enum_actual_conversions(
        enumeration: &ir::definitions::EnumDef,
        internal_package: &str,
        contract: &ir::FfiContract,
    ) -> String {
        let class_name = NamingConvention::class_name(enumeration.id.as_str());
        let to_jvm_arms = enum_variants(enumeration)
            .iter()
            .map(|variant| {
                let variant_name = NamingConvention::class_name(variant.name.as_str());
                let fields = enum_variant_fields(&variant.payload);
                if fields.is_empty() {
                    return format!(
                        "    is {class_name}.{variant_name} -> {internal_package}.{class_name}.{variant_name}"
                    );
                }
                let args = fields
                    .iter()
                    .map(|field| {
                        let name = field.kotlin_name();
                        to_jvm_expr(&field.type_expr, &name, contract, internal_package)
                    })
                    .collect::<Vec<_>>()
                    .join(", ");
                format!(
                    "    is {class_name}.{variant_name} -> {internal_package}.{class_name}.{variant_name}({args})"
                )
            })
            .collect::<Vec<_>>()
            .join("\n");
        let to_common_arms = enum_variants(enumeration)
            .iter()
            .map(|variant| {
                let variant_name = NamingConvention::class_name(variant.name.as_str());
                let fields = enum_variant_fields(&variant.payload);
                if fields.is_empty() {
                    return format!(
                        "    is {internal_package}.{class_name}.{variant_name} -> {class_name}.{variant_name}"
                    );
                }
                let args = fields
                    .iter()
                    .map(|field| {
                        let name = field.kotlin_name();
                        to_common_expr(&field.type_expr, &name, contract, internal_package)
                    })
                    .collect::<Vec<_>>()
                    .join(", ");
                format!(
                    "    is {internal_package}.{class_name}.{variant_name} -> {class_name}.{variant_name}({args})"
                )
            })
            .collect::<Vec<_>>()
            .join("\n");

        format!(
            "private fun {class_name}.toBoltFfiJvm(): {internal_package}.{class_name} = when (this) {{\n{to_jvm_arms}\n}}\n\nprivate fun {internal_package}.{class_name}.toBoltFfiCommon(): {class_name} = when (this) {{\n{to_common_arms}\n}}"
        )
    }

    fn render_ffi_exception_actual_conversion(internal_package: &str) -> String {
        format!(
            "private fun {internal_package}.FfiException.toBoltFfiCommon(): FfiException = FfiException(code, message ?: \"\")"
        )
    }

    fn render_common_enum(
        enumeration: &ir::definitions::EnumDef,
        package_name: &str,
        support: &KmpSurfaceSupport,
    ) -> String {
        if !enumeration.is_error
            && let ir::definitions::EnumRepr::CStyle { tag_type, variants } = &enumeration.repr
        {
            return Self::render_common_c_style_enum(enumeration, *tag_type, variants);
        }

        Self::render_common_sealed_enum(enumeration, package_name, support)
    }

    fn render_common_c_style_enum(
        enumeration: &ir::definitions::EnumDef,
        tag_type: ir::types::PrimitiveType,
        variants: &[ir::definitions::CStyleVariant],
    ) -> String {
        let value_type = enum_value_type(tag_type);
        let entries = variants
            .iter()
            .map(|variant| {
                format!(
                    "{}({})",
                    NamingConvention::enum_entry_name(variant.name.as_str()),
                    enum_literal(variant.discriminant, tag_type)
                )
            })
            .collect::<Vec<_>>()
            .join(",\n    ");
        let class_name = NamingConvention::class_name(enumeration.id.as_str());

        format!(
            "{}enum class {class_name}(val value: {value_type}) {{\n    {entries};\n\n    companion object {{\n        fun fromValue(value: {value_type}): {class_name} = entries.firstOrNull {{ it.value == value }} ?: throw IllegalArgumentException(\"Unknown {class_name} value: $value\")\n    }}\n}}",
            kdoc_block(&enumeration.doc)
        )
    }

    fn render_common_sealed_enum(
        enumeration: &ir::definitions::EnumDef,
        package_name: &str,
        _support: &KmpSurfaceSupport,
    ) -> String {
        let class_name = NamingConvention::class_name(enumeration.id.as_str());
        let error_suffix = if enumeration.is_error {
            " : Exception()"
        } else {
            ""
        };
        let variant_names = enum_variants(enumeration)
            .iter()
            .map(|variant| NamingConvention::class_name(variant.name.as_str()))
            .collect::<HashSet<_>>();
        let variants = enum_variants(enumeration)
            .iter()
            .map(|variant| {
                let variant_name = NamingConvention::class_name(variant.name.as_str());
                let fields = enum_variant_fields(&variant.payload);
                let doc = kdoc_block(&variant.doc);
                if fields.is_empty() {
                    format!("{doc}    data object {variant_name} : {class_name}()")
                } else {
                    let params = fields
                        .iter()
                        .map(|field| {
                            let name = field.kotlin_name();
                            let prefix = if enumeration.is_error && name == "message" {
                                "override "
                            } else {
                                ""
                            };
                            format!(
                                "{prefix}val {}: {}",
                                name,
                                common_type_name_with_disambiguation(
                                    &field.type_expr,
                                    &variant_names,
                                    package_name
                                )
                            )
                        })
                        .collect::<Vec<_>>()
                        .join(", ");
                    format!("{doc}    data class {variant_name}({params}) : {class_name}()")
                }
            })
            .collect::<Vec<_>>()
            .join("\n\n");

        format!(
            "{}sealed class {class_name}{error_suffix} {{\n{variants}\n}}",
            kdoc_block(&enumeration.doc)
        )
    }

    fn render_common_callback(callback: &ir::definitions::CallbackTraitDef) -> String {
        let interface_name = NamingConvention::class_name(callback.id.as_str());
        let fun_prefix = if callback.kind == ir::definitions::CallbackKind::Closure {
            "fun "
        } else {
            ""
        };
        let methods = callback
            .methods
            .iter()
            .map(|method| format!("    {}", render_common_callback_method_signature(method)))
            .collect::<Vec<_>>()
            .join("\n");

        format!(
            "{}{fun_prefix}interface {interface_name} {{\n{methods}\n}}",
            kdoc_block(&callback.doc)
        )
    }

    fn render_common_class(
        class: &ir::definitions::ClassDef,
        contract: &ir::FfiContract,
        support: &KmpSurfaceSupport,
        factory_style: FactoryStyle,
    ) -> String {
        let class_name = NamingConvention::class_name(class.id.as_str());
        let constructor_surfaces = kmp_constructor_surfaces(&class.constructors, factory_style);
        let mut members = Vec::new();
        let mut companion_members = Vec::new();
        let mut unsupported = Vec::new();

        class
            .constructors
            .iter()
            .zip(constructor_surfaces.iter())
            .for_each(|(constructor, surface)| {
                if !constructor_supported(constructor, contract, support) {
                    unsupported.push(format!(
                        "constructor {}",
                        constructor
                            .name()
                            .map(|name| name.as_str().to_string())
                            .unwrap_or_else(|| "new".to_string())
                    ));
                    return;
                }

                let params = constructor.params();
                let param_list = render_param_list(&params);
                match surface {
                    KmpConstructorSurface::Constructor => {
                        members.push(format!("    constructor({param_list})"));
                    }
                    KmpConstructorSurface::CompanionFactory => {
                        let name = constructor
                            .name()
                            .map(|name| NamingConvention::method_name(name.as_str()))
                            .unwrap_or_else(|| "new".to_string());
                        companion_members
                            .push(format!("        fun {name}({param_list}): {class_name}"));
                    }
                }
            });

        members.push("    fun close()".to_string());

        class.methods.iter().for_each(|method| {
            if !method_supported(method, contract, support) {
                unsupported.push(format!("method {}", method.id.as_str()));
                return;
            }

            let declaration = render_common_method_signature(method);
            if method.receiver == ir::definitions::Receiver::Static {
                companion_members.push(format!("        {declaration}"));
            } else {
                members.push(format!("    {declaration}"));
            }
        });

        class.streams.iter().for_each(|stream| {
            if stream_supported(class, stream, contract, support) {
                members.push(format!("    {}", render_common_stream_signature(stream)));
            } else {
                unsupported.push(format!("stream {}", stream.id.as_str()));
            }
        });

        if !companion_members.is_empty() {
            members.push(format!(
                "    companion object {{\n{}\n    }}",
                companion_members.join("\n")
            ));
        }

        if !unsupported.is_empty() {
            members.push(format!(
                "    // Unsupported in this KMP class slice: {}",
                unsupported.join(", ")
            ));
        }

        format!(
            "{}expect class {class_name} {{\n{}\n}}",
            kdoc_block(&class.doc),
            members.join("\n")
        )
    }

    fn render_kotlin_jvm_class_actual(
        class: &ir::definitions::ClassDef,
        contract: &ir::FfiContract,
        support: &KmpSurfaceSupport,
        internal_package: &str,
        factory_style: FactoryStyle,
    ) -> String {
        let class_name = NamingConvention::class_name(class.id.as_str());
        let constructor_surfaces = kmp_constructor_surfaces(&class.constructors, factory_style);
        let mut members = Vec::new();
        let mut companion_members = Vec::new();

        class
            .constructors
            .iter()
            .zip(constructor_surfaces.iter())
            .for_each(|(constructor, surface)| {
                if !constructor_supported(constructor, contract, support) {
                    return;
                }

                match surface {
                    KmpConstructorSurface::Constructor => {
                        members.push(Self::render_kotlin_jvm_constructor_actual(
                            class,
                            constructor,
                            contract,
                            internal_package,
                        ));
                    }
                    KmpConstructorSurface::CompanionFactory => {
                        companion_members.push(Self::render_kotlin_jvm_factory_actual(
                            class,
                            constructor,
                            contract,
                            internal_package,
                        ));
                    }
                }
            });

        members.push(Self::render_kotlin_jvm_close_actual(internal_package));

        class
            .methods
            .iter()
            .filter(|method| method_supported(method, contract, support))
            .for_each(|method| {
                let rendered = Self::render_kotlin_jvm_method_actual(
                    method,
                    &class_name,
                    contract,
                    internal_package,
                );
                if method.receiver == ir::definitions::Receiver::Static {
                    companion_members.push(rendered);
                } else {
                    members.push(indent_lines(&rendered, "    "));
                }
            });

        class
            .streams
            .iter()
            .filter(|stream| stream_supported(class, stream, contract, support))
            .for_each(|stream| {
                let rendered =
                    Self::render_kotlin_jvm_stream_actual(stream, contract, internal_package);
                members.push(indent_lines(&rendered, "    "));
            });

        if !companion_members.is_empty() {
            members.push(format!(
                "    actual companion object {{\n{}\n    }}",
                companion_members
                    .iter()
                    .map(|member| indent_lines(member, "        "))
                    .collect::<Vec<_>>()
                    .join("\n\n")
            ));
        }

        let class_section = format!(
            "actual class {class_name} internal constructor(internal val delegate: {internal_package}.{class_name}) {{\n{}\n}}",
            members.join("\n\n")
        );

        class_section
    }

    fn render_kotlin_jvm_constructor_actual(
        class: &ir::definitions::ClassDef,
        constructor: &ir::definitions::ConstructorDef,
        contract: &ir::FfiContract,
        internal_package: &str,
    ) -> String {
        let class_name = NamingConvention::class_name(class.id.as_str());
        let params = constructor.params();
        let param_list = render_param_list(&params);
        let args = render_jvm_arg_list(&params, contract, internal_package);
        let delegated = format!("{internal_package}.{class_name}({args})");

        format!(
            "    actual constructor({param_list}) : this(\n        kotlin.run {{\n            try {{\n                {delegated}\n            }} catch (err: {internal_package}.FfiException) {{\n                throw err.toBoltFfiCommon()\n            }}\n        }}\n    )"
        )
    }

    fn render_kotlin_jvm_factory_actual(
        class: &ir::definitions::ClassDef,
        constructor: &ir::definitions::ConstructorDef,
        contract: &ir::FfiContract,
        internal_package: &str,
    ) -> String {
        let class_name = NamingConvention::class_name(class.id.as_str());
        let name = constructor
            .name()
            .map(|name| NamingConvention::method_name(name.as_str()))
            .unwrap_or_else(|| "new".to_string());
        let params = constructor.params();
        let param_list = render_param_list(&params);
        let args = render_jvm_arg_list(&params, contract, internal_package);
        let delegated = format!("{class_name}({internal_package}.{class_name}.{name}({args}))");
        let return_line = format!("        return {delegated}");
        let catch_blocks = Self::render_actual_catches(
            &ir::definitions::ReturnDef::Value(ir::types::TypeExpr::Handle(class.id.clone())),
            contract,
            internal_package,
        );

        format!(
            "actual fun {name}({param_list}): {class_name} {{\n    try {{\n{return_line}\n    }}{catch_blocks}\n}}"
        )
    }

    fn render_kotlin_jvm_close_actual(internal_package: &str) -> String {
        format!(
            "    actual fun close() {{\n        try {{\n            delegate.close()\n        }} catch (err: {internal_package}.FfiException) {{\n            throw err.toBoltFfiCommon()\n        }}\n    }}"
        )
    }

    fn render_kotlin_jvm_method_actual(
        method: &ir::definitions::MethodDef,
        class_name: &str,
        contract: &ir::FfiContract,
        internal_package: &str,
    ) -> String {
        let method_name = NamingConvention::method_name(method.id.as_str());
        let suspend_prefix = if method.is_async() { "suspend " } else { "" };
        let params = method.params.iter().collect::<Vec<_>>();
        let param_list = render_param_list(&params);
        let return_type = return_type_name(&method.returns);
        let return_suffix = return_type
            .as_ref()
            .map(|ty| format!(": {ty}"))
            .unwrap_or_default();
        let args = render_jvm_arg_list(&params, contract, internal_package);
        let delegated = if method.receiver == ir::definitions::Receiver::Static {
            format!("{internal_package}.{class_name}.{method_name}({args})")
        } else {
            format!("this.delegate.{method_name}({args})")
        };
        let actual_body = return_type_expr(&method.returns, delegated, contract, internal_package);
        let return_line = actual_return_line(&method.returns, &actual_body);
        let catch_blocks = Self::render_actual_catches(&method.returns, contract, internal_package);

        format!(
            "actual {suspend_prefix}fun {method_name}({param_list}){return_suffix} {{\n    try {{\n{return_line}\n    }}{catch_blocks}\n}}"
        )
    }

    fn render_kotlin_jvm_stream_actual(
        stream: &ir::definitions::StreamDef,
        contract: &ir::FfiContract,
        internal_package: &str,
    ) -> String {
        let method_name = NamingConvention::method_name(stream.id.as_str());
        let item_type = common_type_name(&stream.item_type);
        let item_expr = to_common_expr(
            &stream.item_type,
            "boltffiStreamItem",
            contract,
            internal_package,
        );

        format!(
            "actual fun {method_name}(): Flow<{item_type}> =\n    this.delegate.{method_name}()\n        .map {{ boltffiStreamItem -> {item_expr} }}\n        .catch {{ err ->\n            if (err is {internal_package}.FfiException) {{\n                throw err.toBoltFfiCommon()\n            }}\n            throw err\n        }}"
        )
    }

    fn render_common_function(function: &ir::definitions::FunctionDef) -> String {
        let function_name = NamingConvention::method_name(function.id.as_str());
        let suspend_prefix = if function.is_async() { "suspend " } else { "" };
        let params = Self::render_common_function_params(function);
        let return_type = return_type_name(&function.returns);
        let return_suffix = return_type
            .as_ref()
            .map(|ty| format!(": {ty}"))
            .unwrap_or_default();

        format!(
            "{}expect {suspend_prefix}fun {function_name}({params}){return_suffix}",
            kdoc_block(&function.doc)
        )
    }

    fn render_kotlin_jvm_function_actual(
        function: &ir::definitions::FunctionDef,
        contract: &ir::FfiContract,
        internal_package: &str,
    ) -> String {
        let function_name = NamingConvention::method_name(function.id.as_str());
        let suspend_prefix = if function.is_async() { "suspend " } else { "" };
        let params = Self::render_common_function_params(function);
        let return_type = return_type_name(&function.returns);
        let return_suffix = return_type
            .as_ref()
            .map(|ty| format!(": {ty}"))
            .unwrap_or_default();

        let args = function
            .params
            .iter()
            .map(|param| {
                let name = NamingConvention::param_name(param.name.as_str());
                to_jvm_expr(&param.type_expr, &name, contract, internal_package)
            })
            .collect::<Vec<_>>()
            .join(", ");
        let delegated = format!("{internal_package}.{function_name}({args})");
        let actual_body =
            return_type_expr(&function.returns, delegated, contract, internal_package);
        let return_line = actual_return_line(&function.returns, &actual_body);

        let catch_blocks =
            Self::render_actual_catches(&function.returns, contract, internal_package);
        format!(
            "{}actual {suspend_prefix}fun {function_name}({params}){return_suffix} {{\n    try {{\n{return_line}\n    }}{catch_blocks}\n}}",
            kdoc_block(&function.doc)
        )
    }

    fn render_actual_catches(
        returns: &ir::definitions::ReturnDef,
        contract: &ir::FfiContract,
        internal_package: &str,
    ) -> String {
        let typed_catch = match returns {
            ir::definitions::ReturnDef::Result { err, .. } => {
                typed_error_class_name(err, contract).map(|class_name| {
                    format!(
                        " catch (err: {internal_package}.{class_name}) {{\n        throw err.toBoltFfiCommon()\n    }}"
                    )
                })
            }
            _ => None,
        }
        .unwrap_or_default();

        format!(
            "{typed_catch} catch (err: {internal_package}.FfiException) {{\n        throw err.toBoltFfiCommon()\n    }}"
        )
    }

    fn render_common_function_params(function: &ir::definitions::FunctionDef) -> String {
        function
            .params
            .iter()
            .map(|param| {
                format!(
                    "{}: {}",
                    NamingConvention::param_name(param.name.as_str()),
                    common_type_name(&param.type_expr)
                )
            })
            .collect::<Vec<_>>()
            .join(", ")
    }

    fn render_build_gradle(
        package_name: &str,
        min_sdk: u32,
        apple_targets: &[KmpAppleTarget],
    ) -> String {
        if !apple_targets.is_empty() {
            return Self::render_build_gradle_with_apple(package_name, min_sdk, apple_targets);
        }

        format!(
            r#"import org.jetbrains.kotlin.gradle.dsl.JvmTarget

plugins {{
    kotlin("multiplatform") version "2.3.21"
    id("com.android.library") version "8.5.2"
}}

kotlin {{
    jvm {{
        compilerOptions {{
            jvmTarget.set(JvmTarget.JVM_1_8)
        }}
    }}

    androidTarget {{
        compilerOptions {{
            jvmTarget.set(JvmTarget.JVM_1_8)
        }}
    }}

    sourceSets {{
        commonMain.dependencies {{
            implementation("org.jetbrains.kotlinx:kotlinx-coroutines-core:1.11.0")
        }}
    }}
}}

android {{
    namespace = "{package_name}"
    compileSdk = 35

    defaultConfig {{
        minSdk = {min_sdk}
    }}

    compileOptions {{
        sourceCompatibility = JavaVersion.VERSION_1_8
        targetCompatibility = JavaVersion.VERSION_1_8
    }}
}}
"#
        )
    }

    fn render_build_gradle_with_apple(
        package_name: &str,
        min_sdk: u32,
        apple_targets: &[KmpAppleTarget],
    ) -> String {
        let target_entries = apple_targets
            .iter()
            .map(|target| format!("        {}(),", target.gradle_target_function()))
            .collect::<Vec<_>>()
            .join("\n");
        let apple_source_set_links = apple_targets
            .iter()
            .map(|target| {
                format!(
                    "        val {} by getting {{\n            dependsOn(appleMain)\n        }}",
                    target.main_source_set()
                )
            })
            .collect::<Vec<_>>()
            .join("\n")
            .trim()
            .to_string();

        format!(
            r#"import org.jetbrains.kotlin.gradle.dsl.JvmTarget

plugins {{
    kotlin("multiplatform") version "2.3.21"
    id("com.android.library") version "8.5.2"
}}

kotlin {{
    jvm {{
        compilerOptions {{
            jvmTarget.set(JvmTarget.JVM_1_8)
        }}
    }}

    androidTarget {{
        compilerOptions {{
            jvmTarget.set(JvmTarget.JVM_1_8)
        }}
    }}

    val boltffiAppleTargets = listOf(
{target_entries}
    )

    boltffiAppleTargets.forEach {{ target ->
        target.compilations.getByName("main") {{
            cinterops {{
                val boltffi by creating {{
                    definitionFile.set(project.file("src/nativeInterop/cinterop/boltffi.def"))
                    packageName("{package_name}.cinterop")
                    includeDirs("src/nativeInterop/cinterop/include")
                }}
            }}
        }}
    }}

    sourceSets {{
        val commonMain by getting {{
            dependencies {{
                implementation("org.jetbrains.kotlinx:kotlinx-coroutines-core:1.11.0")
            }}
        }}

        val appleMain = maybeCreate("appleMain").apply {{
            dependsOn(commonMain)
        }}

{apple_source_set_links}
    }}
}}

android {{
    namespace = "{package_name}"
    compileSdk = 35

    defaultConfig {{
        minSdk = {min_sdk}
    }}

    compileOptions {{
        sourceCompatibility = JavaVersion.VERSION_1_8
        targetCompatibility = JavaVersion.VERSION_1_8
    }}
}}
"#
        )
    }

    fn render_native_cinterop_def(header_name: &str) -> String {
        format!(
            r#"# Auto-generated by BoltFFI. Do not edit.
headers = {header_name}
headerFilter = {header_name}
"#
        )
    }

    fn render_settings_gradle(module_name: &str) -> String {
        format!(
            r#"pluginManagement {{
    repositories {{
        google()
        mavenCentral()
        gradlePluginPortal()
    }}
}}

dependencyResolutionManagement {{
    repositoriesMode.set(RepositoriesMode.FAIL_ON_PROJECT_REPOS)
    repositories {{
        google()
        mavenCentral()
    }}
}}

rootProject.name = "{}-kmp"
"#,
            module_name.to_lowercase()
        )
    }
}

fn join_kotlin_sections(sections: Vec<String>) -> String {
    let mut out = sections
        .into_iter()
        .map(|section| section.trim().to_string())
        .filter(|section| !section.is_empty())
        .collect::<Vec<_>>()
        .join("\n\n");
    out.push('\n');
    out
}

fn indent_lines(text: &str, indent: &str) -> String {
    text.lines()
        .map(|line| format!("{indent}{line}"))
        .collect::<Vec<_>>()
        .join("\n")
}

fn render_param_list(params: &[&ir::definitions::ParamDef]) -> String {
    params
        .iter()
        .map(|param| {
            format!(
                "{}: {}",
                NamingConvention::param_name(param.name.as_str()),
                common_type_name(&param.type_expr)
            )
        })
        .collect::<Vec<_>>()
        .join(", ")
}

fn render_jvm_arg_list(
    params: &[&ir::definitions::ParamDef],
    contract: &ir::FfiContract,
    internal_package: &str,
) -> String {
    params
        .iter()
        .map(|param| {
            let name = NamingConvention::param_name(param.name.as_str());
            to_jvm_expr(&param.type_expr, &name, contract, internal_package)
        })
        .collect::<Vec<_>>()
        .join(", ")
}

fn kdoc_block(doc: &Option<String>) -> String {
    doc.as_ref()
        .map(|text| {
            let mut result = "/**\n".to_string();
            text.lines()
                .for_each(|line| result.push_str(&format!(" * {line}\n")));
            result.push_str(" */\n");
            result
        })
        .unwrap_or_default()
}

fn common_enum_is_c_style_value(enumeration: &ir::definitions::EnumDef) -> bool {
    !enumeration.is_error && matches!(enumeration.repr, EnumRepr::CStyle { .. })
}

fn enum_variants(enumeration: &ir::definitions::EnumDef) -> Vec<KmpEnumVariant> {
    match &enumeration.repr {
        EnumRepr::CStyle { variants, .. } => variants
            .iter()
            .map(|variant| KmpEnumVariant {
                name: variant.name.as_str().to_string(),
                payload: VariantPayload::Unit,
                doc: variant.doc.clone(),
            })
            .collect(),
        EnumRepr::Data { variants, .. } => variants
            .iter()
            .map(|variant| KmpEnumVariant {
                name: variant.name.as_str().to_string(),
                payload: variant.payload.clone(),
                doc: variant.doc.clone(),
            })
            .collect(),
    }
}

fn enum_variant_fields(payload: &VariantPayload) -> Vec<KmpEnumField> {
    match payload {
        VariantPayload::Unit => Vec::new(),
        VariantPayload::Tuple(types) => types
            .iter()
            .enumerate()
            .map(|(index, type_expr)| KmpEnumField {
                name: NamingConvention::property_name(&format!("value{index}")),
                type_expr: type_expr.clone(),
            })
            .collect(),
        VariantPayload::Struct(fields) => fields
            .iter()
            .map(|field| KmpEnumField {
                name: NamingConvention::property_name(field.name.as_str()),
                type_expr: field.type_expr.clone(),
            })
            .collect(),
    }
}

fn render_common_method_signature(method: &ir::definitions::MethodDef) -> String {
    let method_name = NamingConvention::method_name(method.id.as_str());
    let suspend_prefix = if method.is_async() { "suspend " } else { "" };
    let params = method.params.iter().collect::<Vec<_>>();
    let param_list = render_param_list(&params);
    let return_type = return_type_name(&method.returns);
    let return_suffix = return_type
        .as_ref()
        .map(|ty| format!(": {ty}"))
        .unwrap_or_default();

    format!("{suspend_prefix}fun {method_name}({param_list}){return_suffix}")
}

fn render_common_stream_signature(stream: &ir::definitions::StreamDef) -> String {
    let method_name = NamingConvention::method_name(stream.id.as_str());
    format!(
        "fun {method_name}(): Flow<{}>",
        common_type_name(&stream.item_type)
    )
}

fn kotlin_native_apple_stub_body(returns: &ir::definitions::ReturnDef) -> String {
    match returns {
        ir::definitions::ReturnDef::Void => {
            format!("{{\n    {}\n}}", kotlin_native_apple_scaffold_throw())
        }
        ir::definitions::ReturnDef::Value(_) | ir::definitions::ReturnDef::Result { .. } => {
            format!("= {}", kotlin_native_apple_scaffold_throw())
        }
    }
}

fn kotlin_native_apple_scaffold_throw() -> &'static str {
    "throw UnsupportedOperationException(\"Kotlin/Native Apple actuals are scaffolded but not implemented yet\")"
}

fn render_common_callback_method_signature(method: &ir::definitions::CallbackMethodDef) -> String {
    let method_name = NamingConvention::method_name(method.id.as_str());
    let suspend_prefix = if method.is_async() { "suspend " } else { "" };
    let params = method.params.iter().collect::<Vec<_>>();
    let param_list = render_param_list(&params);
    let return_type = return_type_name(&method.returns);
    let return_suffix = return_type
        .as_ref()
        .map(|ty| format!(": {ty}"))
        .unwrap_or_default();

    format!("{suspend_prefix}fun {method_name}({param_list}){return_suffix}")
}

fn kmp_constructor_surfaces(
    constructors: &[ir::definitions::ConstructorDef],
    factory_style: FactoryStyle,
) -> Vec<KmpConstructorSurface> {
    let prefer_companion_methods = matches!(factory_style, FactoryStyle::CompanionMethods);
    let mut surfaces = constructors
        .iter()
        .map(|constructor| match constructor {
            ir::definitions::ConstructorDef::Default { .. } => KmpConstructorSurface::Constructor,
            ir::definitions::ConstructorDef::NamedFactory { .. } => {
                KmpConstructorSurface::CompanionFactory
            }
            ir::definitions::ConstructorDef::NamedInit { .. } if prefer_companion_methods => {
                KmpConstructorSurface::CompanionFactory
            }
            ir::definitions::ConstructorDef::NamedInit { .. } => KmpConstructorSurface::Constructor,
        })
        .collect::<Vec<_>>();

    if prefer_companion_methods {
        return surfaces;
    }

    let mut constructors_by_signature = std::collections::HashMap::<Vec<String>, Vec<usize>>::new();
    constructors
        .iter()
        .enumerate()
        .filter(|(index, _)| matches!(surfaces[*index], KmpConstructorSurface::Constructor))
        .for_each(|(index, constructor)| {
            constructors_by_signature
                .entry(
                    constructor
                        .params()
                        .iter()
                        .map(|param| common_type_name(&param.type_expr))
                        .collect(),
                )
                .or_default()
                .push(index);
        });

    constructors_by_signature
        .into_values()
        .filter(|indices| indices.len() > 1)
        .for_each(|indices| {
            let preferred_index = indices
                .iter()
                .copied()
                .min_by_key(|index| {
                    let constructor = &constructors[*index];
                    (
                        !matches!(constructor, ir::definitions::ConstructorDef::Default { .. }),
                        constructor.is_fallible(),
                        *index,
                    )
                })
                .expect("constructor collision group must be non-empty");
            indices
                .into_iter()
                .filter(|index| *index != preferred_index)
                .for_each(|index| surfaces[index] = KmpConstructorSurface::CompanionFactory);
        });

    surfaces
}

fn typed_error_class_name(ty: &ir::types::TypeExpr, contract: &ir::FfiContract) -> Option<String> {
    fn resolve(
        ty: &ir::types::TypeExpr,
        contract: &ir::FfiContract,
        visited_custom_types: &mut HashSet<String>,
    ) -> Option<String> {
        match ty {
            ir::types::TypeExpr::Enum(id) => contract
                .catalog
                .resolve_enum(id)
                .filter(|enumeration| enumeration.is_error)
                .map(|_| NamingConvention::class_name(id.as_str())),
            ir::types::TypeExpr::Record(id) => contract
                .catalog
                .resolve_record(id)
                .filter(|record| record.is_error)
                .map(|_| NamingConvention::class_name(id.as_str())),
            ir::types::TypeExpr::Custom(id) => {
                if !visited_custom_types.insert(id.as_str().to_string()) {
                    return None;
                }
                contract
                    .catalog
                    .resolve_custom(id)
                    .and_then(|custom| resolve(&custom.repr, contract, visited_custom_types))
            }
            ir::types::TypeExpr::Option(inner) => resolve(inner, contract, visited_custom_types),
            _ => None,
        }
    }

    resolve(ty, contract, &mut HashSet::new())
}

fn type_supported(
    ty: &ir::types::TypeExpr,
    contract: &ir::FfiContract,
    support: &KmpSurfaceSupport,
) -> bool {
    type_supported_with_sets(
        ty,
        contract,
        &support.records,
        &support.enums,
        &support.custom_types,
        &support.classes,
        &support.callbacks,
    )
}

fn type_supported_with_sets(
    ty: &ir::types::TypeExpr,
    contract: &ir::FfiContract,
    records: &HashSet<String>,
    enums: &HashSet<String>,
    custom_types: &HashSet<String>,
    classes: &HashSet<String>,
    callbacks: &HashSet<String>,
) -> bool {
    let _ = contract;
    match ty {
        ir::types::TypeExpr::Void
        | ir::types::TypeExpr::Primitive(_)
        | ir::types::TypeExpr::String
        | ir::types::TypeExpr::Bytes => true,
        ir::types::TypeExpr::Vec(inner) | ir::types::TypeExpr::Option(inner) => {
            type_supported_with_sets(
                inner,
                contract,
                records,
                enums,
                custom_types,
                classes,
                callbacks,
            )
        }
        ir::types::TypeExpr::Record(id) => records.contains(id.as_str()),
        ir::types::TypeExpr::Enum(id) => enums.contains(id.as_str()),
        ir::types::TypeExpr::Custom(id) => custom_types.contains(id.as_str()),
        ir::types::TypeExpr::Handle(id) => classes.contains(id.as_str()),
        ir::types::TypeExpr::Callback(id) => callbacks.contains(id.as_str()),
        ir::types::TypeExpr::Result { ok, err } => {
            type_supported_with_sets(
                ok,
                contract,
                records,
                enums,
                custom_types,
                classes,
                callbacks,
            ) && type_supported_with_sets(
                err,
                contract,
                records,
                enums,
                custom_types,
                classes,
                callbacks,
            )
        }
        ir::types::TypeExpr::Builtin(_) => false,
    }
}

fn enum_supported_with_sets(
    enumeration: &ir::definitions::EnumDef,
    contract: &ir::FfiContract,
    records: &HashSet<String>,
    enums: &HashSet<String>,
    custom_types: &HashSet<String>,
    classes: &HashSet<String>,
    callbacks: &HashSet<String>,
) -> bool {
    match &enumeration.repr {
        EnumRepr::CStyle { .. } => true,
        EnumRepr::Data { variants, .. } => variants.iter().all(|variant| {
            enum_variant_fields(&variant.payload).iter().all(|field| {
                type_supported_with_sets(
                    &field.type_expr,
                    contract,
                    records,
                    enums,
                    custom_types,
                    classes,
                    callbacks,
                )
            })
        }),
    }
}

fn return_supported(
    returns: &ir::definitions::ReturnDef,
    contract: &ir::FfiContract,
    support: &KmpSurfaceSupport,
) -> bool {
    return_supported_with_sets(
        returns,
        contract,
        &support.records,
        &support.enums,
        &support.custom_types,
        &support.classes,
        &support.callbacks,
    )
}

fn return_supported_with_sets(
    returns: &ir::definitions::ReturnDef,
    contract: &ir::FfiContract,
    records: &HashSet<String>,
    enums: &HashSet<String>,
    custom_types: &HashSet<String>,
    classes: &HashSet<String>,
    callbacks: &HashSet<String>,
) -> bool {
    match returns {
        ir::definitions::ReturnDef::Void => true,
        ir::definitions::ReturnDef::Value(ty) => {
            !type_contains_callback(ty)
                && type_supported_with_sets(
                    ty,
                    contract,
                    records,
                    enums,
                    custom_types,
                    classes,
                    callbacks,
                )
        }
        ir::definitions::ReturnDef::Result { ok, err } => {
            !type_contains_callback(ok)
                && !type_contains_callback(err)
                && type_supported_with_sets(
                    ok,
                    contract,
                    records,
                    enums,
                    custom_types,
                    classes,
                    callbacks,
                )
                && type_supported_with_sets(
                    err,
                    contract,
                    records,
                    enums,
                    custom_types,
                    classes,
                    callbacks,
                )
        }
    }
}

fn callback_supported_with_sets(
    callback: &ir::definitions::CallbackTraitDef,
    contract: &ir::FfiContract,
    records: &HashSet<String>,
    enums: &HashSet<String>,
    custom_types: &HashSet<String>,
    classes: &HashSet<String>,
    callbacks: &HashSet<String>,
) -> bool {
    !callback.methods.is_empty()
        && callback.methods.iter().all(|method| {
            !method.is_async()
                && return_supported_with_sets(
                    &method.returns,
                    contract,
                    records,
                    enums,
                    custom_types,
                    classes,
                    callbacks,
                )
                && method.params.iter().all(|param| {
                    type_supported_with_sets(
                        &param.type_expr,
                        contract,
                        records,
                        enums,
                        custom_types,
                        classes,
                        callbacks,
                    )
                })
        })
}

fn type_contains_callback(ty: &ir::types::TypeExpr) -> bool {
    match ty {
        ir::types::TypeExpr::Callback(_) => true,
        ir::types::TypeExpr::Vec(inner) | ir::types::TypeExpr::Option(inner) => {
            type_contains_callback(inner)
        }
        ir::types::TypeExpr::Result { ok, err } => {
            type_contains_callback(ok) || type_contains_callback(err)
        }
        _ => false,
    }
}

fn function_supported(
    function: &ir::definitions::FunctionDef,
    contract: &ir::FfiContract,
    support: &KmpSurfaceSupport,
) -> bool {
    return_supported(&function.returns, contract, support)
        && function
            .params
            .iter()
            .all(|param| type_supported(&param.type_expr, contract, support))
}

fn constructor_supported(
    constructor: &ir::definitions::ConstructorDef,
    contract: &ir::FfiContract,
    support: &KmpSurfaceSupport,
) -> bool {
    !constructor.is_optional()
        && constructor
            .params()
            .iter()
            .all(|param| type_supported(&param.type_expr, contract, support))
}

fn method_supported(
    method: &ir::definitions::MethodDef,
    contract: &ir::FfiContract,
    support: &KmpSurfaceSupport,
) -> bool {
    method.receiver != ir::definitions::Receiver::OwnedSelf
        && return_supported(&method.returns, contract, support)
        && method
            .params
            .iter()
            .all(|param| type_supported(&param.type_expr, contract, support))
}

fn stream_supported(
    class: &ir::definitions::ClassDef,
    stream: &ir::definitions::StreamDef,
    _contract: &ir::FfiContract,
    support: &KmpSurfaceSupport,
) -> bool {
    support
        .streams
        .contains(&stream_key(class.id.as_str(), stream.id.as_str()))
}

fn stream_supported_with_sets(
    stream: &ir::definitions::StreamDef,
    contract: &ir::FfiContract,
    records: &HashSet<String>,
    enums: &HashSet<String>,
    custom_types: &HashSet<String>,
    classes: &HashSet<String>,
    callbacks: &HashSet<String>,
) -> bool {
    stream.mode == ir::definitions::StreamMode::Async
        && !type_contains_handle_or_callback(&stream.item_type, contract)
        && type_supported_with_sets(
            &stream.item_type,
            contract,
            records,
            enums,
            custom_types,
            classes,
            callbacks,
        )
}

fn stream_key(class_id: &str, stream_id: &str) -> String {
    format!("{class_id}::{stream_id}")
}

fn type_contains_handle_or_callback(ty: &ir::types::TypeExpr, contract: &ir::FfiContract) -> bool {
    fn resolve(
        ty: &ir::types::TypeExpr,
        contract: &ir::FfiContract,
        visited_custom_types: &mut HashSet<String>,
    ) -> bool {
        match ty {
            ir::types::TypeExpr::Handle(_) | ir::types::TypeExpr::Callback(_) => true,
            ir::types::TypeExpr::Vec(inner) | ir::types::TypeExpr::Option(inner) => {
                resolve(inner, contract, visited_custom_types)
            }
            ir::types::TypeExpr::Result { ok, err } => {
                resolve(ok, contract, visited_custom_types)
                    || resolve(err, contract, visited_custom_types)
            }
            ir::types::TypeExpr::Custom(id) => {
                if !visited_custom_types.insert(id.as_str().to_string()) {
                    return false;
                }
                contract
                    .catalog
                    .resolve_custom(id)
                    .map(|custom| resolve(&custom.repr, contract, visited_custom_types))
                    .unwrap_or(false)
            }
            _ => false,
        }
    }

    resolve(ty, contract, &mut HashSet::new())
}

fn common_type_name(ty: &ir::types::TypeExpr) -> String {
    match ty {
        ir::types::TypeExpr::Void => "Unit".to_string(),
        ir::types::TypeExpr::Primitive(primitive) => primitive_type_name(*primitive),
        ir::types::TypeExpr::String => "String".to_string(),
        ir::types::TypeExpr::Bytes => "ByteArray".to_string(),
        ir::types::TypeExpr::Vec(inner) => vec_type_name(inner),
        ir::types::TypeExpr::Option(inner) => format!("{}?", common_type_name(inner)),
        ir::types::TypeExpr::Record(id) => NamingConvention::class_name(id.as_str()),
        ir::types::TypeExpr::Enum(id) => NamingConvention::class_name(id.as_str()),
        ir::types::TypeExpr::Custom(id) => NamingConvention::class_name(id.as_str()),
        ir::types::TypeExpr::Builtin(id) => NamingConvention::class_name(id.as_str()),
        ir::types::TypeExpr::Handle(id) => NamingConvention::class_name(id.as_str()),
        ir::types::TypeExpr::Callback(id) => NamingConvention::class_name(id.as_str()),
        ir::types::TypeExpr::Result { ok, err } => {
            format!(
                "BoltFFIResult<{}, {}>",
                common_type_name(ok),
                common_type_name(err)
            )
        }
    }
}

fn common_type_name_with_disambiguation(
    ty: &ir::types::TypeExpr,
    reserved_names: &HashSet<String>,
    package_name: &str,
) -> String {
    match ty {
        ir::types::TypeExpr::Void => {
            disambiguated_kotlin_type_name("Unit", reserved_names, "kotlin")
        }
        ir::types::TypeExpr::Primitive(primitive) => {
            let name = primitive_type_name(*primitive);
            disambiguated_kotlin_type_name(&name, reserved_names, "kotlin")
        }
        ir::types::TypeExpr::String => {
            disambiguated_kotlin_type_name("String", reserved_names, "kotlin")
        }
        ir::types::TypeExpr::Bytes => {
            disambiguated_kotlin_type_name("ByteArray", reserved_names, "kotlin")
        }
        ir::types::TypeExpr::Vec(inner) => match inner.as_ref() {
            ir::types::TypeExpr::Primitive(_) => {
                let name = vec_type_name(inner);
                disambiguated_kotlin_type_name(&name, reserved_names, "kotlin")
            }
            _ => {
                let list_name =
                    disambiguated_kotlin_type_name("List", reserved_names, "kotlin.collections");
                format!(
                    "{list_name}<{}>",
                    common_type_name_with_disambiguation(inner, reserved_names, package_name)
                )
            }
        },
        ir::types::TypeExpr::Option(inner) => format!(
            "{}?",
            common_type_name_with_disambiguation(inner, reserved_names, package_name)
        ),
        ir::types::TypeExpr::Result { ok, err } => format!(
            "{}<{}, {}>",
            disambiguated_kotlin_type_name("BoltFFIResult", reserved_names, package_name),
            common_type_name_with_disambiguation(ok, reserved_names, package_name),
            common_type_name_with_disambiguation(err, reserved_names, package_name)
        ),
        ir::types::TypeExpr::Record(id) => {
            disambiguated_class_name(id.as_str(), reserved_names, package_name)
        }
        ir::types::TypeExpr::Enum(id) => {
            disambiguated_class_name(id.as_str(), reserved_names, package_name)
        }
        ir::types::TypeExpr::Custom(id) => {
            disambiguated_class_name(id.as_str(), reserved_names, package_name)
        }
        ir::types::TypeExpr::Builtin(id) => {
            disambiguated_class_name(id.as_str(), reserved_names, package_name)
        }
        ir::types::TypeExpr::Handle(id) => {
            disambiguated_class_name(id.as_str(), reserved_names, package_name)
        }
        ir::types::TypeExpr::Callback(id) => {
            disambiguated_class_name(id.as_str(), reserved_names, package_name)
        }
    }
}

fn disambiguated_kotlin_type_name(
    name: &str,
    reserved_names: &HashSet<String>,
    package_name: &str,
) -> String {
    if reserved_names.contains(name) {
        format!("{package_name}.{name}")
    } else {
        name.to_string()
    }
}

fn disambiguated_class_name(
    id: &str,
    reserved_names: &HashSet<String>,
    package_name: &str,
) -> String {
    let class_name = NamingConvention::class_name(id);
    if reserved_names.contains(&class_name) {
        format!("{package_name}.{class_name}")
    } else {
        class_name
    }
}

fn vec_type_name(inner: &ir::types::TypeExpr) -> String {
    match inner {
        ir::types::TypeExpr::Primitive(primitive) => match primitive {
            ir::types::PrimitiveType::I32 | ir::types::PrimitiveType::U32 => "IntArray".to_string(),
            ir::types::PrimitiveType::I16 | ir::types::PrimitiveType::U16 => {
                "ShortArray".to_string()
            }
            ir::types::PrimitiveType::I64
            | ir::types::PrimitiveType::U64
            | ir::types::PrimitiveType::ISize
            | ir::types::PrimitiveType::USize => "LongArray".to_string(),
            ir::types::PrimitiveType::F32 => "FloatArray".to_string(),
            ir::types::PrimitiveType::F64 => "DoubleArray".to_string(),
            ir::types::PrimitiveType::U8 | ir::types::PrimitiveType::I8 => "ByteArray".to_string(),
            ir::types::PrimitiveType::Bool => "BooleanArray".to_string(),
        },
        _ => format!("List<{}>", common_type_name(inner)),
    }
}

fn primitive_type_name(primitive: ir::types::PrimitiveType) -> String {
    match primitive {
        ir::types::PrimitiveType::Bool => "Boolean".to_string(),
        ir::types::PrimitiveType::I8 => "Byte".to_string(),
        ir::types::PrimitiveType::U8 => "UByte".to_string(),
        ir::types::PrimitiveType::I16 => "Short".to_string(),
        ir::types::PrimitiveType::U16 => "UShort".to_string(),
        ir::types::PrimitiveType::I32 => "Int".to_string(),
        ir::types::PrimitiveType::U32 => "UInt".to_string(),
        ir::types::PrimitiveType::I64 | ir::types::PrimitiveType::ISize => "Long".to_string(),
        ir::types::PrimitiveType::U64 | ir::types::PrimitiveType::USize => "ULong".to_string(),
        ir::types::PrimitiveType::F32 => "Float".to_string(),
        ir::types::PrimitiveType::F64 => "Double".to_string(),
    }
}

fn enum_value_type(primitive: ir::types::PrimitiveType) -> String {
    match primitive {
        ir::types::PrimitiveType::I8 | ir::types::PrimitiveType::U8 => "Byte".to_string(),
        ir::types::PrimitiveType::I16 | ir::types::PrimitiveType::U16 => "Short".to_string(),
        ir::types::PrimitiveType::I32 | ir::types::PrimitiveType::U32 => "Int".to_string(),
        ir::types::PrimitiveType::I64
        | ir::types::PrimitiveType::U64
        | ir::types::PrimitiveType::ISize
        | ir::types::PrimitiveType::USize => "Long".to_string(),
        _ => primitive_type_name(primitive),
    }
}

fn enum_literal(value: i128, primitive: ir::types::PrimitiveType) -> String {
    kotlin_integer_literal(value, &enum_value_type(primitive))
}

fn jvm_type_name(ty: &ir::types::TypeExpr, internal_package: &str) -> String {
    match ty {
        ir::types::TypeExpr::Void => "Unit".to_string(),
        ir::types::TypeExpr::Primitive(primitive) => primitive_type_name(*primitive),
        ir::types::TypeExpr::String => "String".to_string(),
        ir::types::TypeExpr::Bytes => "ByteArray".to_string(),
        ir::types::TypeExpr::Vec(inner) => match inner.as_ref() {
            ir::types::TypeExpr::Primitive(_) => vec_type_name(inner),
            _ => format!("List<{}>", jvm_type_name(inner, internal_package)),
        },
        ir::types::TypeExpr::Option(inner) => {
            format!("{}?", jvm_type_name(inner, internal_package))
        }
        ir::types::TypeExpr::Record(id) => format!(
            "{internal_package}.{}",
            NamingConvention::class_name(id.as_str())
        ),
        ir::types::TypeExpr::Enum(id) => format!(
            "{internal_package}.{}",
            NamingConvention::class_name(id.as_str())
        ),
        ir::types::TypeExpr::Custom(id) => format!(
            "{internal_package}.{}",
            NamingConvention::class_name(id.as_str())
        ),
        ir::types::TypeExpr::Builtin(id) => format!(
            "{internal_package}.{}",
            NamingConvention::class_name(id.as_str())
        ),
        ir::types::TypeExpr::Handle(id) => format!(
            "{internal_package}.{}",
            NamingConvention::class_name(id.as_str())
        ),
        ir::types::TypeExpr::Callback(id) => format!(
            "{internal_package}.{}",
            NamingConvention::class_name(id.as_str())
        ),
        ir::types::TypeExpr::Result { ok, err } => {
            format!(
                "{internal_package}.BoltFFIResult<{}, {}>",
                jvm_type_name(ok, internal_package),
                jvm_type_name(err, internal_package)
            )
        }
    }
}

fn kotlin_integer_literal(value: i128, kotlin_type: &str) -> String {
    match kotlin_type {
        "Byte" => format!("({value}L).toByte()"),
        "Short" => format!("({value}L).toShort()"),
        "Int" => {
            if i32::try_from(value).is_ok() {
                value.to_string()
            } else {
                format!("({value}L).toInt()")
            }
        }
        "Long" => {
            if i64::try_from(value).is_ok() {
                format!("{value}L")
            } else {
                format!("({value}uL).toLong()")
            }
        }
        _ => value.to_string(),
    }
}

fn return_type_name(returns: &ir::definitions::ReturnDef) -> Option<String> {
    match returns {
        ir::definitions::ReturnDef::Void => None,
        ir::definitions::ReturnDef::Value(ty) => Some(common_type_name(ty)),
        ir::definitions::ReturnDef::Result { ok, .. } => Some(common_type_name(ok)),
    }
}

fn jvm_return_type_name(
    returns: &ir::definitions::ReturnDef,
    internal_package: &str,
) -> Option<String> {
    match returns {
        ir::definitions::ReturnDef::Void => None,
        ir::definitions::ReturnDef::Value(ty) => Some(jvm_type_name(ty, internal_package)),
        ir::definitions::ReturnDef::Result { ok, .. } => Some(jvm_type_name(ok, internal_package)),
    }
}

fn render_kotlin_jvm_callback_adapter(
    callback_id: &ir::ids::CallbackId,
    expr: &str,
    contract: &ir::FfiContract,
    internal_package: &str,
) -> String {
    let callback = contract
        .catalog
        .resolve_callback(callback_id)
        .expect("supported callback must resolve");
    let interface_name = NamingConvention::class_name(callback.id.as_str());
    let methods = callback
        .methods
        .iter()
        .map(|method| render_kotlin_jvm_callback_adapter_method(method, contract, internal_package))
        .map(|method| indent_lines(&method, "        "))
        .collect::<Vec<_>>()
        .join("\n\n");

    format!(
        "kotlin.run {{\n    val boltffiCommonCallback = {expr}\n    object : {internal_package}.{interface_name} {{\n{methods}\n    }}\n}}"
    )
}

fn render_kotlin_jvm_callback_adapter_method(
    method: &ir::definitions::CallbackMethodDef,
    contract: &ir::FfiContract,
    internal_package: &str,
) -> String {
    let method_name = NamingConvention::method_name(method.id.as_str());
    let params = method.params.iter().collect::<Vec<_>>();
    let param_list = params
        .iter()
        .map(|param| {
            format!(
                "{}: {}",
                NamingConvention::param_name(param.name.as_str()),
                jvm_type_name(&param.type_expr, internal_package)
            )
        })
        .collect::<Vec<_>>()
        .join(", ");
    let return_type = jvm_return_type_name(&method.returns, internal_package);
    let return_suffix = return_type
        .as_ref()
        .map(|ty| format!(": {ty}"))
        .unwrap_or_default();
    let args = params
        .iter()
        .map(|param| {
            let name = NamingConvention::param_name(param.name.as_str());
            to_common_expr(&param.type_expr, &name, contract, internal_package)
        })
        .collect::<Vec<_>>()
        .join(", ");
    let common_call = format!("boltffiCommonCallback.{method_name}({args})");
    let body = match &method.returns {
        ir::definitions::ReturnDef::Void => format!("            {common_call}"),
        ir::definitions::ReturnDef::Value(ty) => {
            let result = to_jvm_expr(ty, "boltffiCallbackResult", contract, internal_package);
            format!(
                "            val boltffiCallbackResult = {common_call}\n            return {result}"
            )
        }
        ir::definitions::ReturnDef::Result { ok, .. } => {
            let result = to_jvm_expr(ok, "boltffiCallbackResult", contract, internal_package);
            format!(
                "            val boltffiCallbackResult = {common_call}\n            return {result}"
            )
        }
    };
    let catches =
        render_kotlin_jvm_callback_adapter_catches(&method.returns, contract, internal_package);

    format!(
        "override fun {method_name}({param_list}){return_suffix} {{\n        try {{\n{body}\n        }}{catches}\n    }}"
    )
}

fn render_kotlin_jvm_callback_adapter_catches(
    returns: &ir::definitions::ReturnDef,
    contract: &ir::FfiContract,
    internal_package: &str,
) -> String {
    let typed_catch = match returns {
        ir::definitions::ReturnDef::Result { err, .. } => {
            typed_error_class_name(err, contract).map(|class_name| {
                format!(
                    " catch (err: {class_name}) {{\n            throw err.toBoltFfiJvm()\n        }}"
                )
            })
        }
        _ => None,
    }
    .unwrap_or_default();

    format!(
        "{typed_catch} catch (err: FfiException) {{\n            throw {internal_package}.FfiException(err.code, err.message ?: \"\")\n        }}"
    )
}

fn to_jvm_expr(
    ty: &ir::types::TypeExpr,
    expr: &str,
    contract: &ir::FfiContract,
    internal_package: &str,
) -> String {
    match ty {
        ir::types::TypeExpr::Primitive(_) => expr.to_string(),
        ir::types::TypeExpr::Record(_) => format!("{expr}.toBoltFfiJvm()"),
        ir::types::TypeExpr::Handle(_) => format!("{expr}.delegate"),
        ir::types::TypeExpr::Callback(id) => {
            render_kotlin_jvm_callback_adapter(id, expr, contract, internal_package)
        }
        ir::types::TypeExpr::Enum(id) => {
            let class_name = NamingConvention::class_name(id.as_str());
            if contract
                .catalog
                .resolve_enum(id)
                .map(common_enum_is_c_style_value)
                .unwrap_or(false)
            {
                format!("{internal_package}.{class_name}.fromValue({expr}.value)")
            } else {
                format!("{expr}.toBoltFfiJvm()")
            }
        }
        ir::types::TypeExpr::Vec(inner) => to_jvm_vec_expr(inner, expr, contract, internal_package),
        ir::types::TypeExpr::Option(inner) => {
            format!(
                "{expr}?.let {{ {} }}",
                to_jvm_expr(inner, "it", contract, internal_package)
            )
        }
        ir::types::TypeExpr::Custom(id) => contract
            .catalog
            .resolve_custom(id)
            .map(|custom| to_jvm_expr(&custom.repr, expr, contract, internal_package))
            .unwrap_or_else(|| expr.to_string()),
        ir::types::TypeExpr::Result { ok, err } => {
            let ok_expr = to_jvm_expr(ok, "boltffiResult.value", contract, internal_package);
            let err_expr = to_jvm_expr(err, "boltffiResult.error", contract, internal_package);
            format!(
                "when (val boltffiResult = {expr}) {{ is BoltFFIResult.Ok -> {internal_package}.BoltFFIResult.Ok({ok_expr}); is BoltFFIResult.Err -> {internal_package}.BoltFFIResult.Err({err_expr}) }}"
            )
        }
        _ => expr.to_string(),
    }
}

fn to_jvm_vec_expr(
    ty: &ir::types::TypeExpr,
    expr: &str,
    contract: &ir::FfiContract,
    internal_package: &str,
) -> String {
    match ty {
        ir::types::TypeExpr::Primitive(_) => expr.to_string(),
        _ => format!(
            "{expr}.map {{ {} }}",
            to_jvm_expr(ty, "it", contract, internal_package)
        ),
    }
}

fn to_common_expr(
    ty: &ir::types::TypeExpr,
    expr: &str,
    contract: &ir::FfiContract,
    internal_package: &str,
) -> String {
    match ty {
        ir::types::TypeExpr::Primitive(_) => expr.to_string(),
        ir::types::TypeExpr::Record(_) => format!("{expr}.toBoltFfiCommon()"),
        ir::types::TypeExpr::Handle(id) => {
            let class_name = NamingConvention::class_name(id.as_str());
            format!("{class_name}({expr})")
        }
        ir::types::TypeExpr::Enum(id) => {
            let class_name = NamingConvention::class_name(id.as_str());
            if contract
                .catalog
                .resolve_enum(id)
                .map(common_enum_is_c_style_value)
                .unwrap_or(false)
            {
                format!("{class_name}.fromValue({expr}.value)")
            } else {
                format!("{expr}.toBoltFfiCommon()")
            }
        }
        ir::types::TypeExpr::Vec(inner) => {
            to_common_vec_expr(inner, expr, contract, internal_package)
        }
        ir::types::TypeExpr::Option(inner) => {
            format!(
                "{expr}?.let {{ {} }}",
                to_common_expr(inner, "it", contract, internal_package)
            )
        }
        ir::types::TypeExpr::Custom(id) => contract
            .catalog
            .resolve_custom(id)
            .map(|custom| to_common_expr(&custom.repr, expr, contract, internal_package))
            .unwrap_or_else(|| expr.to_string()),
        ir::types::TypeExpr::Result { ok, err } => {
            let ok_expr = to_common_expr(ok, "boltffiResult.value", contract, internal_package);
            let err_expr = to_common_expr(err, "boltffiResult.error", contract, internal_package);
            format!(
                "when (val boltffiResult = {expr}) {{ is {internal_package}.BoltFFIResult.Ok -> BoltFFIResult.Ok({ok_expr}); is {internal_package}.BoltFFIResult.Err -> BoltFFIResult.Err({err_expr}) }}"
            )
        }
        _ => expr.to_string(),
    }
}

fn to_common_vec_expr(
    ty: &ir::types::TypeExpr,
    expr: &str,
    contract: &ir::FfiContract,
    internal_package: &str,
) -> String {
    match ty {
        ir::types::TypeExpr::Primitive(_) => expr.to_string(),
        _ => format!(
            "{expr}.map {{ {} }}",
            to_common_expr(ty, "it", contract, internal_package)
        ),
    }
}

fn actual_return_line(returns: &ir::definitions::ReturnDef, actual_body: &str) -> String {
    match returns {
        ir::definitions::ReturnDef::Void => format!("        {actual_body}"),
        ir::definitions::ReturnDef::Value(_) | ir::definitions::ReturnDef::Result { .. } => {
            format!("        return {actual_body}")
        }
    }
}

fn return_type_expr(
    returns: &ir::definitions::ReturnDef,
    delegated: String,
    contract: &ir::FfiContract,
    internal_package: &str,
) -> String {
    match returns {
        ir::definitions::ReturnDef::Void => delegated,
        ir::definitions::ReturnDef::Value(ty) => {
            to_common_expr(ty, &delegated, contract, internal_package)
        }
        ir::definitions::ReturnDef::Result { ok, .. } => {
            to_common_expr(ok, &delegated, contract, internal_package)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn empty_record() -> ir::definitions::RecordDef {
        ir::definitions::RecordDef {
            id: "Empty".into(),
            is_repr_c: false,
            is_error: false,
            fields: Vec::new(),
            constructors: Vec::new(),
            methods: Vec::new(),
            doc: None,
            deprecated: None,
        }
    }

    fn field(name: &str, type_expr: ir::types::TypeExpr) -> ir::definitions::FieldDef {
        ir::definitions::FieldDef {
            name: name.into(),
            type_expr,
            doc: None,
            default: None,
        }
    }

    fn param(name: &str, type_expr: ir::types::TypeExpr) -> ir::definitions::ParamDef {
        ir::definitions::ParamDef {
            name: name.into(),
            type_expr,
            passing: ir::definitions::ParamPassing::Value,
            doc: None,
        }
    }

    fn callback_param(name: &str, callback_id: &str) -> ir::definitions::ParamDef {
        ir::definitions::ParamDef {
            name: name.into(),
            type_expr: ir::types::TypeExpr::Callback(callback_id.into()),
            passing: ir::definitions::ParamPassing::ImplTrait,
            doc: None,
        }
    }

    fn error_record(id: &str) -> ir::definitions::RecordDef {
        ir::definitions::RecordDef {
            id: id.into(),
            is_repr_c: false,
            is_error: true,
            fields: vec![field("message", ir::types::TypeExpr::String)],
            constructors: Vec::new(),
            methods: Vec::new(),
            doc: None,
            deprecated: None,
        }
    }

    fn counter_class() -> ir::definitions::ClassDef {
        ir::definitions::ClassDef {
            id: "Counter".into(),
            constructors: vec![
                ir::definitions::ConstructorDef::Default {
                    params: vec![param(
                        "initial",
                        ir::types::TypeExpr::Primitive(ir::types::PrimitiveType::I32),
                    )],
                    is_fallible: false,
                    is_optional: false,
                    doc: None,
                    deprecated: None,
                },
                ir::definitions::ConstructorDef::NamedFactory {
                    name: "zero".into(),
                    is_fallible: false,
                    is_optional: false,
                    doc: None,
                    deprecated: None,
                },
            ],
            methods: vec![
                ir::definitions::MethodDef {
                    id: "get".into(),
                    receiver: ir::definitions::Receiver::RefSelf,
                    params: Vec::new(),
                    returns: ir::definitions::ReturnDef::Value(ir::types::TypeExpr::Primitive(
                        ir::types::PrimitiveType::I32,
                    )),
                    execution_kind: boltffi_ffi_rules::callable::ExecutionKind::Sync,
                    doc: None,
                    deprecated: None,
                },
                ir::definitions::MethodDef {
                    id: "reset".into(),
                    receiver: ir::definitions::Receiver::RefMutSelf,
                    params: Vec::new(),
                    returns: ir::definitions::ReturnDef::Void,
                    execution_kind: boltffi_ffi_rules::callable::ExecutionKind::Sync,
                    doc: None,
                    deprecated: None,
                },
                ir::definitions::MethodDef {
                    id: "identity".into(),
                    receiver: ir::definitions::Receiver::Static,
                    params: vec![param(
                        "counter",
                        ir::types::TypeExpr::Handle("Counter".into()),
                    )],
                    returns: ir::definitions::ReturnDef::Value(ir::types::TypeExpr::Option(
                        Box::new(ir::types::TypeExpr::Handle("Counter".into())),
                    )),
                    execution_kind: boltffi_ffi_rules::callable::ExecutionKind::Sync,
                    doc: None,
                    deprecated: None,
                },
            ],
            streams: Vec::new(),
            doc: None,
            deprecated: None,
        }
    }

    fn optional_counter_class() -> ir::definitions::ClassDef {
        ir::definitions::ClassDef {
            id: "OptionalCounter".into(),
            constructors: vec![
                ir::definitions::ConstructorDef::Default {
                    params: vec![param(
                        "initial",
                        ir::types::TypeExpr::Primitive(ir::types::PrimitiveType::I32),
                    )],
                    is_fallible: false,
                    is_optional: true,
                    doc: None,
                    deprecated: None,
                },
                ir::definitions::ConstructorDef::NamedFactory {
                    name: "maybe".into(),
                    is_fallible: false,
                    is_optional: true,
                    doc: None,
                    deprecated: None,
                },
            ],
            methods: Vec::new(),
            streams: Vec::new(),
            doc: None,
            deprecated: None,
        }
    }

    fn delegate_param_class() -> ir::definitions::ClassDef {
        ir::definitions::ClassDef {
            id: "DelegateParam".into(),
            constructors: Vec::new(),
            methods: vec![ir::definitions::MethodDef {
                id: "accept_delegate".into(),
                receiver: ir::definitions::Receiver::RefSelf,
                params: vec![param(
                    "delegate",
                    ir::types::TypeExpr::Primitive(ir::types::PrimitiveType::I32),
                )],
                returns: ir::definitions::ReturnDef::Void,
                execution_kind: boltffi_ffi_rules::callable::ExecutionKind::Sync,
                doc: None,
                deprecated: None,
            }],
            streams: Vec::new(),
            doc: None,
            deprecated: None,
        }
    }

    fn point_record() -> ir::definitions::RecordDef {
        ir::definitions::RecordDef {
            id: "Point".into(),
            is_repr_c: true,
            is_error: false,
            fields: vec![
                field(
                    "x",
                    ir::types::TypeExpr::Primitive(ir::types::PrimitiveType::I32),
                ),
                field(
                    "y",
                    ir::types::TypeExpr::Primitive(ir::types::PrimitiveType::I32),
                ),
            ],
            constructors: Vec::new(),
            methods: Vec::new(),
            doc: None,
            deprecated: None,
        }
    }

    fn stream_def(
        name: &str,
        item_type: ir::types::TypeExpr,
        mode: ir::definitions::StreamMode,
    ) -> ir::definitions::StreamDef {
        ir::definitions::StreamDef {
            id: name.into(),
            item_type,
            mode,
            doc: None,
            deprecated: None,
        }
    }

    fn event_bus_class() -> ir::definitions::ClassDef {
        ir::definitions::ClassDef {
            id: "EventBus".into(),
            constructors: Vec::new(),
            methods: Vec::new(),
            streams: vec![
                stream_def(
                    "subscribe_values",
                    ir::types::TypeExpr::Primitive(ir::types::PrimitiveType::I32),
                    ir::definitions::StreamMode::Async,
                ),
                stream_def(
                    "subscribe_points",
                    ir::types::TypeExpr::Record("Point".into()),
                    ir::definitions::StreamMode::Async,
                ),
                stream_def(
                    "subscribe_values_batch",
                    ir::types::TypeExpr::Primitive(ir::types::PrimitiveType::I32),
                    ir::definitions::StreamMode::Batch,
                ),
                stream_def(
                    "subscribe_values_callback",
                    ir::types::TypeExpr::Primitive(ir::types::PrimitiveType::I32),
                    ir::definitions::StreamMode::Callback,
                ),
            ],
            doc: None,
            deprecated: None,
        }
    }

    fn value_callback() -> ir::definitions::CallbackTraitDef {
        ir::definitions::CallbackTraitDef {
            id: "ValueCallback".into(),
            methods: vec![ir::definitions::CallbackMethodDef {
                id: "on_value".into(),
                params: vec![param(
                    "value",
                    ir::types::TypeExpr::Primitive(ir::types::PrimitiveType::I32),
                )],
                returns: ir::definitions::ReturnDef::Value(ir::types::TypeExpr::Primitive(
                    ir::types::PrimitiveType::I32,
                )),
                execution_kind: boltffi_ffi_rules::callable::ExecutionKind::Sync,
                doc: None,
            }],
            kind: ir::definitions::CallbackKind::Trait,
            doc: None,
        }
    }

    fn result_callback() -> ir::definitions::CallbackTraitDef {
        ir::definitions::CallbackTraitDef {
            id: "ResultCallback".into(),
            methods: vec![ir::definitions::CallbackMethodDef {
                id: "compute".into(),
                params: vec![param(
                    "value",
                    ir::types::TypeExpr::Primitive(ir::types::PrimitiveType::I32),
                )],
                returns: ir::definitions::ReturnDef::Result {
                    ok: ir::types::TypeExpr::Primitive(ir::types::PrimitiveType::I32),
                    err: ir::types::TypeExpr::Record("MathError".into()),
                },
                execution_kind: boltffi_ffi_rules::callable::ExecutionKind::Sync,
                doc: None,
            }],
            kind: ir::definitions::CallbackKind::Trait,
            doc: None,
        }
    }

    fn async_callback() -> ir::definitions::CallbackTraitDef {
        ir::definitions::CallbackTraitDef {
            id: "AsyncFetcher".into(),
            methods: vec![ir::definitions::CallbackMethodDef {
                id: "fetch".into(),
                params: vec![param(
                    "key",
                    ir::types::TypeExpr::Primitive(ir::types::PrimitiveType::I32),
                )],
                returns: ir::definitions::ReturnDef::Value(ir::types::TypeExpr::Primitive(
                    ir::types::PrimitiveType::I32,
                )),
                execution_kind: boltffi_ffi_rules::callable::ExecutionKind::Async,
                doc: None,
            }],
            kind: ir::definitions::CallbackKind::Trait,
            doc: None,
        }
    }

    fn custom_type(id: &str, repr: ir::types::TypeExpr) -> ir::definitions::CustomTypeDef {
        ir::definitions::CustomTypeDef {
            id: id.into(),
            rust_type: ir::ids::QualifiedName::new(format!("demo::{id}")),
            repr,
            converters: ir::ids::ConverterPath {
                into_ffi: ir::ids::QualifiedName::new("into_ffi"),
                try_from_ffi: ir::ids::QualifiedName::new("try_from_ffi"),
            },
            doc: None,
        }
    }

    fn empty_contract() -> ir::FfiContract {
        ir::FfiContract {
            package: ir::PackageInfo {
                name: "demo".to_string(),
                version: None,
            },
            catalog: ir::TypeCatalog::new(),
            functions: Vec::new(),
        }
    }

    #[test]
    fn empty_records_render_as_objects() {
        let record = empty_record();

        let common = KMPEmitter::render_common_record(&record);
        let actual = KMPEmitter::render_record_actual_conversions(
            &record,
            "com.example.demo.jvm",
            &empty_contract(),
        );

        assert_eq!(common, "object Empty");
        assert!(actual.contains("= com.example.demo.jvm.Empty"));
        assert!(actual.contains("= Empty"));
        assert!(!common.contains("data class Empty()"));
        assert!(!actual.contains("Empty()"));
    }

    #[test]
    fn error_record_message_fields_use_normalized_kotlin_name() {
        let mut record = error_record("ServiceError");
        record.fields = vec![field("Message", ir::types::TypeExpr::String)];

        let common = KMPEmitter::render_common_record(&record);

        assert_eq!(
            common,
            "data class ServiceError(override val message: String) : Exception(message)"
        );
    }

    #[test]
    fn classes_render_common_and_jvm_actual_delegate_wrappers() {
        let mut contract = empty_contract();
        contract.catalog.insert_class(counter_class());

        let support = KmpSurfaceSupport::for_contract(&contract);
        let common = KMPEmitter::render_common_surface(
            &contract,
            "com.example.demo",
            &support,
            FactoryStyle::Constructors,
        );
        let actual = KMPEmitter::render_kotlin_jvm_actual(
            &contract,
            "com.example.demo",
            "com.example.demo.jvm",
            &support,
            FactoryStyle::Constructors,
        );

        assert!(common.contains("expect class Counter {"));
        assert!(common.contains("    constructor(initial: Int)"));
        assert!(common.contains("    fun close()"));
        assert!(common.contains("    fun `get`(): Int"));
        assert!(common.contains("    fun reset()"));
        assert!(common.contains("        fun zero(): Counter"));
        assert!(common.contains("        fun identity(counter: Counter): Counter?"));

        assert!(actual.contains(
            "actual class Counter internal constructor(internal val delegate: com.example.demo.jvm.Counter)"
        ));
        assert!(!actual.contains("private fun newCounter0(initial: Int)"));
        assert!(actual.contains("actual constructor(initial: Int) : this("));
        assert!(actual.contains("com.example.demo.jvm.Counter(initial)"));
        assert!(actual.contains("actual fun close()"));
        assert!(actual.contains("delegate.close()"));
        assert!(actual.contains("actual fun `get`(): Int"));
        assert!(actual.contains("return this.delegate.`get`()"));
        assert!(actual.contains("actual fun reset()"));
        assert!(actual.contains("this.delegate.reset()"));
        assert!(actual.contains("actual companion object"));
        assert!(actual.contains("actual fun zero(): Counter"));
        assert!(actual.contains("return Counter(com.example.demo.jvm.Counter.zero())"));
        assert!(actual.contains("actual fun identity(counter: Counter): Counter?"));
        assert!(actual.contains("com.example.demo.jvm.Counter.identity(counter.delegate)"));
        assert!(actual.contains("?.let { Counter(it) }"));
    }

    #[test]
    fn class_constructor_actuals_do_not_collide_with_top_level_functions() {
        let mut contract = empty_contract();
        contract.catalog.insert_class(counter_class());
        contract.functions.push(ir::definitions::FunctionDef {
            id: "new_counter0".into(),
            params: vec![param(
                "initial",
                ir::types::TypeExpr::Primitive(ir::types::PrimitiveType::I32),
            )],
            returns: ir::definitions::ReturnDef::Void,
            execution_kind: boltffi_ffi_rules::callable::ExecutionKind::Sync,
            doc: None,
            deprecated: None,
        });

        let support = KmpSurfaceSupport::for_contract(&contract);
        let actual = KMPEmitter::render_kotlin_jvm_actual(
            &contract,
            "com.example.demo",
            "com.example.demo.jvm",
            &support,
            FactoryStyle::Constructors,
        );

        assert!(actual.contains("actual fun newCounter0(initial: Int)"));
        assert!(!actual.contains("private fun newCounter0(initial: Int)"));
        assert!(actual.contains("actual constructor(initial: Int) : this("));
        assert!(actual.contains("com.example.demo.jvm.Counter(initial)"));
    }

    #[test]
    fn optional_class_constructors_are_rejected_for_now() {
        let mut contract = empty_contract();
        contract.catalog.insert_class(optional_counter_class());

        let support = KmpSurfaceSupport::for_contract(&contract);
        let common = KMPEmitter::render_common_surface(
            &contract,
            "com.example.demo",
            &support,
            FactoryStyle::Constructors,
        );
        let actual = KMPEmitter::render_kotlin_jvm_actual(
            &contract,
            "com.example.demo",
            "com.example.demo.jvm",
            &support,
            FactoryStyle::Constructors,
        );

        assert!(common.contains("expect class OptionalCounter {"));
        assert!(!common.contains("constructor(initial: Int)"));
        assert!(!common.contains("fun maybe(): OptionalCounter"));
        assert!(
            common.contains(
                "Unsupported in this KMP class slice: constructor new, constructor maybe"
            )
        );
        assert!(!actual.contains("actual constructor(initial: Int)"));
        assert!(!actual.contains("actual fun maybe(): OptionalCounter"));
    }

    #[test]
    fn instance_method_actuals_qualify_delegate_receiver() {
        let mut contract = empty_contract();
        contract.catalog.insert_class(delegate_param_class());

        let support = KmpSurfaceSupport::for_contract(&contract);
        let actual = KMPEmitter::render_kotlin_jvm_actual(
            &contract,
            "com.example.demo",
            "com.example.demo.jvm",
            &support,
            FactoryStyle::Constructors,
        );

        assert!(actual.contains("actual fun acceptDelegate(`delegate`: Int)"));
        assert!(actual.contains("this.delegate.acceptDelegate(`delegate`)"));
        assert!(!actual.contains("            delegate.acceptDelegate(`delegate`)"));
    }

    #[test]
    fn async_streams_render_common_flow_apis_and_jvm_flow_adapters() {
        let mut contract = empty_contract();
        contract.catalog.insert_record(point_record());
        contract.catalog.insert_class(event_bus_class());

        let support = KmpSurfaceSupport::for_contract(&contract);
        let common = KMPEmitter::render_common_surface(
            &contract,
            "com.example.demo",
            &support,
            FactoryStyle::Constructors,
        );
        let actual = KMPEmitter::render_kotlin_jvm_actual(
            &contract,
            "com.example.demo",
            "com.example.demo.jvm",
            &support,
            FactoryStyle::Constructors,
        );

        assert!(common.contains("import kotlinx.coroutines.flow.Flow"));
        assert!(common.contains("expect class EventBus {"));
        assert!(common.contains("    fun subscribeValues(): Flow<Int>"));
        assert!(common.contains("    fun subscribePoints(): Flow<Point>"));
        assert!(
            common.contains(
                "Unsupported in this KMP class slice: stream subscribe_values_batch, stream subscribe_values_callback"
            )
        );
        assert!(!common.contains("fun subscribeValuesBatch(): Flow<Int>"));
        assert!(!common.contains("fun subscribeValuesCallback(): Flow<Int>"));

        assert!(actual.contains("import kotlinx.coroutines.flow.Flow"));
        assert!(actual.contains("import kotlinx.coroutines.flow.catch"));
        assert!(actual.contains("import kotlinx.coroutines.flow.map"));
        assert!(actual.contains("actual fun subscribeValues(): Flow<Int> ="));
        assert!(actual.contains("this.delegate.subscribeValues()"));
        assert!(actual.contains(".map { boltffiStreamItem -> boltffiStreamItem }"));
        assert!(actual.contains("if (err is com.example.demo.jvm.FfiException)"));
        assert!(actual.contains("throw err.toBoltFfiCommon()"));
        assert!(actual.contains("actual fun subscribePoints(): Flow<Point> ="));
        assert!(actual.contains("this.delegate.subscribePoints()"));
        assert!(
            actual.contains(".map { boltffiStreamItem -> boltffiStreamItem.toBoltFfiCommon() }")
        );
    }

    #[test]
    fn async_stream_lifecycle_stays_in_platform_delegate_flow() {
        let mut contract = empty_contract();
        contract.catalog.insert_record(point_record());
        contract.catalog.insert_class(event_bus_class());
        let abi = ir::lower::Lowerer::new(&contract).to_abi_contract();
        let output = KMPEmitter::emit(
            &contract,
            &abi,
            KMPOptions {
                package_name: "com.example.demo".to_string(),
                module_name: "Demo".to_string(),
                min_sdk: 23,
                kotlin_options: KotlinOptions::default(),
                native_library_name: "demo".to_string(),
                apple_targets: Vec::new(),
            },
        );

        let jvm_internal_path =
            std::path::PathBuf::from("src/jvmMain/kotlin/com/example/demo/jvm/Demo.kt");
        let jvm_internal = output
            .files
            .iter()
            .find(|file| file.relative_path == jvm_internal_path)
            .expect("internal JVM source should be generated")
            .contents
            .as_str();

        assert!(jvm_internal.contains("fun subscribeValues(): Flow<Int> = callbackFlow {"));
        assert!(
            jvm_internal
                .contains("val subscription = Native.boltffi_event_bus_subscribe_values(handle)")
        );
        assert!(jvm_internal.contains("awaitClose { context.requestTermination() }"));
        assert!(
            jvm_internal
                .contains("unsubscribe = Native::boltffi_event_bus_subscribe_values_unsubscribe")
        );
        assert!(jvm_internal.contains("freeFn = Native::boltffi_event_bus_subscribe_values_free"));
        assert!(jvm_internal.contains("finish = { close() }"));
    }

    #[test]
    fn sync_callback_params_render_common_interfaces_and_jvm_adapters() {
        let mut contract = empty_contract();
        contract.catalog.insert_callback(value_callback());
        contract.functions.push(ir::definitions::FunctionDef {
            id: "invoke_value_callback".into(),
            params: vec![
                callback_param("callback", "ValueCallback"),
                param(
                    "input",
                    ir::types::TypeExpr::Primitive(ir::types::PrimitiveType::I32),
                ),
            ],
            returns: ir::definitions::ReturnDef::Value(ir::types::TypeExpr::Primitive(
                ir::types::PrimitiveType::I32,
            )),
            execution_kind: boltffi_ffi_rules::callable::ExecutionKind::Sync,
            doc: None,
            deprecated: None,
        });

        let support = KmpSurfaceSupport::for_contract(&contract);
        let common = KMPEmitter::render_common_surface(
            &contract,
            "com.example.demo",
            &support,
            FactoryStyle::Constructors,
        );
        let actual = KMPEmitter::render_kotlin_jvm_actual(
            &contract,
            "com.example.demo",
            "com.example.demo.jvm",
            &support,
            FactoryStyle::Constructors,
        );

        assert!(common.contains("interface ValueCallback {"));
        assert!(common.contains("fun onValue(`value`: Int): Int"));
        assert!(
            common.contains(
                "expect fun invokeValueCallback(callback: ValueCallback, input: Int): Int"
            )
        );
        assert!(
            actual.contains(
                "actual fun invokeValueCallback(callback: ValueCallback, input: Int): Int"
            )
        );
        assert!(actual.contains("object : com.example.demo.jvm.ValueCallback"));
        assert!(actual.contains("override fun onValue(`value`: Int): Int"));
        assert!(actual.contains("boltffiCommonCallback.onValue(`value`)"));
        assert!(actual.contains("com.example.demo.jvm.invokeValueCallback(kotlin.run"));
    }

    #[test]
    fn result_callback_adapters_convert_common_errors_to_internal_errors() {
        let mut contract = empty_contract();
        contract.catalog.insert_record(error_record("MathError"));
        contract.catalog.insert_callback(result_callback());
        contract.functions.push(ir::definitions::FunctionDef {
            id: "invoke_result_callback".into(),
            params: vec![callback_param("callback", "ResultCallback")],
            returns: ir::definitions::ReturnDef::Value(ir::types::TypeExpr::Primitive(
                ir::types::PrimitiveType::I32,
            )),
            execution_kind: boltffi_ffi_rules::callable::ExecutionKind::Sync,
            doc: None,
            deprecated: None,
        });

        let support = KmpSurfaceSupport::for_contract(&contract);
        let common = KMPEmitter::render_common_surface(
            &contract,
            "com.example.demo",
            &support,
            FactoryStyle::Constructors,
        );
        let actual = KMPEmitter::render_kotlin_jvm_actual(
            &contract,
            "com.example.demo",
            "com.example.demo.jvm",
            &support,
            FactoryStyle::Constructors,
        );

        assert!(common.contains("interface ResultCallback {"));
        assert!(common.contains("fun compute(`value`: Int): Int"));
        assert!(actual.contains("catch (err: MathError)"));
        assert!(actual.contains("throw err.toBoltFfiJvm()"));
        assert!(actual.contains("catch (err: FfiException)"));
        assert!(actual.contains("throw com.example.demo.jvm.FfiException"));
    }

    #[test]
    fn async_callbacks_remain_unsupported_in_the_sync_callback_slice() {
        let mut contract = empty_contract();
        contract.catalog.insert_callback(async_callback());
        contract.functions.push(ir::definitions::FunctionDef {
            id: "fetch_with_async_callback".into(),
            params: vec![callback_param("fetcher", "AsyncFetcher")],
            returns: ir::definitions::ReturnDef::Value(ir::types::TypeExpr::Primitive(
                ir::types::PrimitiveType::I32,
            )),
            execution_kind: boltffi_ffi_rules::callable::ExecutionKind::Sync,
            doc: None,
            deprecated: None,
        });

        let support = KmpSurfaceSupport::for_contract(&contract);
        let common = KMPEmitter::render_common_surface(
            &contract,
            "com.example.demo",
            &support,
            FactoryStyle::Constructors,
        );

        assert!(!common.contains("interface AsyncFetcher"));
        assert!(!common.contains("expect fun fetchWithAsyncCallback"));
        assert!(
            common.contains(
                "Unsupported in the initial KMP generator slice: fetch_with_async_callback"
            )
        );
    }

    #[test]
    fn error_enum_message_fields_override_throwable_message() {
        let enumeration = ir::definitions::EnumDef {
            id: "DomainError".into(),
            repr: EnumRepr::Data {
                tag_type: ir::types::PrimitiveType::I32,
                variants: vec![ir::definitions::DataVariant {
                    name: "Invalid".into(),
                    discriminant: 0,
                    payload: VariantPayload::Struct(vec![field(
                        "message",
                        ir::types::TypeExpr::String,
                    )]),
                    doc: None,
                }],
            },
            is_error: true,
            constructors: Vec::new(),
            methods: Vec::new(),
            doc: None,
            deprecated: None,
        };

        let common = KMPEmitter::render_common_sealed_enum(
            &enumeration,
            "com.example.demo",
            &KmpSurfaceSupport {
                records: HashSet::new(),
                enums: HashSet::new(),
                custom_types: HashSet::new(),
                classes: HashSet::new(),
                callbacks: HashSet::new(),
                streams: HashSet::new(),
            },
        );

        assert!(
            common.contains("data class Invalid(override val message: String) : DomainError()")
        );
        assert!(!common.contains("data class Invalid(val message: String)"));
    }

    #[test]
    fn result_actual_catches_errors_behind_custom_aliases() {
        let mut contract = empty_contract();
        contract.catalog.insert_record(error_record("ServiceError"));
        contract.catalog.insert_custom(custom_type(
            "ServiceFailureWire",
            ir::types::TypeExpr::Record("ServiceError".into()),
        ));
        contract.catalog.insert_custom(custom_type(
            "ServiceFailure",
            ir::types::TypeExpr::Custom("ServiceFailureWire".into()),
        ));
        contract.functions.push(ir::definitions::FunctionDef {
            id: "load".into(),
            params: Vec::new(),
            returns: ir::definitions::ReturnDef::Result {
                ok: ir::types::TypeExpr::String,
                err: ir::types::TypeExpr::Custom("ServiceFailure".into()),
            },
            execution_kind: boltffi_ffi_rules::callable::ExecutionKind::Sync,
            doc: None,
            deprecated: None,
        });

        let support = KmpSurfaceSupport::for_contract(&contract);
        assert!(function_supported(
            &contract.functions[0],
            &contract,
            &support
        ));

        let actual = KMPEmitter::render_kotlin_jvm_function_actual(
            &contract.functions[0],
            &contract,
            "com.example.demo.jvm",
        );

        assert!(actual.contains("catch (err: com.example.demo.jvm.ServiceError)"));
        assert!(!actual.contains("catch (err: com.example.demo.jvm.ServiceFailure)"));
        assert!(actual.contains("catch (err: com.example.demo.jvm.FfiException)"));
    }

    #[test]
    fn result_actual_catches_errors_behind_options() {
        let mut contract = empty_contract();
        contract.catalog.insert_record(error_record("ServiceError"));
        contract.functions.push(ir::definitions::FunctionDef {
            id: "load".into(),
            params: Vec::new(),
            returns: ir::definitions::ReturnDef::Result {
                ok: ir::types::TypeExpr::String,
                err: ir::types::TypeExpr::Option(Box::new(ir::types::TypeExpr::Record(
                    "ServiceError".into(),
                ))),
            },
            execution_kind: boltffi_ffi_rules::callable::ExecutionKind::Sync,
            doc: None,
            deprecated: None,
        });

        let support = KmpSurfaceSupport::for_contract(&contract);
        assert!(function_supported(
            &contract.functions[0],
            &contract,
            &support
        ));

        let actual = KMPEmitter::render_kotlin_jvm_function_actual(
            &contract.functions[0],
            &contract,
            "com.example.demo.jvm",
        );

        assert!(actual.contains("catch (err: com.example.demo.jvm.ServiceError)"));
        assert!(actual.contains("catch (err: com.example.demo.jvm.FfiException)"));
    }

    #[test]
    fn non_result_actual_converts_internal_ffi_exception() {
        let mut contract = empty_contract();
        contract.functions.push(ir::definitions::FunctionDef {
            id: "describe".into(),
            params: Vec::new(),
            returns: ir::definitions::ReturnDef::Value(ir::types::TypeExpr::String),
            execution_kind: boltffi_ffi_rules::callable::ExecutionKind::Sync,
            doc: None,
            deprecated: None,
        });

        let actual = KMPEmitter::render_kotlin_jvm_function_actual(
            &contract.functions[0],
            &contract,
            "com.example.demo.jvm",
        );

        assert!(actual.contains("actual fun describe(): String {"));
        assert!(actual.contains("return com.example.demo.jvm.describe()"));
        assert!(actual.contains("catch (err: com.example.demo.jvm.FfiException)"));
        assert!(!actual.contains("actual fun describe(): String ="));
    }

    #[test]
    fn data_enum_payload_types_disambiguate_kotlin_builtins() {
        let enumeration = ir::definitions::EnumDef {
            id: "JsonValue".into(),
            repr: EnumRepr::Data {
                tag_type: ir::types::PrimitiveType::I32,
                variants: vec![
                    ir::definitions::DataVariant {
                        name: "String".into(),
                        discriminant: 0,
                        payload: VariantPayload::Tuple(vec![ir::types::TypeExpr::String]),
                        doc: None,
                    },
                    ir::definitions::DataVariant {
                        name: "Int".into(),
                        discriminant: 1,
                        payload: VariantPayload::Tuple(vec![ir::types::TypeExpr::Primitive(
                            ir::types::PrimitiveType::I32,
                        )]),
                        doc: None,
                    },
                    ir::definitions::DataVariant {
                        name: "ByteArray".into(),
                        discriminant: 2,
                        payload: VariantPayload::Tuple(vec![ir::types::TypeExpr::Bytes]),
                        doc: None,
                    },
                    ir::definitions::DataVariant {
                        name: "IntArray".into(),
                        discriminant: 3,
                        payload: VariantPayload::Tuple(vec![ir::types::TypeExpr::Vec(Box::new(
                            ir::types::TypeExpr::Primitive(ir::types::PrimitiveType::I32),
                        ))]),
                        doc: None,
                    },
                    ir::definitions::DataVariant {
                        name: "List".into(),
                        discriminant: 4,
                        payload: VariantPayload::Tuple(vec![ir::types::TypeExpr::Vec(Box::new(
                            ir::types::TypeExpr::String,
                        ))]),
                        doc: None,
                    },
                    ir::definitions::DataVariant {
                        name: "bolt_f_f_i_result".into(),
                        discriminant: 5,
                        payload: VariantPayload::Tuple(vec![ir::types::TypeExpr::Result {
                            ok: Box::new(ir::types::TypeExpr::String),
                            err: Box::new(ir::types::TypeExpr::String),
                        }]),
                        doc: None,
                    },
                ],
            },
            is_error: false,
            constructors: Vec::new(),
            methods: Vec::new(),
            doc: None,
            deprecated: None,
        };

        let common = KMPEmitter::render_common_sealed_enum(
            &enumeration,
            "com.example.demo",
            &KmpSurfaceSupport {
                records: HashSet::new(),
                enums: HashSet::new(),
                custom_types: HashSet::new(),
                classes: HashSet::new(),
                callbacks: HashSet::new(),
                streams: HashSet::new(),
            },
        );

        assert_eq!(
            NamingConvention::class_name("bolt_f_f_i_result"),
            "BoltFFIResult"
        );
        assert!(common.contains("data class String(val value0: kotlin.String) : JsonValue()"));
        assert!(common.contains("data class Int(val value0: kotlin.Int) : JsonValue()"));
        assert!(
            common.contains("data class ByteArray(val value0: kotlin.ByteArray) : JsonValue()")
        );
        assert!(common.contains("data class IntArray(val value0: kotlin.IntArray) : JsonValue()"));
        assert!(common.contains(
            "data class List(val value0: kotlin.collections.List<kotlin.String>) : JsonValue()"
        ));
        assert!(common.contains(
            "data class BoltFFIResult(val value0: com.example.demo.BoltFFIResult<kotlin.String, kotlin.String>) : JsonValue()"
        ));
    }

    #[test]
    fn default_platform_adapters_are_jvm_family_actuals() {
        let adapters = KMPEmitter::default_platform_adapters();

        assert_eq!(
            adapters,
            vec![KmpPlatformAdapter::jvm(), KmpPlatformAdapter::android()]
        );
        assert!(
            adapters
                .iter()
                .all(|adapter| matches!(adapter.backend, KmpActualBackend::KotlinJvm))
        );
    }

    #[test]
    fn apple_platform_adapter_is_opt_in() {
        let adapters = KMPEmitter::platform_adapters(true);

        assert_eq!(adapters.len(), 3);
        assert_eq!(adapters[2], KmpPlatformAdapter::apple());
        assert_eq!(adapters[2].source_set, "appleMain");
        assert!(matches!(
            adapters[2].backend,
            KmpActualBackend::KotlinNativeApple
        ));
    }

    #[test]
    fn apple_scaffold_generates_targets_cinterop_and_stub_actuals() {
        let mut contract = empty_contract();
        contract.catalog.insert_record(point_record());
        contract.catalog.insert_class(counter_class());
        contract.catalog.insert_class(event_bus_class());
        contract.functions.push(ir::definitions::FunctionDef {
            id: "echo_point".into(),
            params: vec![param("point", ir::types::TypeExpr::Record("Point".into()))],
            returns: ir::definitions::ReturnDef::Value(ir::types::TypeExpr::Record("Point".into())),
            execution_kind: boltffi_ffi_rules::callable::ExecutionKind::Sync,
            doc: None,
            deprecated: None,
        });
        let abi = ir::lower::Lowerer::new(&contract).to_abi_contract();
        let output = KMPEmitter::emit(
            &contract,
            &abi,
            KMPOptions {
                package_name: "com.example.demo".to_string(),
                module_name: "Demo".to_string(),
                min_sdk: 23,
                kotlin_options: KotlinOptions::default(),
                native_library_name: "demo".to_string(),
                apple_targets: vec![KmpAppleTarget::IosArm64, KmpAppleTarget::IosSimulatorArm64],
            },
        );

        let build_gradle = output
            .files
            .iter()
            .find(|file| file.relative_path == PathBuf::from("build.gradle.kts"))
            .expect("build.gradle should be generated")
            .contents
            .as_str();
        let apple_actual = output
            .files
            .iter()
            .find(|file| {
                file.relative_path
                    == PathBuf::from("src/appleMain/kotlin/com/example/demo/DemoAppleActual.kt")
            })
            .expect("apple actual source should be generated")
            .contents
            .as_str();
        let cinterop_def = output
            .files
            .iter()
            .find(|file| {
                file.relative_path == PathBuf::from("src/nativeInterop/cinterop/boltffi.def")
            })
            .expect("cinterop definition should be generated")
            .contents
            .as_str();
        let header = output
            .files
            .iter()
            .find(|file| {
                file.relative_path == PathBuf::from("src/nativeInterop/cinterop/include/demo.h")
            })
            .expect("c header should be generated")
            .contents
            .as_str();

        assert!(build_gradle.contains("iosArm64(),"));
        assert!(build_gradle.contains("iosSimulatorArm64(),"));
        assert!(build_gradle.contains("val appleMain = maybeCreate(\"appleMain\")"));
        assert!(build_gradle.contains("val iosArm64Main by getting"));
        assert!(build_gradle.contains("val iosSimulatorArm64Main by getting"));
        assert!(apple_actual.contains("actual class Counter {"));
        assert!(apple_actual.contains("actual constructor(initial: Int)"));
        assert!(apple_actual.contains(
            "actual fun echoPoint(point: Point): Point = throw UnsupportedOperationException"
        ));
        assert!(apple_actual.contains(
            "actual fun subscribeValues(): Flow<Int> = throw UnsupportedOperationException"
        ));
        assert!(build_gradle.contains("includeDirs(\"src/nativeInterop/cinterop/include\")"));
        assert!(cinterop_def.contains("headers = demo.h"));
        assert!(header.contains("typedef struct FfiStatus"));
    }

    #[test]
    fn surfaces_render_common_once_and_platform_actuals_separately() {
        let adapters = KMPEmitter::default_platform_adapters();
        let rendered = KMPEmitter::render_surfaces(
            &empty_contract(),
            "com.example.demo",
            "com.example.demo.jvm",
            FactoryStyle::Constructors,
            &adapters,
        );

        assert!(rendered.common.contains("package com.example.demo"));
        assert_eq!(rendered.platform_actuals.len(), 2);
        assert_eq!(rendered.platform_actuals[0].adapter.source_set, "jvmMain");
        assert_eq!(
            rendered.platform_actuals[1].adapter.source_set,
            "androidMain"
        );
        assert!(
            rendered
                .platform_actuals
                .iter()
                .all(|actual| actual.contents.contains("package com.example.demo"))
        );
    }

    #[test]
    fn unsigned_enum_discriminants_render_as_valid_signed_kotlin_literals() {
        assert_eq!(
            enum_literal(u32::MAX as i128, ir::types::PrimitiveType::U32),
            "(4294967295L).toInt()"
        );
        assert_eq!(
            enum_literal(u64::MAX as i128, ir::types::PrimitiveType::U64),
            "(18446744073709551615uL).toLong()"
        );
    }
}
