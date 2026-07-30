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

//! `TclOO` dispatch resolution shared by go-to-definition, hover, and
//! find-references.
//!
//! Three providers used to answer "which implementation does this `$obj m`
//! / `my m` call reach?" three different ways: `definition.rs` walked the
//! full method-resolution order (superclasses *and* mixins, C-faithfully),
//! while `hover.rs` and `references.rs` each looked the method up only on
//! the receiver's own class and gave up on a miss. The result was that a
//! method reached through a `mixin` or a `superclass` had working
//! go-to-definition but no hover and no references at the identical cursor
//! position (issue #923 idx 28 / 34 / 35).
//!
//! This module owns **one** walk — [`method_dispatch_provider`] — and the
//! three features are thin renderings of its answer, so they cannot drift
//! apart again. The walk is a single pass over the class's already-computed
//! linearisation: `O(chain length)`, no fixpoint.
//!
//! It also owns the `TclOO` **method-context** predicate
//! ([`in_oo_method_context`]), the LSP-side half of issue #1026's scoping
//! rule for the `oo::Helpers` family.

use tcl_compiler::analyser::{AnalysisResult, MethodDef};

use crate::definition::MethodBucket;

/// Whether `byte_offset` sits inside a `TclOO` method context — a
/// `method`, `constructor`, `destructor`, class-side (`self method` /
/// `classmethod`), or `oo::objdefine method` body.
///
/// The one place the LSP asks that question, resolved through the
/// analyser's own scope walk
/// (`tcl_compiler::analyser::scope::innermost_scope_reaches_oo_helpers`)
/// so hover, completion, and the W123 emitter cannot disagree about which
/// bodies count.
///
/// Paired with [`tcl_registry::CommandRegistry::resolves_only_in_method_context`]
/// it implements issue #1026's rule: the registry says *which* commands are
/// method-context-only (`link` / `my` / `next` / `nextto` / `self` /
/// `classvariable`), this says *where* the cursor is, and neither side
/// carries a command name. tclsh 9.0.4 at the top level answers `invalid
/// command name` for every one of them; inside a method body they resolve
/// (`namespace which -command link` → `::oo::Helpers::link`, `… my` →
/// `::oo::ObjN::my`).
#[must_use]
pub(crate) fn in_oo_method_context(analysis: &AnalysisResult, byte_offset: u32) -> bool {
    tcl_compiler::analyser::scope::innermost_scope_reaches_oo_helpers(
        &analysis.global_scope,
        byte_offset,
    )
}

/// The **first applicable implementation** of `method` on `class_q`'s
/// `TclOO` linearisation — the entry `$obj m` / `my m` actually runs — with
/// the qualified name of the class that provides it.
///
/// Mixins come before the class and subclasses before bases, exactly as
/// `ClassHierarchy`'s `mro_map` orders them; the first provider whose entry
/// is *visible* in this context wins:
///
/// * `external` (a `$obj m` / `CLASS m` dispatch) sees exported
///   implementations only.
/// * an internal (`my m`) dispatch also reaches unexported ones, and
///   `private` ones only in the receiver's own class.
///
/// `None` is a definitive "no implementation is callable here" — mirroring
/// C's `unknown method` — not "look somewhere else".
///
/// Returns the provider as well as the [`MethodDef`] because every caller
/// needs it: definition renders the `name_span`, hover names the providing
/// class in its "inherited from" note, and references re-anchors its whole
/// scan on the class that actually declares the method.
#[must_use]
pub(crate) fn method_dispatch_provider<'a>(
    analysis: &'a AnalysisResult,
    class_q: &str,
    method: &str,
    external: bool,
    bucket: MethodBucket,
) -> Option<(&'a str, &'a MethodDef)> {
    let hierarchy = analysis.class_hierarchy();
    let mro = hierarchy.mro_map.get(class_q)?;
    for provider_q in mro {
        let Some(cd) = analysis.all_classes.get(provider_q) else {
            continue;
        };
        let md = match bucket {
            // A class-side method is never itself instance-callable (real
            // tclsh: `unknown method` when an instance calls a
            // classmethod) — no `class_methods` fallback here, matching
            // `completion.rs`'s `method_items`, which already excludes it
            // for the identical reason.
            MethodBucket::Instance => cd.methods.get(method),
            MethodBucket::Class => cd
                .class_methods
                .get(method)
                .filter(|md| provider_q == class_q || !md.is_self_method),
        };
        let Some(md) = md else { continue };
        let visible = if external {
            md.visibility == "public"
        } else {
            md.visibility != "private" || provider_q == class_q
        };
        if visible {
            return Some((provider_q.as_str(), md));
        }
    }
    None
}
