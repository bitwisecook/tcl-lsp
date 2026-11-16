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

//! `TclOO` dispatch keywords and the commands that exist only in a method context or frame, per dialect.

use super::*;

// TclOO method-context keywords and the `Tcl_ConcatObj` eval family.

/// The `TclOO` method-context keyword set is exactly `my` / `next` / `nextto`
/// / `self`, and `CommandRegistry::method_dispatch_keyword` answers with the
/// right kind for each.
///
/// The consumer-visible keyword set must
/// equal the trait-carrying specs, so a consumer can never drift from the
/// registry by re-adding a `head == "my"` literal.  `link` is deliberately
/// out — it *creates* per-class bareword commands rather than dispatching one.
///
/// registry-metadata: which keyword is dispatch and which is introspection is
/// our classification.  (`my`, `next`, `nextto`, `self`, and `link` are all
/// real tclsh 8.6/9.0 `TclOO` commands.)
#[test]
fn tcloo_dispatch_keyword_membership() {
    let reg = static_context_for("tcl8.6").commands();
    assert_eq!(
        reg.method_dispatch_keyword("my"),
        Some(MethodDispatchKind::SelfDispatch)
    );
    assert_eq!(
        reg.method_dispatch_keyword("next"),
        Some(MethodDispatchKind::NextChain)
    );
    assert_eq!(
        reg.method_dispatch_keyword("nextto"),
        Some(MethodDispatchKind::NextChain)
    );
    assert_eq!(
        reg.method_dispatch_keyword("self"),
        Some(MethodDispatchKind::Introspection)
    );
    // Tcl does not install a global `::my`; the leading `::` therefore must
    // not fall back to the contextual bare word.
    assert_eq!(reg.method_dispatch_keyword("::my"), None);
    for negative in ["puts", "set", "link", "proc", "oo::define"] {
        assert_eq!(
            reg.method_dispatch_keyword(negative),
            None,
            "{negative} is not a method-dispatch keyword"
        );
    }
    // The trait-carrying spec set and the query agree, in both directions.
    let mut carriers: Vec<&str> = reg
        .commands_with_trait(Traits::TCLOO_SELF_DISPATCH)
        .into_iter()
        .chain(reg.commands_with_trait(Traits::TCLOO_NEXT_CHAIN))
        .chain(reg.commands_with_trait(Traits::TCLOO_INTROSPECTION))
        .collect();
    carriers.sort_unstable();
    assert_eq!(carriers, ["my", "next", "nextto", "self"]);
    for name in carriers {
        assert!(
            reg.method_dispatch_keyword(name).is_some(),
            "{name} carries a dispatch trait so the query must answer for it"
        );
    }
}

/// The method-context-scoped set — which bare spellings only
/// resolve inside a `TclOO` method body — is exactly `callback` / `link` /
/// `my` / `mymethod` / `next` / `nextto` / `self` / `classvariable`, and
/// never the qualified `oo::Helpers::…` spellings.
///
/// Pinned against tclsh 9.0.4: each bare word at the top level raises
/// `invalid command name`, `info commands ::link` is empty, and inside
/// `oo::class create C { method m {} { … } }` `namespace which -command
/// link` answers `::oo::Helpers::link` while `… my` answers
/// `::oo::ObjN::my`. tclsh 8.6.14 agrees for the four members it ships.
/// The qualified spelling is a real global command — calling it outside a
/// method fails with `::oo::Helpers::link may only be called from inside a
/// method`, a *runtime* error rather than an unknown command — so it must
/// answer `false`.
///
/// The guard against a consumer re-deriving the set from a name list: W123,
/// hover, and completion all ask
/// `CommandRegistry::resolves_only_in_method_context`.
#[test]
fn tcloo_method_context_membership() {
    let reg = static_context_for("tcl9.0").commands();
    let mut carriers = reg.commands_with_trait(Traits::TCLOO_METHOD_CONTEXT);
    carriers.sort_unstable();
    carriers.dedup();
    assert_eq!(
        carriers,
        [
            "callback",
            "classvariable",
            "link",
            "my",
            "mymethod",
            "next",
            "nextto",
            "self"
        ]
    );
    for name in carriers {
        assert!(
            reg.resolves_only_in_method_context(name),
            "{name} carries the trait so the query must answer for it"
        );
        assert!(
            !reg.resolves_only_in_method_context(&format!("::{name}")),
            "Tcl has no global ::{name} command"
        );
    }
    for qualified in [
        "oo::Helpers::link",
        "::oo::Helpers::link",
        "oo::Helpers::self",
        "oo::Helpers::classvariable",
    ] {
        assert!(
            reg.get(qualified).is_some(),
            "{qualified} is a real global command in Tcl 9.0"
        );
        assert!(
            !reg.resolves_only_in_method_context(qualified),
            "{qualified} resolves from anywhere"
        );
    }
    for negative in ["puts", "set", "proc", "oo::define", "oo::class"] {
        assert!(
            !reg.resolves_only_in_method_context(negative),
            "{negative} is an ordinary global command"
        );
    }
    // `commands_with_trait` enumerates the whole spec table, so the dialect
    // gate lives in the query itself: under 8.6 `callback` (9.0+ core only,
    // no Tcllib route) is not method-context-scoped because it does not
    // exist, while `link`, `mymethod`, and `classvariable` still are — each
    // has an 8.6 spelling via the `ooutil`-gated twin.
    let reg86 = static_context_for("tcl8.6").commands();
    assert!(!reg86.resolves_only_in_method_context("callback"));
    for name in [
        "link",
        "my",
        "mymethod",
        "next",
        "nextto",
        "self",
        "classvariable",
    ] {
        assert!(
            reg86.resolves_only_in_method_context(name),
            "{name}: 8.6 has it (via ooutil for link/mymethod/classvariable)"
        );
    }
    // 8.5 has no TclOO at all, so the whole query answers `false` there.
    let reg85 = static_context_for("tcl8.5").commands();
    for name in ["link", "my", "next", "nextto", "self", "classvariable"] {
        assert!(
            !reg85.resolves_only_in_method_context(name),
            "{name}: tcl8.5 has no TclOO"
        );
    }
}

/// `link` — and only `link` — declares that it binds bareword aliases for
/// the current object's methods, so the analyser's class-body walk finds
/// those calls through the registry instead of a `texts[0] == "link"`
/// literal.
#[test]
fn tcloo_method_alias_binding_membership() {
    let reg = static_context_for("tcl9.0").commands();
    let mut carriers = reg.commands_with_trait(Traits::TCLOO_BINDS_METHOD_ALIAS);
    carriers.sort_unstable();
    carriers.dedup();
    assert_eq!(carriers, ["link"]);
    assert!(reg.binds_method_alias("link"));
    assert!(!reg.binds_method_alias("::link"));
    for negative in ["my", "next", "self", "oo::Helpers::link", "proc"] {
        assert!(
            !reg.binds_method_alias(negative),
            "{negative} binds no method alias"
        );
    }
    // 8.5 has no `link` at all.
    assert!(
        !static_context_for("tcl8.5")
            .commands()
            .binds_method_alias("link")
    );
}

/// **Resolving is not calling.** Every `::oo::Helpers` member needs a real
/// method invocation; `my` does not.
///
/// tclsh 9.0.4, inside `oo::class create ::P { initialize { … } }` — a frame
/// with the class object's namespace current and `namespace path` =
/// `::oo::Helpers ::oo`:
///
/// ```text
/// link:          which='::oo::Helpers::link'           call -> link may only be called from inside a method
/// next:          which='::oo::Helpers::next'           call -> next may only be called from inside a method
/// nextto:        which='::oo::Helpers::nextto'         call -> nextto may only be called from inside a method
/// self:          which='::oo::Helpers::self'           call -> self may only be called from inside a method
/// classvariable: which='::oo::Helpers::classvariable'  call -> classvariable may only be called from inside a method
/// callback:      which='::oo::Helpers::callback'       call -> callback may only be called from inside a method
/// mymethod:      which='::oo::Helpers::mymethod'       call -> mymethod may only be called from inside a method
/// my:            which='::oo::Obj20::my'               call -> OK  (`my new` returns ::oo::Obj22)
/// ```
///
/// Everything here still carries `TCLOO_METHOD_CONTEXT` — outside any object
/// frame there is no such command at all — so this set is a strict subset.
#[test]
fn tcloo_method_frame_membership() {
    let reg = static_context_for("tcl9.0").commands();
    let mut carriers = reg.commands_with_trait(Traits::TCLOO_REQUIRES_METHOD_FRAME);
    carriers.sort_unstable();
    carriers.dedup();
    assert_eq!(
        carriers,
        [
            "callback",
            "classvariable",
            "link",
            "mymethod",
            "next",
            "nextto",
            "self"
        ],
        "`my` is `::oo::ObjN::my`, callable from any object frame",
    );
    for name in carriers {
        assert!(reg.requires_oo_method_frame(name), "{name}");
        assert!(
            reg.resolves_only_in_method_context(name),
            "{name}: the method-frame set is a subset of the scoped set",
        );
        assert!(
            !reg.requires_oo_method_frame(&format!("::{name}")),
            "Tcl has no global ::{name} command",
        );
    }
    assert!(reg.resolves_only_in_method_context("my"));
    assert!(
        !reg.requires_oo_method_frame("my"),
        "`my new` in a class `initialize` body really does make an instance",
    );
    for qualified in ["oo::Helpers::link", "::oo::Helpers::self"] {
        assert!(
            !reg.requires_oo_method_frame(qualified),
            "{qualified} is an ordinary global command",
        );
    }
    assert!(!reg.requires_oo_method_frame("puts"));
    // 8.6 core has the three it ships plus `ooutil`'s `link` and
    // `classvariable`; `callback` is 9.0-only (no Tcllib route). 8.5 has no
    // TclOO at all.
    let reg86 = static_context_for("tcl8.6").commands();
    assert!(!reg86.requires_oo_method_frame("callback"));
    assert!(!reg86.requires_oo_method_frame("my"));
    for name in ["link", "next", "nextto", "self", "classvariable"] {
        assert!(reg86.requires_oo_method_frame(name), "{name} under 8.6");
    }
    let reg85 = static_context_for("tcl8.5").commands();
    for name in ["link", "my", "next", "nextto", "self", "classvariable"] {
        assert!(!reg85.requires_oo_method_frame(name), "{name}: no TclOO");
    }
}

/// The **qualified** `oo::Helpers::…` spelling must be gated exactly like its
/// bare twin per dialect — same dialect availability, same package
/// requirement — because they are the same underlying command.
///
/// The regression this pins: `oo::Helpers::link` was derived only from the
/// 9.0 core spec, so an 8.6 document with `package require ooutil` — where
/// Tcllib installs a real `::oo::Helpers::link` — found the qualified call
/// unknown.
#[test]
fn qualified_oo_helpers_spellings_track_their_bare_twin_per_dialect() {
    for dialect in ["tcl8.5", "tcl8.6", "tcl8.7", "tcl9.0"] {
        let reg = static_context_for(dialect).commands();
        let mask = reg.profile().expect("built for a dialect").surface_query();
        let mask = Some(mask);
        for bare in ["link", "next", "nextto", "self", "classvariable"] {
            let qualified = format!("oo::Helpers::{bare}");
            let bare_spec = reg.get_for_surface(bare, mask);
            let qual_spec = reg.get_for_surface(&qualified, mask);
            assert_eq!(
                bare_spec.is_some(),
                qual_spec.is_some(),
                "{dialect}: {bare} and {qualified} must agree on existence",
            );
            if let (Some(b), Some(q)) = (bare_spec, qual_spec) {
                assert_eq!(b.surface, q.surface, "{dialect}: {qualified} dialects");
                assert_eq!(
                    b.required_package, q.required_package,
                    "{dialect}: {qualified} required_package",
                );
            }
        }
    }
}

/// The consumer-visible keyword set equals the trait-carrying specs across
/// every dialect a `TclOO` consumer may run under, including the F5 ones.
///
/// What this guards: `tcl-lsp-core`'s references,
/// call-hierarchy, definition, hover, rename, and semantic-token providers,
/// plus the compiler's `var_command` / `elimination` / `class_lattice` /
/// `var_observability` passes, all resolve these words through
/// `method_dispatch_keyword`. If a consumer ever re-adds a literal name list
/// it will disagree with this set the moment a dialect changes — which is
/// exactly the drift this asserts cannot happen silently.
#[test]
fn tcloo_dispatch_keyword_set_matches_the_trait_carriers_per_dialect() {
    for dialect in ["tcl8.6", "tcl9.0", "f5-irules", "f5-iapps", "expect"] {
        let reg = static_context_for(dialect).commands();
        let mut carriers: Vec<&str> = reg
            .commands_with_trait(Traits::TCLOO_SELF_DISPATCH)
            .into_iter()
            .chain(reg.commands_with_trait(Traits::TCLOO_NEXT_CHAIN))
            .chain(reg.commands_with_trait(Traits::TCLOO_INTROSPECTION))
            .filter(|name| reg.method_dispatch_keyword(name).is_some())
            .collect();
        carriers.sort_unstable();
        let mut answered: Vec<&str> = ["my", "next", "nextto", "self", "link", "puts", "proc"]
            .into_iter()
            .filter(|name| reg.method_dispatch_keyword(name).is_some())
            .collect();
        answered.sort_unstable();
        assert_eq!(
            carriers, answered,
            "under {dialect} the answered keyword set must be exactly the trait carriers"
        );
    }
}

/// All four keywords are `TCL86_PLUS`, so a pre-8.6 dialect registry answers
/// `None` — dialect-awareness comes from the registry instance, never from a
/// per-consumer version check.
///
/// tclsh: `TclOO` shipped with 8.6; `my` / `next` / `self` do not exist in
/// 8.4 or 8.5.
#[test]
fn tcloo_dispatch_keywords_are_absent_before_86() {
    for dialect in ["tcl8.4", "tcl8.5"] {
        let reg = static_context_for(dialect).commands();
        for keyword in ["my", "next", "nextto", "self"] {
            assert_eq!(
                reg.method_dispatch_keyword(keyword),
                None,
                "{keyword} must not resolve under {dialect}"
            );
        }
    }
    // …and still resolves under 9.0.
    let reg = static_context_for("tcl9.0").commands();
    assert_eq!(
        reg.method_dispatch_keyword("my"),
        Some(MethodDispatchKind::SelfDispatch)
    );
}
