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

//! Actual alias stored roles, independent of command observation allocations.

use crate::obj::{self, NativeObjectLifetime, Owned, TclObj};
use std::cell::RefCell;
use tcl_syntax::value::ValueError;

/// An original alias header observation with one separately withdrawable role.
/// Rust command views retain only this allocation, not another Native reference.
pub struct NativeAliasHeader {
    allocation: NativeObjectLifetime,
    role: RefCell<Option<Owned>>,
}
impl NativeAliasHeader {
    pub(crate) fn new(role: Owned) -> Self {
        Self {
            allocation: NativeObjectLifetime::retain(role.as_ptr()),
            role: RefCell::new(Some(role)),
        }
    }
    pub(crate) fn checked_ptr(&self) -> Result<*mut TclObj, ValueError> {
        if self.role.borrow().is_none() {
            return Err(ValueError::CommandProtocolUnavailable(
                "original alias stored role retired",
            ));
        }
        obj::check_native_liveness(self.allocation.as_ptr())?;
        Ok(self.allocation.as_ptr())
    }
    /// Extract the actual role; its callback-bearing release is outside this borrow.
    pub(super) fn take_role(&self) -> Option<Owned> {
        self.role.borrow_mut().take()
    }
    #[cfg(test)]
    pub(crate) fn as_ptr(&self) -> *mut TclObj {
        self.allocation.as_ptr()
    }
}
