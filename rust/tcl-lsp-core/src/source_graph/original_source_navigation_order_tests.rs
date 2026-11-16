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

use super::*;
use tcl_compiler::analyser::Analyser;

fn point(uri: &str, at: u32) -> RunPoint<'_> {
    RunPoint {
        uri,
        at,
        enclosing_body: None,
    }
}

#[test]
fn original_workspace_order_retains_conditional_purpose_without_tombstone_authority() {
    let source = "set before 1\nset after 2\n";
    let mut analysis = Analyser::new().analyse(source, "tcl8.6");
    analysis.dialect = "display-only-changed".into();
    let context = SourceOrderContext::capture(&analysis).unwrap();
    let order = RunOrder::for_source_navigation(&[], [("doc".into(), context)].into_iter());
    let before = point("doc", 0);
    let after = point(
        "doc",
        u32::try_from(source.find("set after").unwrap()).unwrap(),
    );
    // Two straight-line points of one document are ordered by its own
    // sequence. The legacy `has_run` and `cmp_run` stay unavailable for
    // workspace navigation.
    let relation = order.navigation_precedence(before, after).unwrap();
    assert_eq!(
        relation.purpose(),
        SourceNavigationOrderPurpose::DocumentSequence
    );
    assert!(*relation.conditional_relation());
    assert_eq!(relation.modeled_source_exclusion(), Some(true));
    assert_eq!(order.has_run(before, after), None);
    assert_eq!(order.cmp_run(before, after), None);
    let ranking = order.navigation_comparison(before, after).unwrap();
    assert_eq!(*ranking.conditional_relation(), std::cmp::Ordering::Less);
    assert_eq!(
        ranking.modeled_source_exclusion(),
        Some(std::cmp::Ordering::Less)
    );

    let mut events = [
        crate::namespace_import::ExportEvent {
            pattern: "p",
            clears: false,
            at: before,
        },
        crate::namespace_import::ExportEvent {
            pattern: "",
            clears: true,
            at: after,
        },
    ]
    .into_iter();
    assert!(crate::namespace_import::exported_at_import_site(
        &mut events,
        "p",
        &order,
        after
    ));
    let mut aliases = [
        crate::namespace_import::AliasEvent {
            kind: crate::namespace_import::AliasEventKind::Install,
            at: before,
        },
        crate::namespace_import::AliasEvent {
            kind: crate::namespace_import::AliasEventKind::Remove,
            at: after,
        },
    ]
    .into_iter();
    assert!(crate::namespace_import::alias_live_at(
        &mut aliases,
        &order,
        Some(after)
    ));
}

/// `pattern p` / `-clear` and install / remove event pairs at `first`
/// and `second`: whether `p` is exported and the alias live at `query`.
fn exported_and_live(
    order: &RunOrder,
    first: RunPoint<'_>,
    second: RunPoint<'_>,
    query: RunPoint<'_>,
) -> (bool, bool) {
    use crate::namespace_import::{AliasEvent, AliasEventKind, ExportEvent};
    let exports = [
        ExportEvent {
            pattern: "p",
            clears: false,
            at: first,
        },
        ExportEvent {
            pattern: "",
            clears: true,
            at: second,
        },
    ];
    let aliases = [
        AliasEvent {
            kind: AliasEventKind::Install,
            at: first,
        },
        AliasEvent {
            kind: AliasEventKind::Remove,
            at: second,
        },
    ];
    (
        crate::namespace_import::exported_at_import_site(
            &mut exports.into_iter(),
            "p",
            order,
            query,
        ),
        crate::namespace_import::alias_live_at(&mut aliases.into_iter(), order, Some(query)),
    )
}

#[test]
fn document_end_follows_every_straight_removal_but_no_foreign_one() {
    let source = "set before 1\nset after 2\n";
    let analysis = Analyser::new().analyse(source, "tcl8.6");
    let mut other = Analyser::new().analyse(source, "tcl8.6");
    other.dialect = "display-only-other".into();
    let order = RunOrder::for_source_navigation(
        &[],
        [
            (
                "doc".into(),
                SourceOrderContext::capture(&analysis).unwrap(),
            ),
            ("other".into(), SourceOrderContext::capture(&other).unwrap()),
        ]
        .into_iter(),
    );
    let before = point("doc", 0);
    let after = point(
        "doc",
        u32::try_from(source.find("set after").unwrap()).unwrap(),
    );
    let end = point("doc", u32::try_from(source.len()).unwrap());
    // The same-document clear and removal have run at the document end.
    assert_eq!(
        exported_and_live(&order, before, after, end),
        (false, false)
    );
    // One in another document is never ordered after this one's events.
    let foreign = point("other", after.at);
    assert!(
        order
            .navigation_precedence(before, foreign)
            .is_none_or(|relation| {
                relation.purpose() == SourceNavigationOrderPurpose::Conditional
                    && relation.modeled_source_exclusion().is_none()
            })
    );
    assert_eq!(
        exported_and_live(&order, before, foreign, end),
        (true, true)
    );
    // With no query point, as at a link, the document is read after it
    // completes normally: the same-document removal has run, a foreign
    // one is not ordered after the install.
    assert!(!unpositioned_live(&order, before, after));
    assert!(unpositioned_live(&order, before, foreign));
}

/// Whether an alias installed at `install` and removed at `removal` is
/// live for a consumer with no query point.
fn unpositioned_live(order: &RunOrder, install: RunPoint<'_>, removal: RunPoint<'_>) -> bool {
    use crate::namespace_import::{AliasEvent, AliasEventKind};
    let aliases = [
        AliasEvent {
            kind: AliasEventKind::Install,
            at: install,
        },
        AliasEvent {
            kind: AliasEventKind::Remove,
            at: removal,
        },
    ];
    crate::namespace_import::alias_live_at(&mut aliases.into_iter(), order, None)
}

#[test]
fn an_unpositioned_removal_applies_only_before_an_admitted_document_end() {
    // tclsh 8.6 / 9.0, with `namespace import` at `set install` and
    // `namespace forget` at `set removal`: after `source` the forget has
    // run and the alias is gone. Behind a possible early exit, after an
    // end-only cutoff, in a procedure body or a branch, or before the
    // install, the answer stays conditional and the alias stays live.
    for (source, live) in [
        ("set install 1\nset removal 2\n", false),
        (
            "namespace eval ::app {\n    set install 1\n    set removal 2\n}\n",
            false,
        ),
        ("set install 1\nif {$x} return\nset removal 2\n", true),
        ("set install 1\nset removal 2\nsource y.tcl\n", true),
        (
            "proc myreturn {} {return -code return}\nset install 1\nset removal 2\nmyreturn\n",
            true,
        ),
        ("set install 1\nproc q {} {set removal 2}\n", true),
        ("set install 1\nif {$x} {set removal 2}\n", true),
        ("set removal 2\nset install 1\n", true),
    ] {
        let analysis = Analyser::new().analyse(source, "tcl8.6");
        let context = SourceOrderContext::capture(&analysis).unwrap();
        let order = RunOrder::for_source_navigation(&[], [("doc".into(), context)].into_iter());
        let at = |needle: &str| point("doc", u32::try_from(source.find(needle).unwrap()).unwrap());
        assert_eq!(
            unpositioned_live(&order, at("set install"), at("set removal")),
            live,
            "{source}"
        );
    }
}

fn sequence_purposes(
    source: &str,
) -> impl Fn(&str, &str) -> Option<SourceNavigationOrderPurpose> + '_ {
    let analysis = Analyser::new().analyse(source, "tcl8.6");
    let context = SourceOrderContext::capture(&analysis).unwrap();
    let order = RunOrder::for_source_navigation(&[], [("doc".into(), context)].into_iter());
    let at = move |needle: &str| {
        if needle == "END" {
            return u32::try_from(source.len()).unwrap();
        }
        u32::try_from(source.find(needle).unwrap()).unwrap()
    };
    move |a: &str, b: &str| {
        order
            .navigation_precedence(point("doc", at(a)), point("doc", at(b)))
            .map(|relation| relation.purpose())
    }
}

const KNOWN: Option<SourceNavigationOrderPurpose> =
    Some(SourceNavigationOrderPurpose::DocumentSequence);

#[test]
fn document_sequence_orders_straight_line_bodies_and_withholds_branches() {
    // Only top-level unconditional order is known. A `namespace eval`
    // body is entered once, immediately; an `if` branch and a procedure
    // body are not straight-line sequence. A `return` inside a procedure
    // body is no early exit of the document, and neither is a branch
    // that may or may not run. `x` holds an unknown value read from stdin.
    let source = "set before 1\ngets stdin x\nnamespace eval ::n {\n    set inner 2\n}\nif {$x} {set branch 3}\nproc q {} {set body 4; return}\nset after 6\n";
    let purpose = sequence_purposes(source);
    assert_eq!(purpose("set before", "set inner"), KNOWN);
    assert_eq!(purpose("set inner", "set after"), KNOWN);
    assert_eq!(purpose("set after", "END"), KNOWN);
    // tclsh 8.6 / 9.0: with `x` never set, `if {$x}` fails with
    // `can't read "x"`, so the document never completes normally and
    // the later commands are never reached; the end stays withheld.
    let unset = sequence_purposes(
        "set before 1\nif {$x} {set branch 3}\nproc q {} {set body 4; return}\nset after 6\n",
    );
    assert_ne!(unset("set after", "END"), KNOWN);
    for withheld in ["set branch", "set body"] {
        assert_ne!(purpose(withheld, "set after"), KNOWN, "{withheld}");
        assert_ne!(purpose("set before", withheld), KNOWN, "{withheld}");
    }
    // `uplevel` selects its frame from a level word: never entered.
    let shifted = sequence_purposes("set before 1\nuplevel #0 {set shifted 2}\n");
    assert_ne!(shifted("set before", "set shifted"), KNOWN);
    assert_ne!(shifted("set shifted", "END"), KNOWN);
    let analysis = Analyser::new().analyse(source, "tcl8.6");
    let context = SourceOrderContext::capture(&analysis).unwrap();
    let order = RunOrder::for_source_navigation(&[], [("doc".into(), context)].into_iter());
    assert!(
        order
            .navigation_precedence(point("doc", 0), point("elsewhere", 0))
            .is_none()
    );
}

#[test]
fn document_sequence_ends_at_a_possible_early_exit() {
    // tclsh 8.6 / 9.0: a top-level `return`, unconditional or in a taken
    // branch, stops the document; nothing after it is a known position
    // and neither is the document end.
    for source in [
        "set before 1\nreturn\nset after 2\n",
        "set before 1\nif {$x} return\nset after 2\n",
        "set before 1\nif {$x} {exit 1}\nset after 2\n",
    ] {
        let purpose = sequence_purposes(source);
        assert_ne!(purpose("set before", "set after"), KNOWN, "{source}");
        assert_ne!(purpose("set before", "END"), KNOWN, "{source}");
    }
}

#[test]
fn document_end_is_after_normal_completion_only() {
    // tclsh 8.6 / 9.0: a sourced file or a user procedure doing
    // `return -code return`, or a top-level `break` in a file sourced
    // inside a loop, completes the load early and the later statement
    // never runs. A later position is still ordered (it is reached only
    // when control did pass); the document end is withheld.
    for source in [
        "set before 1\nsource y.tcl\nset after 2\n",
        "proc myreturn {} {return -code return}\nset before 1\nmyreturn\nset after 2\n",
        "set before 1\nbreak\nset after 2\n",
        // A script the inventory cannot see runs now: a lambda in a
        // variable, a command prefix, a doubly substituted expression.
        "set l {{} {return -code return}}\nset before 1\napply $l\nset after 2\n",
        "proc cmp {a b} {return -code return}\nset before 1\nlsort -command cmp {b a}\nset after 2\n",
        "set e {[return -code return]}\nset before 1\nexpr $e\nset after 2\n",
        // Math functions are commands: a user `::tcl::mathfunc`
        // procedure, also from a braced `if` condition, or an undefined
        // name reaching a user `unknown`.
        "proc ::tcl::mathfunc::f {x} {return -code return}\nset before 1\nexpr {f(1)}\nset after 2\n",
        "proc ::tcl::mathfunc::f {x} {return -code return}\nset before 1\nif {f(1)} {set y 1}\nset after 2\n",
        "proc unknown {args} {return -code return}\nset before 1\nexpr {nosuchfn(1)}\nset after 2\n",
    ] {
        let purpose = sequence_purposes(source);
        assert_eq!(purpose("set before", "set after"), KNOWN, "{source}");
        assert_ne!(purpose("set before", "END"), KNOWN, "{source}");
        assert_ne!(purpose("set after", "END"), KNOWN, "{source}");
    }
}

#[test]
fn document_sequence_refuses_shadowed_or_namespace_local_entering_commands() {
    // tclsh 8.6 / 9.0: a namespace-local `proc eval` or `proc namespace`
    // swallows the body, so the forget never runs; neither body is
    // straight-line sequence.
    for source in [
        "namespace eval dst {proc eval args {}}\nnamespace eval dst {eval {namespace forget ::src::p}}\nset after 1\n",
        "namespace eval dst {proc namespace args {}}\nnamespace eval dst {namespace forget ::src::p}\nset after 1\n",
        "proc eval args {}\neval {namespace forget ::src::p}\nset after 1\n",
        // A renamed procedure takes the built-in's name without a
        // declaration of that name; the Realm barrier must refuse it.
        "proc myeval args {}\nrename eval {}\nrename myeval eval\neval {namespace forget ::src::p}\nset after 1\n",
    ] {
        let purpose = sequence_purposes(source);
        assert_ne!(purpose("namespace forget", "set after"), KNOWN, "{source}");
        assert_ne!(purpose("namespace forget", "END"), KNOWN, "{source}");
    }
}

#[test]
fn original_source_order_context_keeps_full_input_and_missing_body_owners_unknown() {
    let source = "set value 1\n";
    let analysis = Analyser::new().analyse(source, "tcl8.6");
    assert!(SourceOrderContext::capture(&analysis).is_some());
    let mut changed = analysis.clone();
    changed.body_lexer_config.as_mut().unwrap().strict_quoting ^= true;
    assert!(SourceOrderContext::capture(&changed).is_none());
    let mut missing = analysis.clone();
    missing.resolved_input = None;
    assert!(SourceOrderContext::capture(&missing).is_none());
    let input = analysis.resolved_input.as_ref().unwrap();
    let mut foreign = analysis.clone();
    foreign.resolved_input = Some(tcl_compiler::analyser::ResolvedAnalysisInput::new(
        input.analyser_profile(),
        input.unit_profile(),
        tcl_registry::model::ingress::resolve_environment("tcl8.4").default_context_registry(),
        input.lexer_config(),
    ));
    assert!(SourceOrderContext::capture(&foreign).is_none());
    let dynamic = "$head arbitrary\n";
    let dynamic_analysis = Analyser::new().analyse(dynamic, "tcl8.6");
    let context = SourceOrderContext::capture(&dynamic_analysis).unwrap();
    assert!(context.site(0).is_none());
    let order = RunOrder::for_source_navigation(&[], std::iter::empty());
    assert!(
        order
            .navigation_precedence(point("doc", 0), point("doc", 1))
            .is_none()
    );
    assert_eq!(order.has_run(point("doc", 0), point("doc", 1)), None);

    let source = "proc pending {} {set body 1}\nset root 2\n";
    let mut analysis = Analyser::new().analyse(source, "tcl8.6");
    let body_at = u32::try_from(source.find("set body").unwrap()).unwrap();
    assert!(analysis.original_definition_body_span(body_at).is_some());
    let context = SourceOrderContext::capture(&analysis).unwrap();
    if let Some(site) = context.site(body_at) {
        assert_eq!(site.body, analysis.original_definition_body_span(body_at));
    }
    let original_body = analysis.original_definition_body_span(body_at);
    analysis.all_procs.clear();
    // Reporting-map deletion neither erases the independent original body
    // nor manufactures a source-global point from it.
    assert_eq!(
        analysis.original_definition_body_span(body_at),
        original_body
    );
    let context = SourceOrderContext::capture(&analysis).unwrap();
    if let Some(site) = context.site(body_at) {
        assert_eq!(site.body, original_body);
    }
}

#[test]
fn original_source_graph_uses_real_points_and_preserves_cycle_and_re_source_refusal() {
    let parent = "set before 1\nsource child.tcl\nsource child.tcl\nset after 2\n";
    let child = "set child 1\nsource parent.tcl\n";
    let parent_analysis = Analyser::new().analyse(parent, "tcl8.6");
    let child_analysis = Analyser::new().analyse(child, "tcl8.6");
    let at = u32::try_from(parent.find("source child").unwrap()).unwrap();
    let contexts = || {
        [
            (
                "parent".into(),
                SourceOrderContext::capture(&parent_analysis).unwrap(),
            ),
            (
                "child".into(),
                SourceOrderContext::capture(&child_analysis).unwrap(),
            ),
        ]
        .into_iter()
    };
    let edge = RunEdge {
        parent: "parent".into(),
        child: "child".into(),
        at,
        enclosing_body: Some(tcl_lexer::Span::new(
            0,
            u32::try_from(parent.len()).unwrap(),
        )),
        kind: RunEdgeKind::Source,
    };
    let order = RunOrder::for_source_navigation(std::slice::from_ref(&edge), contexts());
    // Supplied report body geometry cannot replace the genuine root point.
    assert_eq!(order.paths.get("child").unwrap()[0].enclosing_body, None);
    let relation = order
        .navigation_precedence(point("parent", 0), point("child", 0))
        .unwrap();
    assert!(*relation.conditional_relation());
    assert_eq!(relation.modeled_source_exclusion(), None);
    let re_source = RunEdge {
        at: u32::try_from(parent.rfind("source child").unwrap()).unwrap(),
        ..edge.clone()
    };
    let order = RunOrder::for_source_navigation(&[edge.clone(), re_source], contexts());
    assert!(
        order
            .navigation_precedence(point("parent", 0), point("child", 0))
            .is_none()
    );
    let cycle = RunEdge {
        parent: "child".into(),
        child: "parent".into(),
        at: u32::try_from(child.find("source parent").unwrap()).unwrap(),
        enclosing_body: None,
        kind: RunEdgeKind::Source,
    };
    let order = RunOrder::for_source_navigation(&[edge, cycle], contexts());
    assert!(
        order
            .navigation_comparison(point("parent", 0), point("child", 0))
            .is_none()
    );
}
