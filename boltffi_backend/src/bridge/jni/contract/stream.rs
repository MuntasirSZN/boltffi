use crate::{
    bridge::{
        c::{self, TypeFragment},
        jni::{ClosureRegistration, JniSymbolName, JvmClassPath, NativeMethod},
    },
    core::{Error, Result},
};

const JNI_BRIDGE: &str = "jni";

#[derive(Clone, Debug, Eq, PartialEq)]
#[non_exhaustive]
/// JNI methods generated for one C stream protocol.
pub struct StreamProtocolMethods {
    methods: Vec<NativeMethod>,
    direct_batches: Vec<DirectStreamBatchMethod>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
#[non_exhaustive]
/// JNI method that returns a direct stream batch as a byte array.
pub struct DirectStreamBatchMethod {
    symbol: JniSymbolName,
    c_function: c::Function,
    subscription_type: TypeFragment,
    item_type: TypeFragment,
    item_size: u64,
}

impl StreamProtocolMethods {
    /// Creates JNI stream methods from a C stream protocol.
    pub fn from_c_stream(
        class: &JvmClassPath,
        stream: &c::Stream,
        callbacks: &[c::Callback],
        closures: &[ClosureRegistration],
    ) -> Result<Self> {
        let direct_batch_name = stream
            .direct_batch()
            .map(|batch| batch.function().name().to_owned());
        let methods = stream
            .functions()
            .into_iter()
            .filter(|function| Some(function.name()) != direct_batch_name.as_deref())
            .map(|function| NativeMethod::new(class, function, callbacks, closures))
            .collect::<Result<Vec<_>>>()?;
        let direct_batches = stream
            .direct_batch()
            .map(|batch| DirectStreamBatchMethod::from_c_batch(class, batch))
            .transpose()?
            .into_iter()
            .collect();

        Ok(Self {
            methods,
            direct_batches,
        })
    }

    /// Returns stream protocol methods rendered by the generic JNI method path.
    pub fn methods(&self) -> &[NativeMethod] {
        &self.methods
    }

    /// Returns direct batch methods that need stream-specific JNI rendering.
    pub fn direct_batches(&self) -> &[DirectStreamBatchMethod] {
        &self.direct_batches
    }
}

impl DirectStreamBatchMethod {
    fn from_c_batch(class: &JvmClassPath, batch: &c::DirectStreamBatch) -> Result<Self> {
        let subscription =
            batch
                .function()
                .params()
                .first()
                .ok_or(Error::BrokenBridgeContract {
                    bridge: JNI_BRIDGE,
                    invariant: "direct stream batch function is missing subscription parameter",
                })?;
        Ok(Self {
            symbol: JniSymbolName::native_method(class, batch.function().name())?,
            c_function: batch.function().clone(),
            subscription_type: TypeFragment::anonymous(subscription.ty())?,
            item_type: TypeFragment::anonymous(batch.item())?,
            item_size: batch.item_size().get(),
        })
    }

    /// Returns the JNI export symbol.
    pub fn symbol(&self) -> &JniSymbolName {
        &self.symbol
    }

    /// Returns the C stream batch function.
    pub fn c_function(&self) -> &c::Function {
        &self.c_function
    }

    /// Returns the C subscription handle type.
    pub fn subscription_type(&self) -> &TypeFragment {
        &self.subscription_type
    }

    /// Returns the C type stored in the native batch buffer.
    pub fn item_type(&self) -> &TypeFragment {
        &self.item_type
    }

    /// Returns the byte size of one direct stream item.
    pub const fn item_size(&self) -> u64 {
        self.item_size
    }
}
