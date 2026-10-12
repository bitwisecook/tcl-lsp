//! Native table readers retaining an authenticated static extension lifetime.

use std::ffi::{CStr, c_char, c_void};
use std::rc::Rc;

use tcl_core_types::{NativeIndexTable, NativeIndexUnavailable};

pub(crate) struct StaticIndexTable {
    table: *const c_void,
    stride: usize,
    entries: usize,
    _extension: Rc<crate::state::StaticExtensionLifetime>,
}

impl StaticIndexTable {
    /// # Safety
    /// The actual extension promises this table and its terminated entries
    /// remain valid until all original and duplicated native caches are freed.
    pub(crate) unsafe fn new(
        table: *const c_void,
        stride: usize,
        entries: usize,
        extension: Rc<crate::state::StaticExtensionLifetime>,
    ) -> Self {
        Self {
            table,
            stride,
            entries,
            _extension: extension,
        }
    }
}

impl NativeIndexTable for StaticIndexTable {
    fn identity(&self) -> usize {
        self.table as usize
    }

    fn word(&self, index: usize, stride: usize) -> Result<Rc<[u8]>, NativeIndexUnavailable> {
        if stride != self.stride || index >= self.entries {
            return Err(NativeIndexUnavailable);
        }
        let distance = index.checked_mul(stride).ok_or(NativeIndexUnavailable)?;
        let distance = isize::try_from(distance).map_err(|_| NativeIndexUnavailable)?;
        // SAFETY: construction retains the extension's original persistent
        // table guarantee; the checked index and original stride select a slot.
        let entry = unsafe {
            self.table
                .cast::<u8>()
                .offset(distance)
                .cast::<*const c_char>()
                .read_unaligned()
        };
        if entry.is_null() {
            return Err(NativeIndexUnavailable);
        }
        // SAFETY: the extension's lifetime guarantee covers the current entry
        // and its terminator. This deliberately rereads the live table word.
        Ok(Rc::from(unsafe { CStr::from_ptr(entry) }.to_bytes()))
    }
}
