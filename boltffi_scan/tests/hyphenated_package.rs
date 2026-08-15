//! A package name is not a module name, and ids are written with the latter.
//!
//! Every fixture in the unit suites is named `demo`, which has only one
//! spelling, so the two never disagree there. These go through `scan_package`,
//! which is where the root name reaches visible-path discovery.

use std::fs;
use std::path::PathBuf;

use boltffi_ast::{PackageInfo, PathRoot};
use boltffi_scan::{ScanInput, scan_package};

const SOURCE: &str = r#"
pub mod api {
    #[data]
    #[derive(Clone, Copy, Debug, Default)]
    pub struct Info { pub a: u32 }

    #[export]
    pub fn nested(value: Info) -> u32 { value.a }
}
"#;

/// `case` keeps concurrent tests off each other's files: the scratch directory
/// is shared between them, and two writers of the same path race.
fn scan(case: &str, package: &str) -> boltffi_scan::PackageScan {
    let directory = PathBuf::from(env!("CARGO_TARGET_TMPDIR"))
        .join(case)
        .join(package);
    fs::create_dir_all(&directory).expect("scratch directory");
    let source = directory.join("lib.rs");
    fs::write(&source, SOURCE).expect("source file");

    scan_package(&ScanInput::new(&source, PackageInfo::new(package, None))).expect("package scans")
}

#[test]
fn nested_items_stay_crate_rooted_under_a_hyphenated_package() {
    for package in ["my-root", "myroot"] {
        let scanned = scan("nested", package);
        let paths = scanned.root_visible_paths().collect::<Vec<_>>();

        let (_, path) = paths
            .iter()
            .find(|(id, _)| id.ends_with("::Info"))
            .unwrap_or_else(|| panic!("`{package}` should expose a visible path for `Info`"));

        // Relative here means the item was read as belonging to some other
        // crate, and the expander would emit `Info` with nothing in front.
        assert_eq!(
            path.root,
            PathRoot::Crate,
            "`{package}` should reach its own nested item through `crate`",
        );
        assert_eq!(
            path.segments
                .iter()
                .map(|segment| segment.name.as_str())
                .collect::<Vec<_>>(),
            ["api", "Info"],
        );
    }
}

#[test]
fn declaration_ids_use_the_module_name() {
    let ids = scan("ids", "my-root")
        .root()
        .records
        .iter()
        .map(|record| record.id.as_str().to_owned())
        .collect::<Vec<_>>();

    assert_eq!(ids, ["my_root::api::Info"]);
}
