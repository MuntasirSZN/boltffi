/// 3: byte buffers returned from an exported callable cross unframed. A host
/// built for 2 reads the length prefix it expects and hands back four bytes of
/// header as payload, which nothing else would catch.
pub const WASM_ABI_VERSION: u32 = 3;

#[cfg(any(test, target_arch = "wasm32"))]
use std::alloc::{Layout, alloc, dealloc};
#[cfg(target_arch = "wasm32")]
use std::mem;
#[cfg(any(test, target_arch = "wasm32"))]
use std::ptr;

#[cfg(target_arch = "wasm32")]
#[repr(C)]
pub struct WasmCallbackOutBuf {
    ptr: u32,
    len: u32,
    cap: u32,
}

#[cfg(target_arch = "wasm32")]
impl WasmCallbackOutBuf {
    pub const fn empty() -> Self {
        Self {
            ptr: 0,
            len: 0,
            cap: 0,
        }
    }

    pub unsafe fn as_slice(&self) -> &[u8] {
        if self.ptr == 0 || self.len == 0 {
            &[]
        } else {
            unsafe { core::slice::from_raw_parts(self.ptr as *const u8, self.len as usize) }
        }
    }
}

#[cfg(target_arch = "wasm32")]
impl Drop for WasmCallbackOutBuf {
    fn drop(&mut self) {
        if self.ptr != 0 && self.cap > 0 {
            boltffi_wasm_free_impl(self.ptr as *mut u8, self.cap as usize);
        }
    }
}

#[cfg(any(test, target_arch = "wasm32"))]
fn boltffi_wasm_alloc_impl(size: usize) -> *mut u8 {
    if size == 0 {
        return ptr::null_mut();
    }

    let layout = match Layout::from_size_align(size, 8) {
        Ok(layout) => layout,
        Err(_) => return ptr::null_mut(),
    };

    unsafe { alloc(layout) }
}

#[cfg(any(test, target_arch = "wasm32"))]
pub(crate) fn boltffi_wasm_free_impl(pointer: *mut u8, size: usize) {
    if pointer.is_null() || size == 0 {
        return;
    }

    let layout = match Layout::from_size_align(size, 8) {
        Ok(layout) => layout,
        Err(_) => return,
    };

    unsafe { dealloc(pointer, layout) };
}

#[cfg(any(test, target_arch = "wasm32"))]
pub(crate) fn boltffi_wasm_free_buf_impl(pointer: *mut u8, size: usize, align: usize) {
    if pointer.is_null() || size == 0 || align == 0 {
        return;
    }
    let layout = match Layout::from_size_align(size, align) {
        Ok(layout) => layout,
        Err(_) => return,
    };
    unsafe { dealloc(pointer, layout) };
}

#[cfg(any(test, target_arch = "wasm32"))]
pub(crate) unsafe fn boltffi_wasm_free_string_return_impl(pointer: *mut u8, len: usize) {
    if pointer.is_null() || len == 0 {
        return;
    }
    unsafe { drop(Vec::from_raw_parts(pointer, len, len)) };
}

#[cfg(any(test, target_arch = "wasm32"))]
fn boltffi_wasm_alloc_owned_bytes_impl(len: usize) -> *mut u8 {
    if len == 0 {
        return ptr::null_mut();
    }
    let layout = match Layout::array::<u8>(len) {
        Ok(layout) => layout,
        Err(_) => return ptr::null_mut(),
    };
    unsafe { alloc(layout) }
}

#[cfg(any(test, target_arch = "wasm32"))]
fn boltffi_wasm_realloc_impl(pointer: *mut u8, old_size: usize, new_size: usize) -> *mut u8 {
    if new_size == 0 {
        boltffi_wasm_free_impl(pointer, old_size);
        return ptr::null_mut();
    }

    if pointer.is_null() {
        return boltffi_wasm_alloc_impl(new_size);
    }

    let old_layout = match Layout::from_size_align(old_size, 8) {
        Ok(layout) => layout,
        Err(_) => return ptr::null_mut(),
    };

    unsafe { std::alloc::realloc(pointer, old_layout, new_size) }
}

#[cfg(target_arch = "wasm32")]
static mut RETURN_SLOT: [u32; 4] = [0, 0, 0, 0];

#[cfg(target_arch = "wasm32")]
#[inline(always)]
pub fn write_return_slot(ptr: u32, len: u32, cap: u32, align: u32) {
    unsafe {
        core::ptr::write_volatile(&raw mut RETURN_SLOT[0], ptr);
        core::ptr::write_volatile(&raw mut RETURN_SLOT[1], len);
        core::ptr::write_volatile(&raw mut RETURN_SLOT[2], cap);
        core::ptr::write_volatile(&raw mut RETURN_SLOT[3], align);
    }
}

#[cfg(target_arch = "wasm32")]
#[inline(always)]
pub fn write_option_f64_presence(present: bool) {
    unsafe { core::ptr::write_volatile(&raw mut RETURN_SLOT[0], u32::from(present)) }
}

#[cfg(target_arch = "wasm32")]
pub unsafe fn take_return_slot_vec<T>() -> Vec<T> {
    let pointer = unsafe { core::ptr::read_volatile(&raw const RETURN_SLOT[0]) };
    let length = unsafe { core::ptr::read_volatile(&raw const RETURN_SLOT[1]) };
    let capacity = unsafe { core::ptr::read_volatile(&raw const RETURN_SLOT[2]) };
    let alignment = unsafe { core::ptr::read_volatile(&raw const RETURN_SLOT[3]) };
    write_return_slot(0, 0, 0, 0);
    if pointer == 0 || length == 0 {
        return Vec::new();
    }
    debug_assert_eq!(alignment as usize, mem::align_of::<T>());
    debug_assert_eq!((length as usize) % mem::size_of::<T>(), 0);
    debug_assert_eq!((capacity as usize) % mem::size_of::<T>(), 0);
    unsafe {
        Vec::from_raw_parts(
            pointer as *mut T,
            length as usize / mem::size_of::<T>(),
            capacity as usize / mem::size_of::<T>(),
        )
    }
}

#[cfg(target_arch = "wasm32")]
#[inline(always)]
pub fn return_slot_addr() -> u32 {
    (&raw const RETURN_SLOT) as u32
}

#[cfg(target_arch = "wasm32")]
pub unsafe fn take_packed_utf8_string(packed: u64) -> String {
    let bytes = unsafe { take_packed_bytes(packed) };
    unsafe { String::from_utf8_unchecked(bytes) }
}

#[cfg(target_arch = "wasm32")]
pub unsafe fn take_packed_bytes(packed: u64) -> Vec<u8> {
    if packed == 0 {
        return Vec::new();
    }

    let pointer = (packed as u32) as usize;
    let length = ((packed >> 32) as u32) as usize;
    unsafe { Vec::from_raw_parts(pointer as *mut u8, length, length) }
}

#[cfg(target_arch = "wasm32")]
mod exports {
    #[unsafe(no_mangle)]
    pub extern "C" fn boltffi_wasm_abi_version() -> u32 {
        super::WASM_ABI_VERSION
    }

    #[unsafe(no_mangle)]
    pub extern "C" fn boltffi_wasm_return_slot_addr() -> u32 {
        super::return_slot_addr()
    }

    #[unsafe(no_mangle)]
    pub extern "C" fn boltffi_wasm_alloc(size: u32) -> u32 {
        super::boltffi_wasm_alloc_impl(size as usize) as u32
    }

    #[unsafe(no_mangle)]
    pub extern "C" fn boltffi_wasm_free(ptr: u32, size: u32) {
        super::boltffi_wasm_free_impl(ptr as *mut u8, size as usize);
    }

    #[unsafe(no_mangle)]
    pub extern "C" fn boltffi_wasm_realloc(ptr: u32, old_size: u32, new_size: u32) -> u32 {
        super::boltffi_wasm_realloc_impl(ptr as *mut u8, old_size as usize, new_size as usize)
            as u32
    }

    #[unsafe(no_mangle)]
    pub unsafe extern "C" fn boltffi_wasm_free_string_return(ptr: u32, len: u32) {
        unsafe { super::boltffi_wasm_free_string_return_impl(ptr as *mut u8, len as usize) };
    }

    #[unsafe(no_mangle)]
    pub extern "C" fn boltffi_wasm_alloc_owned_bytes(len: u32) -> u32 {
        super::boltffi_wasm_alloc_owned_bytes_impl(len as usize) as u32
    }

    #[unsafe(no_mangle)]
    pub extern "C" fn boltffi_wasm_free_buf(ptr: u32, size: u32, align: u32) {
        super::boltffi_wasm_free_buf_impl(ptr as *mut u8, size as usize, align as usize);
    }
}

#[cfg(test)]
mod tests {
    use std::ptr::{self, NonNull};

    use super::{
        WASM_ABI_VERSION, boltffi_wasm_alloc_impl, boltffi_wasm_alloc_owned_bytes_impl,
        boltffi_wasm_free_buf_impl, boltffi_wasm_free_impl, boltffi_wasm_free_string_return_impl,
        boltffi_wasm_realloc_impl,
    };

    #[test]
    fn wasm_abi_version_is_stable() {
        assert_eq!(WASM_ABI_VERSION, 3);
    }

    #[test]
    fn alloc_zero_returns_zero_pointer() {
        assert!(boltffi_wasm_alloc_impl(0).is_null());
    }

    #[test]
    fn alloc_non_zero_returns_pointer_and_free_accepts_it() {
        let pointer = boltffi_wasm_alloc_impl(16);
        assert!(!pointer.is_null());
        boltffi_wasm_free_impl(pointer, 16);
    }

    #[test]
    fn realloc_from_null_matches_alloc_behavior() {
        let pointer = boltffi_wasm_realloc_impl(ptr::null_mut(), 0, 24);
        assert!(!pointer.is_null());
        boltffi_wasm_free_impl(pointer, 24);
    }

    #[test]
    fn realloc_growth_preserves_prefix_bytes() {
        let old_size = 32usize;
        let new_size = 96usize;
        let old_pointer = boltffi_wasm_alloc_impl(old_size);
        assert!(!old_pointer.is_null());

        let expected_prefix = (0..old_size)
            .map(|index| ((index * 7 + 3) % 256) as u8)
            .collect::<Vec<_>>();

        unsafe {
            std::slice::from_raw_parts_mut(old_pointer, old_size).copy_from_slice(&expected_prefix);
        }

        let new_pointer = boltffi_wasm_realloc_impl(old_pointer, old_size, new_size);
        assert!(!new_pointer.is_null());

        let actual_prefix = unsafe { std::slice::from_raw_parts(new_pointer, old_size) }.to_vec();
        assert_eq!(actual_prefix, expected_prefix);

        boltffi_wasm_free_impl(new_pointer, new_size);
    }

    #[test]
    fn realloc_to_zero_returns_null_pointer() {
        let pointer = boltffi_wasm_alloc_impl(64);
        assert!(!pointer.is_null());

        let reallocated = boltffi_wasm_realloc_impl(pointer, 64, 0);
        assert!(reallocated.is_null());
    }

    #[test]
    fn free_ignores_zero_inputs() {
        boltffi_wasm_free_impl(ptr::null_mut(), 32);
        boltffi_wasm_free_impl(NonNull::dangling().as_ptr(), 0);
    }

    #[test]
    fn free_buf_releases_with_explicit_alignment() {
        let pointer = boltffi_wasm_alloc_impl(64);
        assert!(!pointer.is_null());
        boltffi_wasm_free_buf_impl(pointer, 64, 8);
    }

    #[test]
    fn free_string_return_releases_owned_buffer() {
        let text = "boltffi";
        let bytes = text.as_bytes().to_vec().into_boxed_slice();
        let len = bytes.len();
        let pointer = Box::into_raw(bytes).cast::<u8>();
        unsafe { boltffi_wasm_free_string_return_impl(pointer, len) };
    }

    #[test]
    fn owned_byte_allocation_matches_string_return_release() {
        let len = 16;
        let pointer = boltffi_wasm_alloc_owned_bytes_impl(len);
        assert!(!pointer.is_null());
        unsafe { core::ptr::write_bytes(pointer, 0, len) };
        unsafe { boltffi_wasm_free_string_return_impl(pointer, len) };
    }
}
