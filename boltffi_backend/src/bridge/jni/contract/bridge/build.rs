//! Crate-wide build pass for the JNI bridge contract.
//!
//! JNI glue is one generated C file, but its pieces are connected. A native
//! method can use a closure signature that is also used by a callback method. A
//! callback can need an async completion helper. Stream protocol functions must
//! not also appear as normal native methods.
//!
//! This module performs that whole-file pass once. It reads the lower C bridge
//! contract, registers shared closure signatures, builds callback registrations,
//! filters stream protocol functions out of the native method list, and records
//! the support symbols the final source file needs.

use std::collections::BTreeSet;

use crate::{
    bridge::{
        c::{self, HeaderInclude, Identifier},
        jni::JvmClassPath,
    },
    core::{BridgeCapability, BridgeContract, FilePath, Result},
};

use super::{
    CallbackCompletionInvoker, CallbackRegistration, ClosureRegistration, JniBridgeContract,
    NativeMethod, StreamProtocolMethods,
};

impl JniBridgeContract {
    /// Builds the JNI bridge contract from the C bridge contract.
    pub fn from_c_bridge(
        class: JvmClassPath,
        source_path: FilePath,
        c_bridge: &c::CBridgeContract,
    ) -> Result<Self> {
        let closures =
            ClosureRegistration::from_c_bridge(&class, c_bridge.functions(), c_bridge.callbacks())?;
        let callbacks = c_bridge
            .callbacks()
            .iter()
            .map(|callback| {
                CallbackRegistration::from_c_callback(
                    &class,
                    callback,
                    c_bridge.callbacks(),
                    &closures,
                )
            })
            .collect::<Result<Vec<_>>>()?;
        let callback_completions = CallbackCompletionInvoker::from_callbacks(&class, &callbacks)?;
        let stream_function_names = c_bridge
            .streams()
            .iter()
            .flat_map(c::Stream::functions)
            .map(|function| function.name().to_owned())
            .collect::<BTreeSet<_>>();
        let methods = c_bridge
            .functions()
            .iter()
            .filter(|function| !stream_function_names.contains(function.name()))
            .map(|function| NativeMethod::new(&class, function, c_bridge.callbacks(), &closures))
            .collect::<Result<Vec<_>>>()?;
        let streams = c_bridge
            .streams()
            .iter()
            .map(|stream| {
                StreamProtocolMethods::from_c_stream(
                    &class,
                    stream,
                    c_bridge.callbacks(),
                    &closures,
                )
            })
            .collect::<Result<Vec<_>>>()?;
        Ok(Self {
            capabilities: c_bridge
                .capabilities()
                .clone()
                .stable(BridgeCapability::Jni),
            c_header: HeaderInclude::from_files(&source_path, c_bridge.header_path())?,
            free_buffer: Identifier::parse(c_bridge.support().buffer_free()?.name())?,
            callbacks,
            callback_completions,
            methods,
            streams,
            closures,
            class,
            source_path,
        })
    }
}
