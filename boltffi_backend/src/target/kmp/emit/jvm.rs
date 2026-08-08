//! JVM-family source-set file rendering for KMP emission.

use askama::Template as AskamaTemplate;

use crate::{
    core::Result,
    target::{jvm::NativeLibraries, kotlin::render::native_library_loader::NativeLibraryLoader},
};

use super::{
    super::plan::{KmpApiBody, KmpFunctionPlan, KmpModule},
    common::{RenderedFunction, unsupported_body_emission},
};

#[derive(AskamaTemplate)]
#[template(path = "target/kmp/platform_actual.kt", escape = "none")]
struct PlatformActualTemplate<'module> {
    package_name: &'module str,
    internal_package: &'module str,
    functions: Vec<RenderedFunction>,
}

#[derive(AskamaTemplate)]
#[template(path = "target/kmp/internal_kotlin.kt", escape = "none")]
struct InternalKotlinTemplate<'module> {
    internal_package: &'module str,
    native_library_loader: String,
    native_functions: Vec<RenderedFunction>,
    functions: Vec<RenderedFunction>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct KmpJvmAdapter {
    pub(crate) source_set: &'static str,
    pub(crate) actual_file_suffix: &'static str,
}

impl KmpJvmAdapter {
    pub(crate) const fn jvm() -> Self {
        Self {
            source_set: "jvmMain",
            actual_file_suffix: "JvmActual",
        }
    }

    pub(crate) const fn android() -> Self {
        Self {
            source_set: "androidMain",
            actual_file_suffix: "AndroidActual",
        }
    }
}

pub(crate) fn default_adapters() -> Vec<KmpJvmAdapter> {
    vec![KmpJvmAdapter::jvm(), KmpJvmAdapter::android()]
}

pub(crate) fn render_platform_actual(
    module: &KmpModule,
    package_name: &str,
    internal_package: &str,
) -> Result<String> {
    let functions = function_plans(module)?;

    Ok(PlatformActualTemplate {
        package_name,
        internal_package,
        functions: rendered_functions(&functions)?,
    }
    .render()?)
}

pub(crate) fn render_internal_kotlin(
    module: &KmpModule,
    internal_package: &str,
    native_libraries: &NativeLibraries,
) -> Result<String> {
    let functions = function_plans(module)?;

    Ok(InternalKotlinTemplate {
        internal_package,
        native_library_loader: NativeLibraryLoader::new(native_libraries).render()?,
        native_functions: rendered_functions(&functions)?,
        functions: rendered_functions(&functions)?,
    }
    .render()?)
}

fn function_plans(module: &KmpModule) -> Result<Vec<&KmpFunctionPlan>> {
    module
        .common()
        .apis()
        .iter()
        .map(|api| match api.body() {
            KmpApiBody::Function(function) => Ok(function),
            KmpApiBody::Unsupported => Err(unsupported_body_emission()),
        })
        .collect()
}

fn rendered_functions(functions: &[&KmpFunctionPlan]) -> Result<Vec<RenderedFunction>> {
    functions
        .iter()
        .map(|function| RenderedFunction::from_plan(function))
        .collect()
}
