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
