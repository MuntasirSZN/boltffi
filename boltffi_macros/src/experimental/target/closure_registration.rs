use boltffi_binding::{ImportSymbol, Native, NativeSymbol, Surface, Wasm32, native, wasm32};

use crate::experimental::error::Error;

/// A render lane for a foreign-provided closure registering with Rust.
pub enum IncomingClosureLane<'a> {
    /// Invoke function pointer, context pointer, and release function.
    InvokeContextRelease,
    /// Handle backed by imported invoke and release functions.
    HandleImports {
        /// Import Rust calls to invoke the closure.
        call: &'a ImportSymbol,
        /// Import Rust calls when the closure handle is released.
        free: &'a ImportSymbol,
    },
}

/// A render lane for a Rust-provided closure registering with foreign code.
pub enum OutgoingClosureLane<'a> {
    /// Invoke function pointer, context pointer, and release function.
    InvokeContextRelease,
    /// Handle backed by exported invoke and release functions.
    HandleExports {
        /// Export foreign code calls to invoke the closure.
        call: &'a NativeSymbol,
        /// Export foreign code calls when releasing the closure handle.
        free: &'a NativeSymbol,
    },
}

/// How inline closures cross on a surface.
///
/// A closure crosses by registering an invocation surface with the other
/// side. The IR records one registration value per direction; this trait
/// resolves each value to the render lane the wrapper emits.
pub trait ClosureCrossings: Surface {
    /// Resolves a foreign-provided closure registration to its render lane.
    fn incoming_closure_lane(
        registration: &Self::IncomingClosureRegistration,
    ) -> Result<IncomingClosureLane<'_>, Error>;

    /// Resolves a Rust-provided closure registration to its render lane.
    fn outgoing_closure_lane(
        registration: &Self::OutgoingClosureRegistration,
    ) -> Result<OutgoingClosureLane<'_>, Error>;
}

impl ClosureCrossings for Native {
    fn incoming_closure_lane(
        registration: &native::ClosureRegistration,
    ) -> Result<IncomingClosureLane<'_>, Error> {
        match registration {
            native::ClosureRegistration::InvokeContextRelease => {
                Ok(IncomingClosureLane::InvokeContextRelease)
            }
            _ => Err(Error::UnsupportedExpansion(
                "unknown native closure registration",
            )),
        }
    }

    fn outgoing_closure_lane(
        registration: &native::ClosureRegistration,
    ) -> Result<OutgoingClosureLane<'_>, Error> {
        match registration {
            native::ClosureRegistration::InvokeContextRelease => {
                Ok(OutgoingClosureLane::InvokeContextRelease)
            }
            _ => Err(Error::UnsupportedExpansion(
                "unknown native closure return registration",
            )),
        }
    }
}

impl ClosureCrossings for Wasm32 {
    fn incoming_closure_lane(
        registration: &wasm32::IncomingClosureRegistration,
    ) -> Result<IncomingClosureLane<'_>, Error> {
        Ok(IncomingClosureLane::HandleImports {
            call: registration.call(),
            free: registration.free(),
        })
    }

    fn outgoing_closure_lane(
        registration: &wasm32::OutgoingClosureRegistration,
    ) -> Result<OutgoingClosureLane<'_>, Error> {
        Ok(OutgoingClosureLane::HandleExports {
            call: registration.call(),
            free: registration.free(),
        })
    }
}
