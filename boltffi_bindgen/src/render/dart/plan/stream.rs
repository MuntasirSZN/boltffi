use crate::ir::StreamMode;

#[derive(Debug, Clone)]
pub struct DartStream {
    pub name: String,
    pub item_ty: super::DartType,
    pub ffi_item_ty: super::DartNativeType,
    pub ffi_item_size: Option<usize>,
    pub subscribe_fn: super::DartNativeFunction,
    pub poll_fn: super::DartNativeFunction,
    pub pop_batch_fn: super::DartNativeFunction,
    pub wait_fn: super::DartNativeFunction,
    pub unsubscribe_fn: super::DartNativeFunction,
    pub free_fn: super::DartNativeFunction,
    pub mode: StreamMode,
}
