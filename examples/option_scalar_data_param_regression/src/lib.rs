//! Regression coverage for https://github.com/boltffi/boltffi/issues/621:
//! an `#[export]` function taking a `#[data]` enum with a data-carrying
//! variant (named-field or tuple) and returning `Option<{integer}>` failed
//! to compile with `expected FfiBuf, found f64`.
//!
//! This crate is intentionally excluded from the main workspace (see
//! `Cargo.toml`'s root `exclude` list) so it exercises the plain,
//! non-dogfooded `#[export]`/`#[data]` expansion path that ordinary
//! consumers hit — `boltffi_tests` (the in-workspace test crate) routes
//! exports through a separate build-script-driven IR expansion and does
//! not exercise this code path at all.

use boltffi::*;

#[derive(Clone)]
#[data]
pub enum Signal {
    Idle,
    Pending(u8),
}

#[export]
pub fn maybe_count(signal: Signal) -> Option<u8> {
    match signal {
        Signal::Idle => None,
        Signal::Pending(count) => Some(count),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use boltffi::__private::FfiBuf;
    use boltffi::__private::wire::{WireDecode, WireEncode};

    fn decode_buf<T: WireDecode>(buf: FfiBuf) -> T {
        let (value, _) = T::decode_from(unsafe { buf.as_byte_slice() }).unwrap();
        value
    }

    fn with_encoded<T: WireEncode, R>(value: &T, call: impl FnOnce(*const u8, usize) -> R) -> R {
        let buf = FfiBuf::wire_encode(value);
        call(buf.as_ptr(), buf.len())
    }

    /// See https://github.com/boltffi/boltffi/issues/621.
    #[test]
    fn data_enum_param_with_scalar_option_return_crosses() {
        let some = with_encoded(&Signal::Pending(7), |ptr, len| unsafe {
            boltffi_maybe_count(ptr, len)
        });
        assert_eq!(decode_buf::<Option<u8>>(some), Some(7));

        let none = with_encoded(&Signal::Idle, |ptr, len| unsafe {
            boltffi_maybe_count(ptr, len)
        });
        assert_eq!(decode_buf::<Option<u8>>(none), None);
    }
}
