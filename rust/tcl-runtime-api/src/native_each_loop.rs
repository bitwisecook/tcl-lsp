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

//! Generic native foreach/lmap schedules, independent of compiler selection.

use tcl_dialect::TclVersion;

/// The selected registered implementation, independent of its written name.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NativeEachLoopKind {
    Foreach,
    Lmap,
}
impl NativeEachLoopKind {
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Foreach => "foreach",
            Self::Lmap => "lmap",
        }
    }
}

/// Physical generic-command recipe. A pure recipe grants no engine authority.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NativeEachLoopRecipe {
    C(TclVersion),
    Jim084,
}
impl NativeEachLoopRecipe {
    #[must_use]
    pub const fn copies_headers(self) -> bool {
        matches!(self, Self::C(version) if !matches!(version, TclVersion::V8_4))
    }
    #[must_use]
    pub const fn refetches_groups(self) -> bool {
        matches!(self, Self::C(TclVersion::V8_4))
    }
    #[must_use]
    pub const fn live_iterators(self) -> bool {
        matches!(self, Self::Jim084)
    }
    #[must_use]
    pub const fn pins_assignment_value(self) -> bool {
        matches!(self, Self::C(TclVersion::V8_4))
    }
    #[must_use]
    pub const fn variables_first(self) -> bool {
        matches!(self, Self::Jim084)
    }
    #[must_use]
    pub const fn empty_lmap_publishes_list(self) -> bool {
        matches!(self, Self::Jim084)
    }
}
