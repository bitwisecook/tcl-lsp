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

//! `TclOO` frame classification for completion and hover.
//!
//! [`OoFrame`] keeps helper-name resolution separate from method invocation
//! callability and applies the registry's command-context requirements.

use tcl_compiler::analyser::AnalysisResult;
use tcl_registry::CommandRegistry;

/// What kind of `TclOO` frame a cursor sits in — the LSP-side half of the
/// `oo::Helpers` scoping rule, and the one place the question is asked.
///
/// Two facts, because real Tcl keeps them apart:
///
/// * `resolves` — the frame's namespace path reaches `::oo::Helpers` (and
///   the object's own namespace, home of `my`), so the family's bare
///   spellings **resolve** here.
/// * `method` — the frame is a real **method invocation**, so they are also
///   **callable**.
///
/// They differ in exactly one place: a Tcl 9 class-level `initialise` /
/// `initialize` body. tclsh 9.0.4, inside
/// `oo::class create ::P { initialize { … } }`:
///
/// ```text
/// ns=::oo::Obj20  path=::oo::Helpers ::oo
/// link:  which='::oo::Helpers::link'  call -> link may only be called from inside a method
/// self:  which='::oo::Helpers::self'  call -> self may only be called from inside a method
/// my:    which='::oo::Obj20::my'      call -> OK  (`my new` returns ::oo::Obj22)
/// ```
///
/// So `W123` — "is this an unknown command" — keys on `resolves` and stays
/// in the analyser, while completion and hover ask [`Self::admits`], which
/// keys on callability. Which commands need which is registry data
/// (`Traits::TCLOO_METHOD_CONTEXT` / `Traits::TCLOO_REQUIRES_METHOD_FRAME`),
/// never a name list on this side.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct OoFrame {
    /// The bare `oo::Helpers` family resolves at this offset.
    pub(crate) resolves: bool,
    /// This offset is inside a real method invocation, so the family is
    /// callable and not merely resolvable.
    pub(crate) method: bool,
}

impl OoFrame {
    /// Classify the frame containing `byte_offset`.
    ///
    /// Both halves come from the analyser's own scope walk
    /// (`innermost_scope_reaches_oo_helpers` /
    /// `innermost_scope_is_oo_method_frame`), which share one descent, so
    /// hover, completion, and the W123 emitter cannot disagree about which
    /// scope is innermost.
    #[must_use]
    pub(crate) fn at(analysis: &AnalysisResult, byte_offset: u32) -> Self {
        Self {
            resolves: tcl_compiler::analyser::innermost_scope_reaches_oo_helpers(
                &analysis.global_scope,
                byte_offset,
            ),
            method: tcl_compiler::analyser::innermost_scope_is_oo_method_frame(
                &analysis.global_scope,
                byte_offset,
            ),
        }
    }

    /// Whether a consumer that offers commands to the user (completion,
    /// hover) may offer `name` in this frame.
    ///
    /// `true` for everything the registry does not scope at all. A scoped
    /// word needs the frame to resolve it, plus — when the registry says the
    /// word needs a real method invocation — a method frame. `my` is the one
    /// family member that does not, because it is the object's own dispatch
    /// command rather than an `::oo::Helpers` member and a class is an
    /// object, so `my new` in an `initialize` body genuinely works.
    #[must_use]
    pub(crate) fn admits(self, registry: &CommandRegistry, name: &str) -> bool {
        if !registry.resolves_only_in_method_context(name) {
            return true;
        }
        self.resolves && (self.method || !registry.requires_oo_method_frame(name))
    }
}
