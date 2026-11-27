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

//! Authentic generic each-loop command selection.

use tcl_dialect::TclVersion;
use tcl_dialect::model::{Family, Release};
use tcl_runtime_api::native_each_loop::{NativeEachLoopKind, NativeEachLoopRecipe};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
/// Selected native loop recipe with its independently retained command kind.
pub struct NativeEachLoopProtocol {
    recipe: NativeEachLoopRecipe,
    kind: NativeEachLoopKind,
}
impl NativeEachLoopProtocol {
    #[must_use]
    /// Physical iteration and ownership recipe for the selected engine.
    pub const fn recipe(self) -> NativeEachLoopRecipe {
        self.recipe
    }
    #[must_use]
    /// Result collection policy of the actual selected loop command.
    pub const fn kind(self) -> NativeEachLoopKind {
        self.kind
    }
}
impl crate::InvocationDialect {
    /// Select the actual generic registered implementation's runtime recipe.
    #[must_use]
    pub fn native_each_loop_protocol(
        self,
        kind: NativeEachLoopKind,
    ) -> Option<NativeEachLoopProtocol> {
        let recipe = match self.family()? {
            Family::Tcl => {
                let version = self.tcl_version?;
                if kind == NativeEachLoopKind::Lmap
                    && matches!(version, TclVersion::V8_4 | TclVersion::V8_5)
                {
                    return None;
                }
                NativeEachLoopRecipe::C(version)
            }
            Family::Jim if self.core_point?.release() == Release::JIM_0_84 => {
                NativeEachLoopRecipe::Jim084
            }
            _ => return None,
        };
        Some(NativeEachLoopProtocol { recipe, kind })
    }
}
