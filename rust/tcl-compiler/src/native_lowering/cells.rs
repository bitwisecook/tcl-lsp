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

//! Variable cells: the cell-storage lattice (plan §3.4) and the native
//! *shadow* of a cell's value that lets values stay native between the
//! statements that write and read them.
//!
//! A top-level script keeps every variable as a named runtime cell that is
//! written at the statement defining it (a hosted module must leave its
//! globals observable). What the native tier elides is the *read back*: when
//! no trace, no invocation, and no other observer can reach the cell between
//! its write and a later read, the later read reuses the NLIR value that was
//! written. That value is the cell's shadow.
//!
//! Shadows are tracked per block. They flow along an edge only when the
//! successor has exactly that one predecessor and is not a loop header, so a
//! join or a back edge never sees a shadow from just one of its paths.

use std::collections::BTreeMap;

use super::ir::NativeValueId;

/// One Tcl variable cell addressed by name.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum CellPlace {
    /// A scalar variable (or a whole array, when written as one).
    Named {
        /// The exact variable name.
        name: String,
    },
    /// One element of an array with a literal key.
    Element {
        /// The array name.
        name: String,
        /// The literal element key.
        key: String,
    },
}

impl CellPlace {
    /// The base variable name the place belongs to.
    #[must_use]
    pub fn base(&self) -> &str {
        match self {
            Self::Named { name } | Self::Element { name, .. } => name,
        }
    }

    /// The name as Tcl spells it (`a` or `a(k)`).
    #[must_use]
    pub fn spelling(&self) -> String {
        match self {
            Self::Named { name } => name.clone(),
            Self::Element { name, key } => format!("{name}({key})"),
        }
    }
}

/// Where a cell lives — the cell-storage lattice element decided for a
/// function's variable.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CellStorage {
    /// No runtime cell: the value lives only in NLIR values.
    Register,
    /// An indexed runtime slot whose name is bound lazily.
    Slot(u32),
    /// A named runtime cell; traces and introspection see it.
    Cell,
    /// A cell linked to another frame's cell (`upvar`/`global` target).
    Linked,
}

impl CellStorage {
    /// Stable Explorer spelling.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Register => "register",
            Self::Slot(_) => "slot",
            Self::Cell => "cell",
            Self::Linked => "linked",
        }
    }
}

/// The native shadows live in one block.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ShadowState {
    shadows: BTreeMap<CellPlace, NativeValueId>,
}

impl ShadowState {
    /// The value a read of `place` may reuse instead of reading the cell.
    #[must_use]
    pub fn read(&self, place: &CellPlace) -> Option<NativeValueId> {
        self.shadows.get(place).copied()
    }

    /// Record that `place` now holds `value`.
    ///
    /// A whole-variable write invalidates every element shadow of the same
    /// base name, and an element write invalidates the whole-variable shadow.
    pub fn write(&mut self, place: CellPlace, value: NativeValueId) {
        let base = place.base().to_owned();
        self.shadows
            .retain(|shadowed, _| shadowed.base() != base || shadowed == &place);
        self.shadows.insert(place, value);
    }

    /// Forget every shadow of `base` and its elements.
    pub fn forget_base(&mut self, base: &str) {
        self.shadows.retain(|shadowed, _| shadowed.base() != base);
    }

    /// Forget every shadow: an observer may have reached any cell.
    pub fn clobber(&mut self) {
        self.shadows.clear();
    }

    /// Whether no shadow is held.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.shadows.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn element_and_whole_shadows_invalidate_each_other() {
        let mut state = ShadowState::default();
        let whole = CellPlace::Named {
            name: "a".to_owned(),
        };
        let element = CellPlace::Element {
            name: "a".to_owned(),
            key: "k".to_owned(),
        };
        state.write(whole.clone(), NativeValueId(1));
        assert_eq!(state.read(&whole), Some(NativeValueId(1)));
        state.write(element.clone(), NativeValueId(2));
        assert_eq!(state.read(&whole), None);
        assert_eq!(state.read(&element), Some(NativeValueId(2)));
        state.write(whole.clone(), NativeValueId(3));
        assert_eq!(state.read(&element), None);
        state.forget_base("a");
        assert!(state.is_empty());
    }

    #[test]
    fn places_spell_themselves_as_tcl_does() {
        assert_eq!(
            CellPlace::Element {
                name: "a".to_owned(),
                key: "k".to_owned()
            }
            .spelling(),
            "a(k)"
        );
        assert_eq!(CellStorage::Slot(3).as_str(), "slot");
    }
}
