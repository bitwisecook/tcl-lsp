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

//! `oo::Helpers` — the namespace whose commands a `TclOO` method body
//! reaches by bare name, and their qualified spellings.
//!
//! A method body runs with the *object's* namespace current and
//! `::oo::Helpers` on that namespace's `namespace path`, so the family's
//! bare spellings resolve there and nowhere else (tclsh 9.0.4, inside
//! `oo::class create C { method m {} { … } }`):
//!
//! ```text
//! ns:   ::oo::Obj22
//! path: ::oo::Helpers
//! namespace which -command link  ->  ::oo::Helpers::link
//! namespace which -command next  ->  ::oo::Helpers::next
//! namespace which -command self  ->  ::oo::Helpers::self
//! namespace which -command my    ->  ::oo::Obj22::my        <- NOT a helper
//! ```
//!
//! Two consequences this module and [`crate::traits::Traits::TCLOO_METHOD_CONTEXT`]
//! encode between them:
//!
//! * The **bare** spellings are method-context-only. At the top level
//!   tclsh 9.0.4 raises `invalid command name "link"` (likewise `my`,
//!   `next`, `nextto`, `self`, `classvariable`), and `info commands ::link`
//!   is empty — so they carry `TCLOO_METHOD_CONTEXT` on their own specs.
//! * The **qualified** spellings registered here are ordinary global
//!   commands, reachable from anywhere: `info commands ::oo::Helpers::link`
//!   answers `::oo::Helpers::link`, and calling it at the top level fails
//!   with the *runtime* `::oo::Helpers::link may only be called from inside
//!   a method`, not with "invalid command name". So they must **not** carry
//!   `TCLOO_METHOD_CONTEXT` — the command genuinely exists — and they must
//!   not carry the dispatch traits either, which answer for the bare
//!   keyword a method body writes.
//!
//! `my` and `myclass` are deliberately absent: they live in each object's
//! *own* namespace (`::oo::Obj22::my`), which has no statically nameable
//! spelling, so `my`'s single bare spec carries the whole model.
//!
//! Under 8.6 the namespace holds only `next`, `nextto`, and `self`
//! (tclsh 8.6.14: `info commands ::oo::Helpers::*`); each qualified spec
//! inherits its bare twin's own `dialects` mask, so the 9.0-only members
//! stay 9.0-only here too.
use crate::prelude::*;

/// The bare specs whose `oo::Helpers::…` spelling is also a real,
/// separately-callable command, paired with that spelling.
///
/// Derived from the bare spec rather than retyped, so hover text, arity,
/// dialect gating, and options cannot drift between the two spellings.
fn family() -> Vec<(&'static str, CommandSpec)> {
    vec![
        ("oo::Helpers::link", super::oo_link::spec()),
        ("oo::Helpers::next", super::oo_next::spec()),
        ("oo::Helpers::nextto", super::nextto::spec()),
        ("oo::Helpers::self", super::oo_self::spec()),
        (
            "oo::Helpers::classvariable",
            super::oo_classvariable::spec(),
        ),
    ]
}

/// Standalone specs for the `oo::Helpers` members under their qualified
/// spelling — the counterpart of `dict::qualified_specs`.
///
/// `CommandRegistry::get` falls back from a `::`-rooted name to the bare
/// key, so one registration per member serves both `oo::Helpers::link` and
/// `::oo::Helpers::link`.
#[must_use]
pub fn qualified_specs() -> Vec<CommandSpec> {
    family()
        .into_iter()
        .map(|(qualified, bare)| {
            // The scope restriction and the dispatch-keyword identity both
            // belong to the bare word a method body writes; the qualified
            // command is an ordinary global one. Everything else — arity,
            // dialects, hover, forms, package gating — carries over.
            let mut traits = bare.traits;
            traits.remove(QUALIFIED_SPELLING_EXCLUDED);
            CommandSpec {
                name: qualified,
                traits,
                implementation_namespace: Some("::oo::Helpers"),
                ..bare
            }
        })
        .collect()
}

/// Traits that belong to the bare, method-context spelling only.
const QUALIFIED_SPELLING_EXCLUDED: Traits = Traits::TCLOO_METHOD_CONTEXT
    .union(Traits::TCLOO_SELF_DISPATCH)
    .union(Traits::TCLOO_NEXT_CHAIN)
    .union(Traits::TCLOO_INTROSPECTION)
    .union(Traits::TCLOO_BINDS_METHOD_ALIAS);

#[cfg(test)]
mod tests {
    use super::*;

    /// tclsh 9.0.4: `info commands ::oo::Helpers::*` lists exactly
    /// `callback classvariable link mymethod next nextto self`. Of those,
    /// the registry models the five with bare specs; `my` is **not** among
    /// them (it is `::oo::ObjN::my`), so it must never gain a qualified
    /// `oo::Helpers::my` spelling here.
    #[test]
    fn my_is_not_an_oo_helpers_member() {
        assert!(
            !qualified_specs().iter().any(|s| s.name.ends_with("::my")),
            "`my` lives in the object's own namespace, not ::oo::Helpers",
        );
    }

    /// The qualified spelling is a real global command — calling it outside
    /// a method is a runtime error, not an unknown command — so it must not
    /// inherit the bare word's method-context scope or dispatch identity.
    #[test]
    fn qualified_spelling_drops_the_bare_word_traits() {
        for spec in qualified_specs() {
            assert!(
                !spec.traits.contains(Traits::TCLOO_METHOD_CONTEXT),
                "{} must not be method-context-scoped",
                spec.name
            );
            assert!(
                !spec.traits.contains(Traits::TCLOO_NEXT_CHAIN)
                    && !spec.traits.contains(Traits::TCLOO_INTROSPECTION)
                    && !spec.traits.contains(Traits::TCLOO_BINDS_METHOD_ALIAS),
                "{} must not carry the bare keyword's dispatch traits",
                spec.name
            );
        }
    }

    /// Each qualified spec inherits its bare twin's dialect gate, so the
    /// 9.0-only members (`link`, `classvariable`) stay 9.0-only and the
    /// 8.6 members (`next`, `nextto`, `self`) stay 8.6+ — matching
    /// `info commands ::oo::Helpers::*` on both interpreters.
    #[test]
    fn qualified_spelling_inherits_the_bare_dialect_gate() {
        for (qualified, bare) in family() {
            let found = qualified_specs()
                .into_iter()
                .find(|s| s.name == qualified)
                .expect("registered above");
            assert_eq!(found.dialects, bare.dialects, "{qualified}");
        }
    }
}
