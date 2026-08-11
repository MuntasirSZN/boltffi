use boltffi_binding::BindingMetadataSurface;

use crate::{
    build::{
        BindingExpansion, BuildOptions, BuildSelection, Builder, CargoBuildProfile, OutputCallback,
        all_successful, failed_targets, resolve_build_profile,
    },
    cargo::Cargo,
    cli::{CliError, Result},
    commands::{
        generate::{GenerateOptions, GenerateTarget, run_generate_with_output},
        pack::PackDartOptions,
    },
    config::Config,
    pack::{PackError, print_cargo_line, resolve_build_cargo_args},
    reporter::{Reporter, Step},
};

fn build_dart_targets(
    config: &Config,
    release: bool,
    build_cargo_args: &[String],
    step: &Step,
) -> Result<()> {
    let on_output: Option<OutputCallback> = if step.is_verbose() {
        Some(Box::new(|line: &str| print_cargo_line(line)))
    } else {
        None
    };

    // Cargo only sets CARGO_FEATURE_* for build scripts, so this must build
    // as a binding expansion for the macros to see active features (same
    // fix as the Python target's cdylib build).
    let expansion = BindingExpansion::resolve_for_surface(
        config,
        build_cargo_args,
        BindingMetadataSurface::Native,
    )?;

    let builder = Builder::new(config, dart_build_options(expansion, release, on_output));
    let results = builder.build_targets(&config.dart_targets())?;

    if all_successful(&results) {
        return Ok(());
    }

    let failed = failed_targets(&results);
    Err(CliError::Pack(PackError::BuildFailed { targets: failed }))
}

fn dart_build_options(
    expansion: BindingExpansion,
    release: bool,
    on_output: Option<OutputCallback>,
) -> BuildOptions {
    BuildOptions {
        release,
        selection: BuildSelection::Expanded(Box::new(expansion)),
        on_output,
    }
}

pub(crate) fn pack_dart(
    config: &Config,
    options: PackDartOptions,
    reporter: &Reporter,
) -> Result<()> {
    if !config.is_dart_enabled() {
        return Err(CliError::CommandFailed {
            command: "targets.dart.enabled = false".to_string(),
            status: None,
        });
    }

    reporter.section("☕", "Packing Dart");

    let build_cargo_args = resolve_build_cargo_args(config, &options.execution.cargo_args);
    let build_profile = resolve_build_profile(options.execution.release, &build_cargo_args);

    if !options.execution.no_build {
        let step = reporter.step("Building Rust cdylib");
        build_dart_targets(
            config,
            matches!(build_profile, CargoBuildProfile::Release),
            &build_cargo_args,
            &step,
        )?;
        step.finish_success();
    }

    if options.execution.regenerate {
        let step = reporter.step("Generating Dart bindings");
        run_generate_with_output(
            config,
            GenerateOptions {
                target: GenerateTarget::Dart,
                output: Some(config.dart_output()),
                experimental: options.experimental,
                cargo_args: build_cargo_args.clone(),
                deny_skipped: options.execution.deny_skipped,
            },
        )?;

        step.finish_success();
    }

    let step = reporter.step("Packaging native libraries");

    let cargo = Cargo::current(&build_cargo_args)?;

    let metadata = cargo.metadata()?;
    let cargo_manifest_path = cargo.manifest_path()?;
    let package_selector =
        cargo.effective_package_selector(config, &metadata, &cargo_manifest_path);

    let libraries = metadata.resolve_built_libraries_for_targets(
        &cargo_manifest_path,
        build_profile.output_directory_name(),
        &config.crate_artifact_name(),
        package_selector.as_deref(),
        &config.dart_targets(),
    )?;

    let package_dir = config.dart_output().join(&config.package.name);
    let native_libs_dir = package_dir.join("native");
    std::fs::create_dir_all(&native_libs_dir).map_err(|source| {
        CliError::CreateDirectoryFailed {
            path: native_libs_dir.clone(),
            source,
        }
    })?;

    for l in libraries {
        let native_lib_triple_dir = native_libs_dir.join(l.target.triple());
        std::fs::create_dir_all(&native_lib_triple_dir).map_err(|source| {
            CliError::CreateDirectoryFailed {
                path: native_lib_triple_dir.clone(),
                source,
            }
        })?;

        let native_lib_filepath =
            native_lib_triple_dir.join(l.path.file_name().expect("file shouldn't terminate in .."));

        std::fs::copy(&l.path, &native_lib_filepath).map_err(|source| CliError::CopyFailed {
            from: l.path,
            to: native_lib_filepath,
            source,
        })?;
    }

    step.finish_success();

    reporter.finish();
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{BindingExpansion, BuildSelection, dart_build_options};

    /// `pack dart` must build the cdylib as a binding expansion, not a plain
    /// `cargo build`: the #[data]/#[error] macros read active features from
    /// BINDING_METADATA_FEATURES_ENV, which only `BuildSelection::Expanded`
    /// wires up (see `Builder::apply_expansion`). A plain build silently
    /// drops every #[cfg(feature = ...)]-gated module from the FFI surface.
    #[test]
    fn dart_cdylib_builds_as_a_binding_expansion() {
        let expansion = BindingExpansion::fixture(
            "/workspace/Cargo.toml",
            "/workspace/demo/Cargo.toml",
            ["--features".to_string(), "ffi".to_string()],
        );

        let options = dart_build_options(expansion, false, None);

        assert!(matches!(options.selection, BuildSelection::Expanded(_)));
    }
}
