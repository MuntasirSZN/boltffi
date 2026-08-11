use boltffi_binding::BindingMetadataSurface;

use crate::build::{
    BindingExpansion, BuildOptions, BuildResult, BuildSelection, Builder, all_successful,
    count_successful, failed_targets, resolve_build_profile,
};
use crate::cli::Result;
use crate::config::Config;
use crate::pack::PackError;

pub enum BuildPlatform {
    Apple,
    Android,
    Wasm,
    Dart,
    All,
}

pub struct BuildCommandOptions {
    pub platform: BuildPlatform,
    pub release: bool,
    pub cargo_args: Vec<String>,
}

pub fn run_build(config: &Config, options: BuildCommandOptions) -> Result<Vec<BuildResult>> {
    let BuildCommandOptions {
        platform,
        release,
        cargo_args: cli_cargo_args,
    } = options;

    let cargo_args: Vec<String> = config
        .cargo_args_for_command("build")
        .into_iter()
        .chain(cli_cargo_args)
        .collect();

    let build_profile = resolve_build_profile(release, &cargo_args);

    let profile = build_profile.output_directory_name();

    let results = match platform {
        BuildPlatform::Apple => {
            if !config.is_apple_enabled() {
                return Ok(Vec::new());
            }
            println!("Building for Apple ({})...", profile);
            expanded_builder(config, release, cargo_args.clone())?
                .build_targets(&config.apple_targets())?
        }
        BuildPlatform::Android => {
            if !config.is_android_enabled() {
                return Ok(Vec::new());
            }
            println!("Building for Android ({})...", profile);
            expanded_builder(config, release, cargo_args.clone())?
                .build_android(&config.android_targets())?
        }
        BuildPlatform::Wasm => {
            if !config.is_wasm_enabled() {
                return Ok(Vec::new());
            }
            println!("Building for wasm ({})...", profile);
            wasm_builder(config, release, cargo_args.clone())?
                .build_wasm_with_triple(config.wasm_triple())?
        }
        BuildPlatform::Dart => {
            if !config.is_dart_enabled() {
                return Ok(Vec::new());
            }
            println!("Building for dart ({})...", profile);
            expanded_builder(config, release, cargo_args.clone())?
                .build_targets(&config.dart_targets())?
        }
        BuildPlatform::All => {
            println!("Building all targets ({})...", profile);
            let mut all_results = Vec::new();
            if config.is_apple_enabled() {
                all_results.extend(
                    expanded_builder(config, release, cargo_args.clone())?
                        .build_targets(&config.apple_targets())?,
                );
            }
            if config.is_android_enabled() {
                all_results.extend(
                    expanded_builder(config, release, cargo_args.clone())?
                        .build_android(&config.android_targets())?,
                );
            }
            if config.is_wasm_enabled() {
                all_results.extend(
                    wasm_builder(config, release, cargo_args.clone())?
                        .build_wasm_with_triple(config.wasm_triple())?,
                );
            }
            if config.is_dart_enabled() {
                all_results.extend(
                    expanded_builder(config, release, cargo_args.clone())?
                        .build_targets(&config.dart_targets())?,
                );
            }
            all_results
        }
    };

    if results.is_empty() {
        println!("No enabled targets matched the requested platform");
        return Ok(results);
    }

    print_build_results(&results);

    if all_successful(&results) {
        Ok(results)
    } else {
        Err(PackError::BuildFailed {
            targets: failed_targets(&results),
        }
        .into())
    }
}

// Cargo only sets CARGO_FEATURE_* for build scripts, so every platform here
// must build as a binding expansion for the #[data]/#[error] macros to see
// active features -- a plain `cargo build` silently drops feature-gated
// modules from the FFI surface (same bug fixed for `pack dart` and, before
// that, `pack python` in 8bd6ab4a).
fn expanded_builder(
    config: &Config,
    release: bool,
    cargo_args: Vec<String>,
) -> Result<Builder<'_>> {
    let expansion = BindingExpansion::resolve(config, &cargo_args)?;
    Ok(Builder::new(
        config,
        expanded_build_options(expansion, release),
    ))
}

// Wasm needs its own surface: `pack wasm` and wasm binding generation both
// resolve with `BindingMetadataSurface::Wasm32`, and the macro picks
// surface-specific lowering off it. The generic `expanded_builder` above
// defaults to `Native`, which would compile the wrong ABI for a wasm32
// target.
fn wasm_builder(config: &Config, release: bool, cargo_args: Vec<String>) -> Result<Builder<'_>> {
    let expansion = wasm_expansion(config, &cargo_args)?;
    Ok(Builder::new(
        config,
        expanded_build_options(expansion, release),
    ))
}

fn wasm_expansion(config: &Config, cargo_args: &[String]) -> Result<BindingExpansion> {
    BindingExpansion::resolve_for_surface(config, cargo_args, BindingMetadataSurface::Wasm32)
}

fn expanded_build_options(expansion: BindingExpansion, release: bool) -> BuildOptions {
    build_options(release, BuildSelection::Expanded(Box::new(expansion)))
}

fn build_options(release: bool, selection: BuildSelection) -> BuildOptions {
    BuildOptions {
        release,
        selection,
        on_output: None,
    }
}

fn print_build_results(results: &[BuildResult]) {
    println!();

    results.iter().for_each(|result| {
        let icon = if result.success { "[ok]" } else { "[failed]" };
        println!("  {} {}", icon, result.triple);
    });

    println!();

    let success_count = count_successful(results);
    let total = results.len();

    if all_successful(results) {
        println!("Built {}/{} targets successfully", success_count, total);
    } else {
        println!(
            "Built {}/{} targets ({} failed)",
            success_count,
            total,
            total - success_count
        );
        println!();
        println!("Failed targets:");
        failed_targets(results).iter().for_each(|triple| {
            println!("  - {}", triple);
        });
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use boltffi_binding::BindingMetadataSurface;

    use super::{BindingExpansion, BuildSelection, expanded_build_options, wasm_expansion};
    use crate::config::Config;

    fn demo_manifest_path() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../examples/demo/Cargo.toml")
    }

    fn parse_config(input: &str) -> Config {
        let parsed: Config = toml::from_str(input).expect("toml parse failed");
        parsed.validate().expect("config validation failed");
        parsed
    }

    /// `boltffi build` (apple/android/wasm/dart/all) must go through the same
    /// binding expansion `pack` does, or the #[data]/#[error] macros never
    /// see active features and silently drop feature-gated modules from the
    /// FFI surface.
    #[test]
    fn build_command_uses_a_binding_expansion() {
        let expansion = BindingExpansion::fixture(
            "/workspace/Cargo.toml",
            "/workspace/demo/Cargo.toml",
            ["--features".to_string(), "ffi".to_string()],
        );

        let options = expanded_build_options(expansion, false);

        assert!(matches!(options.selection, BuildSelection::Expanded(_)));
    }

    /// `boltffi build wasm` must resolve with the wasm32 surface, not the
    /// generic builder's `Native` default -- `pack wasm` and wasm binding
    /// generation both use `Wasm32`, and the macro's surface-specific
    /// lowering has to agree with the TypeScript bindings the wasm build
    /// will be paired with.
    #[test]
    fn wasm_build_resolves_the_wasm32_surface() {
        let config = parse_config("[package]\nname = \"demo\"\n");
        let cargo_args = vec![
            "--manifest-path".to_string(),
            demo_manifest_path().display().to_string(),
        ];

        let expansion = wasm_expansion(&config, &cargo_args).expect("wasm expansion resolves");

        assert_eq!(expansion.surface(), BindingMetadataSurface::Wasm32);
    }
}
