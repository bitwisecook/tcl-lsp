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
