//! Root JNI bridge contract.
//!
//! A `JniBridgeContract` is the complete plan for one generated JNI C file. It
//! names the JVM owner class, the C header to include, the lifecycle hooks, and
//! every native method, callback registration, closure registration, stream
//! helper, and async completion invoker that must be emitted.
//!
//! This is the only place where the JNI bridge walks the whole C bridge
//! contract. After construction, lower modules work from these typed pieces, so
//! templates never need to rediscover which declarations exist or how they fit
//! together.

mod build;

use boltffi_binding::Native;

use crate::{
    bridge::{
        c::{HeaderInclude, Identifier},
        jni::JvmClassPath,
    },
    core::{BridgeCapabilities, BridgeContract, FilePath, contract::sealed},
};

use super::{
    CallbackCompletionInvoker, CallbackRegistration, ClosureRegistration, NativeMethod,
    StreamProtocolMethods,
};

/// Contract produced by the JNI bridge layer.
#[derive(Clone, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub struct JniBridgeContract {
    capabilities: BridgeCapabilities,
    class: JvmClassPath,
    source_path: FilePath,
    c_header: HeaderInclude,
    free_buffer: Identifier,
    callbacks: Vec<CallbackRegistration>,
    callback_completions: Vec<CallbackCompletionInvoker>,
    closures: Vec<ClosureRegistration>,
    methods: Vec<NativeMethod>,
    streams: Vec<StreamProtocolMethods>,
}

impl JniBridgeContract {
    /// Returns the JVM class that owns generated native methods.
    pub fn class(&self) -> &JvmClassPath {
        &self.class
    }

    /// Returns the generated JNI source path.
    pub fn source_path(&self) -> &FilePath {
        &self.source_path
    }

    /// Returns the C header include path used by the JNI source.
    pub fn c_header(&self) -> &HeaderInclude {
        &self.c_header
    }

    /// Returns the C support function that releases owned BoltFFI byte buffers.
    pub fn free_buffer(&self) -> &Identifier {
        &self.free_buffer
    }

    /// Returns generated callback registrations.
    pub fn callbacks(&self) -> &[CallbackRegistration] {
        &self.callbacks
    }

    /// Returns async callback completion invokers.
    pub fn callback_completions(&self) -> &[CallbackCompletionInvoker] {
        &self.callback_completions
    }

    /// Returns generated closure trampoline registrations.
    pub fn closures(&self) -> &[ClosureRegistration] {
        &self.closures
    }

    /// Returns generated native methods.
    pub fn methods(&self) -> &[NativeMethod] {
        &self.methods
    }

    /// Returns generated stream protocol methods.
    pub fn streams(&self) -> &[StreamProtocolMethods] {
        &self.streams
    }
}

impl BridgeContract for JniBridgeContract {
    type Surface = Native;

    fn capabilities(&self) -> &BridgeCapabilities {
        &self.capabilities
    }
}

impl sealed::BridgeContract for JniBridgeContract {}
