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
//! It also owns the `TclOO` **frame** classifier ([`OoFrame`]), the
//! LSP-side half of issue #1026's scoping rule for the `oo::Helpers`
//! family.

use tcl_compiler::analyser::{AnalysisResult, MethodDef};
use tcl_registry::CommandRegistry;

use crate::definition::MethodBucket;

/// What kind of `TclOO` frame a cursor sits in — the LSP-side half of issue
/// #1026's scoping rule, and the one place the question is asked.
///
/// Two facts, because real Tcl keeps them apart (Codex review of PR #1084):
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

/// The **first applicable implementation** of `method` on `class_q`'s
/// `TclOO` linearisation — the entry `$obj m` / `my m` actually runs — with
/// the qualified name of the class that provides it.
///
/// Mixins come before the class and subclasses before bases, exactly as
/// `ClassHierarchy`'s `mro_map` orders them; the first provider whose entry
/// is *visible* in this context wins:
///
/// * `external` (a `$obj m` / `CLASS m` dispatch) sees exported
///   implementations only — and "exported" is the receiver's *effective*
///   state ([`superclass_chain_export_state`]), not the provider's own
///   declaration, because a subclass can `export` / `unexport` a name it
///   inherits without redeclaring it.  A provider reached through a mixin
///   keeps the per-entry reading: C enters each mixin with a fresh copy of
///   the dispatch flags, so the spine's state does not travel there.
/// * an internal (`my m`) dispatch also reaches unexported ones, and
///   `private` ones only in the receiver's own class.  Export state is not
///   consulted at all: `my` ignores it (tclsh 8.6.16 / 9.0.4 —
///   `oo::define Child { unexport tick }` leaves `my tick` reaching
///   `Base`'s body).
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
    // Read once for the whole walk: the flag is a property of the *receiver*,
    // and every spine provider answers to it.
    let spine_export = external
        .then(|| superclass_chain_export_state(analysis, class_q, method, bucket))
        .flatten();
    let spine: &[String] = if spine_export.is_some() {
        hierarchy.spine_map.get(class_q).map_or(&[], Vec::as_slice)
    } else {
        &[]
    };
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
            match spine_export {
                // `private` is the one state an `export` cannot lift: it is
                // not a flag on a table entry but a separate, class-local
                // slot (tclsh 9.0.4 — a subclass's `export priv` still
                // leaves `[Child new] priv` an `unknown method`).
                Some(exported) if spine.contains(provider_q) => {
                    exported && md.visibility != "private"
                }
                _ => md.visibility == "public",
            }
        } else {
            md.visibility != "private" || provider_q == class_q
        };
        if visible {
            return Some((provider_q.as_str(), md));
        }
    }
    None
}

/// The export flag an **external** dispatch (`$obj m`, `CLASS m`, or a
/// captured `[self]` object command) reads for `method` on a receiver of
/// class `class_q` — `None` when nothing on the superclass spine mentions the
/// name, so there is no state to read.
///
/// The spine is `ClassHierarchy::spine_map` — the mixin-free linearisation,
/// built from the same resolved superclass edges as `mro_map`, because C
/// enters each mixin with a fresh copy of the dispatch flags and so lets a
/// mixin's `unexport` empty only its own branch.
///
/// `TclOO` takes this flag from the *most specific* spine class that mentions
/// the member and the **implementation** from the first class that declares a
/// body; the two are genuinely independent, because `export` / `unexport`
/// accept a name their class does not define and create a body-less table
/// entry whose only content is the flag.  `info class methods` does not list
/// such an entry, which is why this cannot be read off the member tables
/// alone.  Oracle, byte-identical on tclsh 8.6.16 and 9.0.4:
///
/// ```tcl
/// oo::class create Base   { method tick {} { return base } }
/// oo::class create Child  { superclass Base }
/// oo::define Child { unexport tick }
/// [Child new] tick    ;# -> unknown method "tick"  (Base's public body, suppressed here)
/// oo::class create Base3  { method tock {} { return b3 } ; unexport tock }
/// oo::class create Child3 { superclass Base3 ; export tock }
/// [Child3 new] tock   ;# -> b3   (Base3's unexported body, revived here)
/// [Base3 new] tock    ;# -> unknown method "tock"
/// ```
///
/// A declaration on the deciding class outranks that class's own stub sets: a
/// later `method g {…}` re-applies the name's default visibility over an
/// earlier `unexport g` (tclsh 8.6.16 / 9.0.4 — `oo::define G { unexport g ;
/// method g {} {…} }` leaves `[G new] g` callable), and the analyser has
/// already folded that source order into `MethodDef::visibility`.
///
/// Multiple `superclass`es are walked in declaration order and the first
/// mention wins, matching the order the chain builder appends them in.  C
/// keeps each branch's flags independent, so a *later* branch disagreeing
/// with the first is approximated rather than modelled — the same
/// single-linearisation approximation `mro_map` already makes.
fn superclass_chain_export_state(
    analysis: &AnalysisResult,
    class_q: &str,
    method: &str,
    bucket: MethodBucket,
) -> Option<bool> {
    let hierarchy = analysis.class_hierarchy();
    for spine_q in hierarchy.spine_map.get(class_q)? {
        let Some(cd) = analysis.all_classes.get(spine_q) else {
            continue;
        };
        let (table, exports, unexports) = match bucket {
            MethodBucket::Instance => (&cd.methods, &cd.exports, &cd.unexports),
            MethodBucket::Class => (&cd.class_methods, &cd.class_exports, &cd.class_unexports),
        };
        if let Some(md) = table.get(method) {
            return Some(md.visibility == "public");
        }
        if exports.contains(method) {
            return Some(true);
        }
        if unexports.contains(method) {
            return Some(false);
        }
    }
    None
}
