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

//! Value types for the typed BPF-IR. Deliberately fixed-width — there is no
//! dynamic Tcl shimmering here.

/// A bounded memory region a pointer can refer to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Region {
    /// The program context / packet buffer (the socket-filter `__sk_buff` data).
    Ctx,
}

/// A value type in the BPF-IR.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Ty {
    /// A 64-bit integer. In v1 every scalar verb (`setint`/`seti32`/`setu32`,
    /// the `load*` results, `pktlen`) lands here; sub-64-bit width fidelity is
    /// a follow-on.
    Int,
    /// A pointer into a bounded [`Region`].
    Ptr(Region),
}

impl Ty {
    /// Whether this type may be used as an arithmetic / comparison operand.
    #[must_use]
    pub fn is_int(self) -> bool {
        matches!(self, Ty::Int)
    }
}

/// The width of a packet load.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Width {
    /// 8-bit (1 byte).
    B8,
    /// 16-bit (2 bytes).
    B16,
    /// 32-bit (4 bytes).
    B32,
}
