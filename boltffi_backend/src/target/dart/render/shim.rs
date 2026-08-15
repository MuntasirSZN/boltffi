//! Symbol names for Dart dual-path stubs. The stubs themselves are emitted
//! by `boltffi_macros` into the user crate (`cfg(boltffi_dart)`), not as
//! generated Rust source from this backend.

pub(crate) fn shim_prefix(trait_name: &str) -> String {
    format!("BoltFFIDartShim_{trait_name}")
}

pub(crate) fn method_symbol(trait_name: &str, method: &str) -> String {
    format!("{}_{method}", shim_prefix(trait_name))
}

pub(crate) fn register_symbol(trait_name: &str) -> String {
    format!("{}_register", shim_prefix(trait_name))
}

pub(crate) fn release_symbol(trait_name: &str) -> String {
    format!("{}_release", shim_prefix(trait_name))
}
