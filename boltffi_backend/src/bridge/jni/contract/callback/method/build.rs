use crate::{
    bridge::{
        c::{self, Identifier},
        jni::{CallbackArgument, ClosureRegistration, JvmMethodReturn},
    },
    core::{Error, Result},
};

use super::CallbackMethod;

const JNI_BRIDGE: &str = "jni";

impl CallbackMethod {
    pub(in crate::bridge::jni::contract::callback) fn from_slot(
        stem: &str,
        slot: &c::CallbackSlot,
        callbacks: &[c::Callback],
        closures: &[ClosureRegistration],
    ) -> Result<Self> {
        let Some(c::Type::Uint64) = slot.parameters().first().map(c::Parameter::ty) else {
            return Err(Error::BrokenBridgeContract {
                bridge: JNI_BRIDGE,
                invariant: "callback vtable slot does not start with a uint64 handle",
            });
        };
        let returns = JvmMethodReturn::from_c_type(slot.returns())?;
        let arguments = slot
            .parameter_groups()
            .iter()
            .map(|group| CallbackArgument::from_group(slot, group, callbacks, closures))
            .collect::<Result<Vec<_>>>()?;
        let signature = format!(
            "({}){}",
            arguments
                .iter()
                .map(CallbackArgument::jni_signature)
                .collect::<Vec<_>>()
                .join(""),
            returns.signature()
        );
        Ok(Self {
            function: Identifier::parse(format!("{stem}_{}", slot.name()))?,
            method: slot.name().clone(),
            method_id: Identifier::parse(format!("g_{stem}_{}_method", slot.name()))?,
            signature,
            returns,
            arguments,
        })
    }
}
