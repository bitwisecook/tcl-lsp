// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program.  If not, see <https://www.gnu.org/licenses/>.
//
// SPDX-License-Identifier: AGPL-3.0-or-later

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
