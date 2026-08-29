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
//! Under 8.6 core the namespace holds only `next`, `nextto`, and `self`
//! (tclsh 8.6.14: `info commands ::oo::Helpers::*`); each qualified spec
//! inherits its bare twin's own `dialects` mask, so the 9.0-only members
//! stay 9.0-only here too.
//!
//! `link` therefore needs the **same two entries** its bare twin has, not
//! one (Codex review of PR #1084): Tcllib's `ooutil` installs a real
//! `::oo::Helpers::link` under 8.6/8.7, so a document that says
//! `package require oo::util` may legitimately write the qualified spelling
//! and must get completion, hover, and no `W123`. Deriving only from the
//! 9.0 core spec would have made the qualified call unknown on exactly the
//! dialect where a user has to reach for it. Both twins are derived from
//! their bare originals, so the package gating (`required_package`,
//! `tcllib_package`) carries across unchanged and the qualified/bare pair
//! answer alike per dialect. `mymethod` and `classvariable` are the other
//! two such members and get the identical treatment — `ooutil.tcl:18` and
//! `ooutil.tcl:27` define `proc ::oo::Helpers::mymethod` and
//! `proc ::oo::Helpers::classvariable` respectively — while `callback`
//! (9.0 alias of `mymethod`) has a single core entry, because no Tcllib
//! package provides that spelling (issue #923 audit, `ticklecharts` idx 51).
use crate::prelude::*;

/// The bare specs whose `oo::Helpers::…` spelling is also a real,
/// separately-callable command, paired with that spelling.
///
/// Derived from the bare spec rather than retyped, so hover text, arity,
/// dialect gating, **package gating**, and options cannot drift between the
/// two spellings. `link` and `mymethod` each appear twice for the same
/// reason they do in the bare table — a 9.0 core entry and an 8.6/8.7
/// `ooutil` one; the registry keys duplicates by name and picks per dialect
/// (`best_visible`).
fn family() -> Vec<(&'static str, CommandSpec)> {
    vec![
        ("oo::Helpers::callback", super::oo_callback::spec()),
        ("oo::Helpers::mymethod", super::oo_callback::mymethod_spec()),
        (
            "oo::Helpers::mymethod",
            super::oo_callback::mymethod_spec_ooutil_86(),
        ),
        ("oo::Helpers::link", super::oo_link::spec()),
        ("oo::Helpers::link", super::oo_link::spec_ooutil_86()),
        ("oo::Helpers::next", super::oo_next::spec()),
        ("oo::Helpers::nextto", super::nextto::spec()),
        ("oo::Helpers::self", super::oo_self::spec()),
        (
            "oo::Helpers::classvariable",
            super::oo_classvariable::spec(),
        ),
        (
            "oo::Helpers::classvariable",
            super::oo_classvariable::spec_ooutil_86(),
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
///
/// `TCLOO_REQUIRES_METHOD_FRAME` is among them for the same reason
/// `TCLOO_METHOD_CONTEXT` is: both describe where the *bare* word may be
/// written. The qualified name is an ordinary global command that a
/// consumer may offer anywhere — its own "only from inside a method"
/// failure is a runtime error the caller sees, not a static one.
const QUALIFIED_SPELLING_EXCLUDED: Traits = Traits::TCLOO_METHOD_CONTEXT
    .union(Traits::TCLOO_REQUIRES_METHOD_FRAME)
    .union(Traits::TCLOO_SELF_DISPATCH)
    .union(Traits::TCLOO_NEXT_CHAIN)
    .union(Traits::TCLOO_INTROSPECTION)
    .union(Traits::TCLOO_BINDS_METHOD_ALIAS);

#[cfg(test)]
mod tests {
    use tcl_dialect::model::{SpecSurface};
    use super::*;

    /// tclsh 9.0.4: `info commands ::oo::Helpers::*` lists exactly
    /// `callback classvariable link mymethod next nextto self` — all seven
    /// of which the registry now models with bare specs. `my` is **not**
    /// among them (it is `::oo::ObjN::my`), so it must never gain a
    /// qualified `oo::Helpers::my` spelling here.
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
                !spec.traits.contains(Traits::TCLOO_METHOD_CONTEXT)
                    && !spec.traits.contains(Traits::TCLOO_REQUIRES_METHOD_FRAME),
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

    /// Each qualified spec inherits its bare twin's dialect **and package**
    /// gate, so `next` / `nextto` / `self` stay 8.6+, `callback` stays
    /// 9.0-only, and `link` / `mymethod` / `classvariable` each keep both
    /// of their entries — 9.0 core plus the 8.6/8.7 `ooutil` one —
    /// matching `info commands ::oo::Helpers::*` on both interpreters.
    ///
    /// Compared pairwise against `family()`'s own order rather than by
    /// name lookup, so the two `link` rows cannot alias onto each other.
    #[test]
    fn qualified_spelling_inherits_the_bare_dialect_and_package_gate() {
        let derived = qualified_specs();
        assert_eq!(derived.len(), family().len());
        for ((qualified, bare), found) in family().into_iter().zip(derived) {
            assert_eq!(found.name, qualified);
            assert_eq!(found.surface, bare.surface, "{qualified} dialects");
            assert_eq!(
                found.required_package, bare.required_package,
                "{qualified} required_package"
            );
            assert_eq!(
                found.tcllib_package, bare.tcllib_package,
                "{qualified} tcllib_package"
            );
        }
    }

    /// tclsh 8.6.14 core has no `::oo::Helpers::link` at all, but Tcllib's
    /// `ooutil` installs one — so the qualified spelling needs the same
    /// two-entry treatment the bare word has, and the `ooutil` entry must
    /// keep the package gate that makes W120 fire without the
    /// `package require` and stay silent with it (Codex review of PR
    /// #1084).
    #[test]
    fn qualified_link_has_both_a_core_90_and_an_ooutil_86_entry() {
        let links: Vec<CommandSpec> = qualified_specs()
            .into_iter()
            .filter(|s| s.name == "oo::Helpers::link")
            .collect();
        assert_eq!(links.len(), 2, "one core 9.0 entry, one ooutil 8.6 entry");
        let core = links
            .iter()
            .find(|s| s.required_package.is_none())
            .expect("a core entry");
        assert_eq!(core.surface, Some(SpecSurface::TCL90_PLUS));
        let ooutil = links
            .iter()
            .find(|s| s.required_package == Some("oo::util"))
            .expect("an ooutil entry");
        assert_eq!(ooutil.surface, Some(SpecSurface::TCL86));
        assert_eq!(ooutil.tcllib_package, Some("oo::util"));
    }
}
