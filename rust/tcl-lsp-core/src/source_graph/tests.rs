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

fn reqs(pairs: &[(&str, &[&str])]) -> HashMap<String, Vec<String>> {
    pairs
        .iter()
        .map(|(uri, pkgs)| {
            (
                (*uri).to_owned(),
                pkgs.iter().map(|p| (*p).to_owned()).collect(),
            )
        })
        .collect()
}

#[test]
fn resolves_relative_source_against_parent_dir() {
    let got = resolve_source_target(Path::new("/proj/app.tcl"), "lib/util.tcl");
    assert_eq!(got, PathBuf::from("/proj/lib/util.tcl"));
}

#[test]
fn resolves_dot_and_dotdot_segments() {
    let got = resolve_source_target(Path::new("/proj/sub/main.tcl"), "../lib/./util.tcl");
    assert_eq!(got, PathBuf::from("/proj/lib/util.tcl"));
}

#[test]
fn absolute_source_passes_through() {
    let got = resolve_source_target(Path::new("/proj/app.tcl"), "/opt/x/y.tcl");
    assert_eq!(got, PathBuf::from("/opt/x/y.tcl"));
}

#[test]
fn inherits_requires_from_direct_parent() {
    // app.tcl requires Tk and sources lib.tcl.
    let edges = vec![("app".to_owned(), "lib".to_owned())];
    let requires = reqs(&[("app", &["Tk", "http"]), ("lib", &[])]);
    let got = ancestor_requires("lib", &edges, &requires);
    assert_eq!(got, vec!["Tk".to_owned(), "http".to_owned()]);
}

#[test]
fn inherits_transitively_through_a_chain() {
    // app -> mid -> leaf; leaf inherits from both app and mid.
    let edges = vec![
        ("app".to_owned(), "mid".to_owned()),
        ("mid".to_owned(), "leaf".to_owned()),
    ];
    let requires = reqs(&[("app", &["Tk"]), ("mid", &["json"]), ("leaf", &[])]);
    let mut got = ancestor_requires("leaf", &edges, &requires);
    got.sort();
    assert_eq!(got, vec!["Tk".to_owned(), "json".to_owned()]);
}

#[test]
fn unions_requires_from_multiple_entry_points() {
    // Both a.tcl and b.tcl source shared.tcl with different requires.
    let edges = vec![
        ("a".to_owned(), "shared".to_owned()),
        ("b".to_owned(), "shared".to_owned()),
    ];
    let requires = reqs(&[("a", &["Tk"]), ("b", &["http"]), ("shared", &[])]);
    let got = ancestor_requires("shared", &edges, &requires);
    assert_eq!(got, vec!["Tk".to_owned(), "http".to_owned()]);
}

#[test]
fn tolerates_cycles() {
    // a -> b -> a (pathological); must not loop forever.
    let edges = vec![
        ("a".to_owned(), "b".to_owned()),
        ("b".to_owned(), "a".to_owned()),
    ];
    let requires = reqs(&[("a", &["Tk"]), ("b", &["http"])]);
    let got = ancestor_requires("a", &edges, &requires);
    assert_eq!(got, vec!["Tk".to_owned(), "http".to_owned()]);
}

#[test]
fn a_root_with_no_ancestors_inherits_nothing() {
    let edges = vec![("app".to_owned(), "lib".to_owned())];
    let requires = reqs(&[("app", &["Tk"])]);
    assert_eq!(
        ancestor_requires("app", &edges, &requires),
        [] as [std::string::String; 0]
    );
}

// `package prefer latest` across the source graph.
//
// tclsh-proof (8.6.14) that the latch really is interpreter-global and
// crosses `source`:  with `lib.tcl` holding `puts [package prefer]`,
//
//   # app.tcl:  package prefer latest ; source lib.tcl
//   -> latest
//   # app.tcl:  source lib.tcl ; package prefer latest
//   -> stable

fn edge(parent: &str, child: &str, at: u32) -> RunEdge {
    RunEdge {
        parent: parent.to_owned(),
        child: child.to_owned(),
        at,
        enclosing_body: None,
        kind: RunEdgeKind::Source,
    }
}

fn raises(pairs: &[(&str, &[u32])]) -> HashMap<String, Vec<u32>> {
    pairs
        .iter()
        .map(|(uri, offs)| ((*uri).to_owned(), offs.to_vec()))
        .collect()
}

/// TP: a raise written **above** the `source` is in force in the child.
#[test]
fn a_raise_before_the_source_reaches_the_child() {
    let edges = vec![edge("app", "lib", 30)];
    assert!(ancestor_prefer_latest_raised(
        "lib",
        &edges,
        &raises(&[("app", &[0])])
    ));
}

/// TN: a raise written **below** the `source` has not run when the child
/// loads, so the child keeps the default.
#[test]
fn a_raise_after_the_source_does_not_reach_the_child() {
    let edges = vec![edge("app", "lib", 0)];
    assert!(!ancestor_prefer_latest_raised(
        "lib",
        &edges,
        &raises(&[("app", &[30])])
    ));
}

/// TP: the latch is transitive — once up on entry to a node it is up for
/// everything that node sources, wherever the `source` sits.
#[test]
fn the_latch_travels_the_whole_graph_once_raised() {
    let edges = vec![edge("app", "mid", 30), edge("mid", "leaf", 0)];
    assert!(ancestor_prefer_latest_raised(
        "leaf",
        &edges,
        &raises(&[("app", &[0])])
    ));
}

/// TN: a raise in a node the target is **not** reachable from changes
/// nothing — the graph, not the workspace, decides.
#[test]
fn an_unrelated_raise_does_not_reach_the_target() {
    let edges = vec![edge("app", "lib", 30), edge("other", "sibling", 30)];
    assert!(!ancestor_prefer_latest_raised(
        "lib",
        &edges,
        &raises(&[("other", &[0])])
    ));
}

/// TP: a `source` written inside a proc body still sees a raise written
/// later at the parent's load level — the whole file loads before any
/// body runs, the `in_effect_within` rule every other cross-document
/// ordering question uses.
#[test]
fn a_body_source_sees_a_later_load_level_raise() {
    let edges = vec![RunEdge {
        parent: "app".to_owned(),
        child: "lib".to_owned(),
        at: 10,
        enclosing_body: Some(tcl_lexer::Span::new(0, 20)),
        kind: RunEdgeKind::Source,
    }];
    assert!(ancestor_prefer_latest_raised(
        "lib",
        &edges,
        &raises(&[("app", &[30])])
    ));
    // …but a raise inside that same body, after the `source`, has not run.
    assert!(!ancestor_prefer_latest_raised(
        "lib",
        &edges,
        &raises(&[("app", &[15])])
    ));
}

/// A cycle terminates, and the target is still answered.
#[test]
fn the_prefer_walk_tolerates_cycles() {
    let edges = vec![edge("a", "b", 30), edge("b", "a", 0)];
    assert!(ancestor_prefer_latest_raised(
        "b",
        &edges,
        &raises(&[("a", &[0])])
    ));
    assert!(!ancestor_prefer_latest_raised("b", &edges, &raises(&[])));
}

/// TN: the target's *own* raises are not this question — that is the
/// position-sensitive single-document answer.
#[test]
fn the_targets_own_raise_is_not_an_ancestor_raise() {
    let edges = vec![edge("app", "lib", 30)];
    assert!(!ancestor_prefer_latest_raised(
        "lib",
        &edges,
        &raises(&[("lib", &[0])])
    ));
}

// The `source`-graph load order.
//
// Oracle for the whole shape, byte-identical on tclsh 8.6.14 and 9.0.4 —
// `source` inlines the sourced file's whole body at the `source`
// statement's position, so the DFS of the source forest is the run order:
//
//   # src.tcl:  namespace eval ::src { proc p {} {return P} }
//   # exp.tcl:  namespace eval ::src { namespace export p }
//   # imp.tcl:  namespace eval ::dst { namespace import ::src::* }
//
//   # app.tcl:  source src.tcl ; source exp.tcl ; source imp.tcl
//   ::dst::p   -> P                       (export ran before the import)
//   # app.tcl:  source src.tcl ; source imp.tcl ; source exp.tcl
//   ::dst::p   -> invalid command name "::dst::p"   (it did not)

fn point(uri: &str, at: u32) -> RunPoint<'_> {
    RunPoint {
        uri,
        at,
        enclosing_body: None,
    }
}

fn body_point(uri: &str, at: u32, body: (u32, u32)) -> RunPoint<'_> {
    RunPoint {
        uri,
        at,
        enclosing_body: Some(tcl_lexer::Span::new(body.0, body.1)),
    }
}

/// TP: two files sourced by one parent are ordered by their `source`
/// statements — everything in the first has run when the second loads.
#[test]
fn siblings_are_ordered_by_their_source_statements() {
    let order = RunOrder::build(&[edge("app", "exp", 10), edge("app", "imp", 20)]);
    assert_eq!(
        order.has_run(point("exp", 999), point("imp", 0)),
        Some(true),
        "exp.tcl is sourced first, so all of it has run when imp.tcl loads",
    );
    assert_eq!(
        order.has_run(point("imp", 0), point("exp", 999)),
        Some(false),
        "…and nothing in imp.tcl has run while exp.tcl is still loading",
    );
}

/// TP: a parent statement written above the `source` has run inside the
/// child; one written below has not.
#[test]
fn a_parent_statement_is_ordered_against_the_child_body() {
    let order = RunOrder::build(&[edge("app", "lib", 50)]);
    assert_eq!(order.has_run(point("app", 10), point("lib", 5)), Some(true));
    assert_eq!(
        order.has_run(point("app", 90), point("lib", 5)),
        Some(false)
    );
    // …and the child's own statements have run for a parent statement
    // written below the `source`.
    assert_eq!(order.has_run(point("lib", 5), point("app", 90)), Some(true));
    assert_eq!(
        order.has_run(point("lib", 5), point("app", 10)),
        Some(false)
    );
}

/// TP: the leniency at the common document is `in_effect_within`'s — a
/// child sourced from a parent's load level has run for a query inside
/// one of that parent's proc bodies, however far below the `source` is
/// written.
#[test]
fn a_body_local_query_observes_a_later_top_level_source() {
    let order = RunOrder::build(&[edge("app", "lib", 900)]);
    assert_eq!(
        order.has_run(point("lib", 5), body_point("app", 10, (0, 20))),
        Some(true),
        "the whole file loads — sourcing lib.tcl included — before any body runs",
    );
    // …but a `source` written inside that same body, after the query,
    // is an ordinary later statement of the running script.
    let order = RunOrder::build(&[RunEdge {
        parent: "app".to_owned(),
        child: "lib".to_owned(),
        at: 15,
        enclosing_body: Some(tcl_lexer::Span::new(0, 20)),
        kind: RunEdgeKind::Source,
    }]);
    assert_eq!(
        order.has_run(point("lib", 5), body_point("app", 10, (0, 20))),
        Some(false),
    );
}

/// TP: a grandchild is ordered against a sibling of its parent, through
/// the deepest common document.
#[test]
fn a_grandchild_is_ordered_through_the_common_document() {
    // app sources exp @10, then mid @20; mid sources leaf @5.
    let order = RunOrder::build(&[
        edge("app", "exp", 10),
        edge("app", "mid", 20),
        edge("mid", "leaf", 5),
    ]);
    assert_eq!(
        order.has_run(point("exp", 999), point("leaf", 0)),
        Some(true)
    );
    assert_eq!(
        order.has_run(point("leaf", 0), point("exp", 999)),
        Some(false)
    );
}

/// TN: documents in different trees have no order — the pre-source-graph
/// behaviour, and what an unrelated pair of files still gets.
#[test]
fn different_trees_are_incomparable() {
    let order = RunOrder::build(&[edge("app", "lib", 10), edge("other", "helper", 10)]);
    assert_eq!(order.has_run(point("lib", 0), point("helper", 0)), None);
    assert_eq!(order.cmp_run(point("lib", 0), point("helper", 0)), None);
}

/// TN: an empty graph orders nothing across documents…
#[test]
fn an_empty_graph_orders_nothing_across_documents() {
    let order = RunOrder::default();
    assert_eq!(order.has_run(point("a", 0), point("b", 0)), None);
    // …and a document the graph has never heard of is equally unordered.
    let order = RunOrder::build(&[edge("app", "lib", 10)]);
    assert_eq!(order.has_run(point("lib", 0), point("stray", 0)), None);
}

/// TP: two statements in the *same* document stay ordered regardless of
/// the graph — the rule that predates this type, unchanged.
#[test]
fn one_document_is_ordered_without_the_graph() {
    let order = RunOrder::default();
    assert_eq!(order.has_run(point("a", 10), point("a", 20)), Some(true));
    assert_eq!(order.has_run(point("a", 20), point("a", 10)), Some(false));
    // Even for a re-sourced file, whose cross-document position abstains.
    let order = RunOrder::build(&[edge("app", "lib", 10), edge("other", "lib", 10)]);
    assert_eq!(
        order.has_run(point("lib", 10), point("lib", 20)),
        Some(true)
    );
}

/// TN (CRITICAL): a file reachable from two different `source` sites has
/// no unique position, so every cross-document comparison touching it
/// abstains — Tcl tolerates re-sourcing, and inventing an order would
/// silently drop references.
#[test]
fn a_re_sourced_document_has_no_position() {
    let order = RunOrder::build(&[edge("app", "lib", 10), edge("app", "lib", 90)]);
    assert_eq!(order.has_run(point("lib", 0), point("app", 50)), None);
    // Two different parents, same story.
    let order = RunOrder::build(&[edge("app", "lib", 10), edge("tool", "lib", 10)]);
    assert_eq!(order.has_run(point("lib", 0), point("app", 50)), None);
}

/// TN: the ambiguity is inherited — a document below a re-sourced one has
/// no unique position either.
#[test]
fn ambiguity_is_inherited_by_descendants() {
    let order = RunOrder::build(&[
        edge("app", "lib", 10),
        edge("app", "lib", 90),
        edge("lib", "deep", 5),
    ]);
    assert_eq!(order.has_run(point("deep", 0), point("app", 50)), None);
}

/// TN: a `source` cycle gives nothing on it a position, and terminates.
#[test]
fn a_source_cycle_abstains() {
    let order = RunOrder::build(&[edge("a", "b", 10), edge("b", "a", 10)]);
    assert_eq!(order.has_run(point("a", 0), point("b", 0)), None);
    // A self-`source` likewise.
    let order = RunOrder::build(&[edge("a", "a", 10), edge("a", "b", 20)]);
    assert_eq!(order.has_run(point("b", 0), point("a", 50)), None);
}

/// `cmp_run` is the strict order: a body-local statement never counts as
/// having run at another document's load level, where `has_run`'s
/// leniency says it may have.
#[test]
fn cmp_run_is_strict_where_has_run_is_lenient() {
    let order = RunOrder::build(&[edge("app", "lib", 900)]);
    let body_stmt = body_point("app", 10, (0, 20));
    // Lenient: the body statement is treated as having run for a query in
    // the sourced child (the body may have been entered).
    assert_eq!(order.has_run(body_stmt, point("lib", 5)), Some(true));
    // Strict: it is *not* known to have run before the child loaded.
    assert_eq!(
        order.cmp_run(body_stmt, point("lib", 5)),
        Some(std::cmp::Ordering::Greater),
        "a body-local statement sorts after every load-level one",
    );
    // Two load-level statements in different files rank by their `source`
    // statements, which is genuine execution order.
    let order = RunOrder::build(&[edge("app", "one", 10), edge("app", "two", 20)]);
    assert_eq!(
        order.cmp_run(point("one", 999), point("two", 0)),
        Some(std::cmp::Ordering::Less),
    );
}

/// Nothing runs before itself, at either level — what keeps an import
/// from conflicting with itself once the order spans documents.
#[test]
fn a_point_does_not_precede_itself() {
    let order = RunOrder::build(&[edge("app", "lib", 10)]);
    assert_eq!(
        order.cmp_run(point("lib", 7), point("lib", 7)),
        Some(std::cmp::Ordering::Equal),
    );
    assert_eq!(order.has_run(point("lib", 7), point("lib", 7)), Some(false));
}

// The `package require` half of the load order.
//
// Oracle, byte-identical on tclsh 8.6.14 and 9.0.4.  `pkg/lib.tcl` holds
//
//   puts "lib.tcl body running"
//   namespace eval ::lib { proc p {} {return P} ; namespace export p }
//   package provide mylib 1.0
//
// (1) A require IS a load event, and the provider's whole body — export
//     included — has run by the time it returns:
//
//       # app.tcl
//       before require: lib not loaded
//       lib.tcl body running
//       after require:  lib loaded
//       app::p -> P
//
// (2) …but only for whichever require runs first.  `a.tcl` and `b.tcl`
//     hold the SAME two lines — a probe, then `package require mylib` —
//     and a driver sources a.tcl then b.tcl:
//
//       a.tcl: before its own require, is lib loaded? NO
//       lib.tcl body running
//       b.tcl: before its own require, is lib loaded? YES
//
//     Identical text, opposite answers.  So a require bounds the
//     provider's position from above and pins nothing.

fn require_edge(parent: &str, child: &str, at: u32) -> RunEdge {
    RunEdge {
        kind: RunEdgeKind::PackageRequire,
        ..edge(parent, child, at)
    }
}

/// TP: the provider's statements have run for every query after the
/// `package require` — oracle case (1), `app::p -> P`.
#[test]
fn a_require_proves_the_provider_has_already_run() {
    let order = RunOrder::build(&[require_edge("app", "lib", 10)]);
    assert_eq!(
        order.has_run(point("lib", 30), point("app", 50)),
        Some(true),
        "`package require` at 10 leaves the provider loaded for the import at 50",
    );
    // …and the mirror: the requiring document's later statement did not
    // run before the provider's, which the same bound settles.
    assert_eq!(
        order.has_run(point("app", 50), point("lib", 30)),
        Some(false),
    );
}

/// TP: the provider's *transitive* files ride the same bound — a file the
/// provider `source`s runs when the provider does.
#[test]
fn the_bound_covers_what_the_provider_sources() {
    let order = RunOrder::build(&[require_edge("app", "lib", 10), edge("lib", "deep", 5)]);
    assert_eq!(
        order.has_run(point("deep", 0), point("app", 50)),
        Some(true)
    );
    assert_eq!(
        order.has_run(point("app", 50), point("deep", 0)),
        Some(false)
    );
}

/// TP: one provider required from many files still ranks — the whole
/// point of a bound rather than a position.  A `source` child entered
/// twice is ambiguous; a `package require`d provider is not, because no
/// require claims the provider ran *there*.
#[test]
fn many_requires_of_one_package_all_bound_it() {
    let order = RunOrder::build(&[
        require_edge("a", "lib", 10),
        require_edge("b", "lib", 10),
        require_edge("c", "lib", 10),
    ]);
    for doc in ["a", "b", "c"] {
        assert_eq!(
            order.has_run(point("lib", 30), point(doc, 50)),
            Some(true),
            "{doc}'s own require bounds the provider for {doc}'s later statements",
        );
    }
}

/// ABSTENTION 1 (`auto_path` is mutable) is the caller's — it never emits
/// an edge for a package two documents provide — but the order must not
/// invent one from the *other* files' requires either: a document with no
/// require of its own is not ordered against the provider.
#[test]
fn a_document_that_does_not_require_the_package_is_not_ordered_by_it() {
    let order = RunOrder::build(&[require_edge("a", "lib", 10)]);
    assert_eq!(
        order.has_run(point("lib", 30), point("bystander", 50)),
        None
    );
    assert_eq!(
        order.cmp_run(point("lib", 30), point("bystander", 50)),
        None
    );
}

/// ABSTENTION 2 (CRITICAL — the package may already be loaded): a query
/// written **above** the `package require` gets `None`, never
/// `Some(false)`.
///
/// Oracle case (2) above: `b.tcl`'s probe, written above its own require,
/// answers YES because `a.tcl` loaded the package first — while the
/// byte-identical probe in `a.tcl` answers NO.  A static `Some(false)`
/// here would revoke a `namespace forget` / `-clear` that really had run.
#[test]
fn a_require_never_proves_the_provider_has_not_run_yet() {
    let order = RunOrder::build(&[require_edge("app", "lib", 10)]);
    assert_eq!(
        order.has_run(point("lib", 30), point("app", 5)),
        None,
        "another file may have required the package before app.tcl started",
    );
    assert_eq!(
        order.has_run(point("app", 5), point("lib", 30)),
        None,
        "…so app.tcl's earlier statement is not known to precede the provider either",
    );
    // A `source` edge in the same shape *does* answer both, which is what
    // makes this an abstention rather than a limitation of the machinery.
    let sourced = RunOrder::build(&[edge("app", "lib", 10)]);
    assert_eq!(
        sourced.has_run(point("lib", 30), point("app", 5)),
        Some(false)
    );
    assert_eq!(
        sourced.has_run(point("app", 5), point("lib", 30)),
        Some(true)
    );
}

/// TN: two documents each brought in by a `package require` are ranked
/// against each other exactly as before — not at all.  Both sides are
/// bounds, and two bounds settle nothing.
#[test]
fn two_package_loaded_documents_are_incomparable() {
    let order = RunOrder::build(&[
        require_edge("app", "one", 10),
        require_edge("app", "two", 20),
    ]);
    assert_eq!(order.has_run(point("one", 0), point("two", 0)), None);
    assert_eq!(order.has_run(point("two", 0), point("one", 0)), None);
    assert_eq!(order.cmp_run(point("one", 0), point("two", 0)), None);
    // The `source` shape of the same workspace *is* ordered — the
    // difference is the load event, not the file layout.
    let sourced = RunOrder::build(&[edge("app", "one", 10), edge("app", "two", 20)]);
    assert_eq!(
        sourced.has_run(point("one", 999), point("two", 0)),
        Some(true)
    );
}

/// TN: a document that is both `source`d and `package require`d has two
/// incompatible stories — the `source` says "it ran exactly there", the
/// require says "it had already run" — so it gets neither.
#[test]
fn a_document_both_sourced_and_required_is_ambiguous() {
    let order = RunOrder::build(&[edge("app", "lib", 90), require_edge("app", "lib", 10)]);
    assert_eq!(order.has_run(point("lib", 0), point("app", 50)), None);
    assert_eq!(order.has_run(point("app", 50), point("lib", 0)), None);
}

/// TP: the bound is judged by `in_effect_within` like any other statement
/// — a `package require` at a document's load level counts for a query
/// inside one of that document's proc bodies, however far below it is
/// written, because the whole file loads before any body runs.
#[test]
fn a_body_local_query_observes_a_later_top_level_require() {
    let order = RunOrder::build(&[require_edge("app", "lib", 900)]);
    assert_eq!(
        order.has_run(point("lib", 5), body_point("app", 10, (0, 20))),
        Some(true),
    );
    // …but a require written inside that same body, after the query, is
    // an ordinary later statement of the running script — and this time
    // the abstention applies, because "not yet" is what a bound cannot
    // say.
    let order = RunOrder::build(&[RunEdge {
        kind: RunEdgeKind::PackageRequire,
        ..RunEdge {
            parent: "app".to_owned(),
            child: "lib".to_owned(),
            at: 15,
            enclosing_body: Some(tcl_lexer::Span::new(0, 20)),
            kind: RunEdgeKind::Source,
        }
    }]);
    assert_eq!(
        order.has_run(point("lib", 5), body_point("app", 10, (0, 20))),
        None,
    );
}

/// `cmp_run` inherits the same one-sidedness: the conflict rule may learn
/// that a provider's import ran *first*, never that it ran *second*.
#[test]
fn cmp_run_reads_only_the_half_the_bound_supports() {
    let order = RunOrder::build(&[require_edge("app", "lib", 10)]);
    assert_eq!(
        order.cmp_run(point("lib", 30), point("app", 50)),
        Some(std::cmp::Ordering::Less),
        "the provider's whole body precedes a statement below the require",
    );
    assert_eq!(
        order.cmp_run(point("app", 50), point("lib", 30)),
        Some(std::cmp::Ordering::Greater),
    );
    // Above the require, neither direction is a fact.
    assert_eq!(order.cmp_run(point("lib", 30), point("app", 5)), None);
    assert_eq!(order.cmp_run(point("app", 5), point("lib", 30)), None);
}

// Resolving, per call, "is there any package edge at all / is this
// document ambiguous / what is its root / does that root have require
// sites" costs three hash probes on owned URI strings, on the hottest
// path in the cross-document import walk.  All
// four questions are fixed at build time, so they are one probe of
// `package_alternatives`.  These pin that the folded answer is the same
// answer, case by case, because a silent change here would move which
// cross-document events the import tiers can rank.

/// TP: a document a `package require` loads, and everything under it,
/// gets the require sites; the require's own document does not.
#[test]
fn package_alternatives_cover_the_required_tree_and_nothing_else() {
    let order = RunOrder::build(&[require_edge("app", "lib", 10), edge("lib", "deep", 5)]);
    assert_eq!(order.alternatives(point("lib", 0)).len(), 1);
    assert_eq!(
        order.alternatives(point("deep", 0)).len(),
        1,
        "a file the provider sources rides the provider's bound",
    );
    assert!(
        order.alternatives(point("app", 0)).is_empty(),
        "the requiring document is not itself bounded by its own require",
    );
}

/// TN: no package edge anywhere leaves every document without
/// alternatives — the shape the old `package_entries.is_empty()` fast
/// path existed for, which must stay free of any behaviour change.
#[test]
fn a_workspace_with_no_package_edge_has_no_alternatives() {
    let order = RunOrder::build(&[edge("a", "b", 10), edge("b", "c", 5)]);
    for doc in ["a", "b", "c"] {
        assert!(order.alternatives(point(doc, 0)).is_empty(), "{doc}");
    }
}

/// FP guard: an **ambiguous** document must get no alternatives even
/// though its root has require sites.  This was the `self.ambiguous`
/// test that stood in front of the old lookup; folding it into the build
/// must not lose it.  A document both `source`d and `package require`d is
/// exactly that case.
#[test]
fn an_ambiguous_document_gets_no_alternatives() {
    let order = RunOrder::build(&[require_edge("app", "lib", 10), edge("other", "lib", 3)]);
    assert!(
        order.alternatives(point("lib", 0)).is_empty(),
        "both sourced and required: two incompatible stories, so neither",
    );
    // …and ambiguity is inherited, so a file below it is barred too.
    let order = RunOrder::build(&[
        require_edge("app", "lib", 10),
        edge("other", "lib", 3),
        edge("lib", "deep", 1),
    ]);
    assert_eq!(order.alternatives(point("deep", 0)), []);
}

/// TN: a document the graph has never heard of has no alternatives, and
/// asking does not panic — the `map_or(p.uri, …)` fallback the old code
/// took, restated as a plain miss.
#[test]
fn an_unknown_document_has_no_alternatives() {
    let order = RunOrder::build(&[require_edge("app", "lib", 10)]);
    assert_eq!(order.alternatives(point("never-indexed", 0)), []);
    assert_eq!(RunOrder::default().alternatives(point("a", 0)), []);
}

/// TP: many requires of one provider all reach it — the per-document
/// fold shares one site vector rather than dropping the extras.
#[test]
fn every_require_site_survives_the_per_document_fold() {
    let order = RunOrder::build(&[
        require_edge("a", "lib", 10),
        require_edge("b", "lib", 20),
        require_edge("c", "lib", 30),
    ]);
    let sites = order.alternatives(point("lib", 0));
    assert_eq!(sites.len(), 3);
    let mut ats: Vec<u32> = sites.iter().map(|s| s.at).collect();
    ats.sort_unstable();
    assert_eq!(ats, vec![10, 20, 30]);
}

/// `trusted` is the whole rule in one place: an answer needs the side it
/// would break on to be pinned.
#[test]
fn trust_needs_the_side_the_answer_rests_on() {
    let exact = Placed {
        at: 0,
        enclosing_body: None,
        exact: true,
    };
    let bound = Placed {
        exact: false,
        ..exact
    };
    assert!(RunOrder::trusted(true, exact, exact));
    assert!(RunOrder::trusted(false, exact, exact));
    // "a first" survives `a` sliding earlier…
    assert!(RunOrder::trusted(true, bound, exact));
    assert!(!RunOrder::trusted(false, bound, exact));
    // …"a not first" survives `b` sliding earlier.
    assert!(RunOrder::trusted(false, exact, bound));
    assert!(!RunOrder::trusted(true, exact, bound));
    // Two bounds settle nothing.
    assert!(!RunOrder::trusted(true, bound, bound));
    assert!(!RunOrder::trusted(false, bound, bound));
}

// `descendant_requires` — the up direction
//
// Oracle for the whole group, C Tcl 9.0.4:
//
// ```
// % cat child.tcl
// package require msgcat
// % cat t1.tcl
// puts [expr {[lsearch -exact [package names] msgcat] >= 0}]  ;# 0
// source child.tcl
// puts [package present msgcat]                               ;# 1.7.1
// ```
//
// i.e. the package is absent before the `source` statement and present
// after it — which is why these results carry a position.

// Load-level `source` edges are built with the module-test `edge` helper
// defined above for the `ancestor_prefer_latest_raised` group — same
// shape, same meaning, so it is shared rather than duplicated.

/// The reported shape: `main.tcl` sources `tkFile.tcl`, which
/// requires Tk. Tk becomes available to `main` *at the source statement*.
#[test]
fn descendant_requires_carries_a_sourced_files_package_up() {
    let edges = vec![edge("main", "tk", 7)];
    let requires = reqs(&[("tk", &["Tk"])]);
    let got = descendant_requires("main", &edges, &requires);
    assert_eq!(got.len(), 1, "{got:?}");
    assert_eq!(got[0].name, "Tk");
    assert_eq!(
        got[0].at, 7,
        "the package arrives at the `source` statement, not at offset 0",
    );
}

/// A file that sources something requiring nothing contributes nothing —
/// the TN that keeps `source`-following from silencing W120 wholesale.
#[test]
fn descendant_requires_is_empty_for_a_sourced_file_with_no_requires() {
    let edges = vec![edge("main", "plain", 7)];
    let requires = reqs(&[("plain", &[])]);
    assert_eq!(
        descendant_requires("main", &edges, &requires),
        [] as [crate::source_graph::PlacedRequirement<std::string::String>; 0]
    );
}

/// Transitive: `main` → `mid` → `leaf`. `leaf`'s package reaches `main`,
/// attributed to the position of *`main`'s own* `source`, since that is
/// where the whole subtree enters `main`'s run.
#[test]
fn descendant_requires_is_transitive_and_placed_at_the_first_hop() {
    let edges = vec![edge("main", "mid", 12), edge("mid", "leaf", 400)];
    let requires = reqs(&[("leaf", &["Tk"])]);
    let got = descendant_requires("main", &edges, &requires);
    assert_eq!(got.len(), 1, "{got:?}");
    assert_eq!(got[0].name, "Tk");
    assert_eq!(
        got[0].at, 12,
        "a transitive package enters at the outermost hop's position, not \
             at an offset that belongs to a different document",
    );
}

/// A cycle terminates instead of spinning, and still reports what it found.
#[test]
fn descendant_requires_terminates_on_a_cycle() {
    let edges = vec![edge("a", "b", 5), edge("b", "a", 9)];
    let requires = reqs(&[("b", &["Tk"])]);
    let got = descendant_requires("a", &edges, &requires);
    assert_eq!(got.len(), 1, "{got:?}");
    assert_eq!(got[0].name, "Tk");
}

/// The target's *own* requires are the single-document question and are
/// never reported here — otherwise a file would inherit from itself and
/// the caller would double-count.
#[test]
fn descendant_requires_excludes_the_targets_own_requires() {
    let edges = vec![edge("main", "child", 7)];
    let requires = reqs(&[("main", &["http"]), ("child", &["Tk"])]);
    let got = descendant_requires("main", &edges, &requires);
    assert_eq!(
        got.iter().map(|p| p.name.as_str()).collect::<Vec<_>>(),
        ["Tk"]
    );
}

/// Only `source` edges are followed. A `package require` edge bounds when
/// a provider ran but says nothing about *this* document acquiring it, so
/// it must not contribute — otherwise every document requiring a package
/// would inherit every other requirer's packages.
#[test]
fn descendant_requires_ignores_package_require_edges() {
    let mut e = edge("main", "provider", 7);
    e.kind = RunEdgeKind::PackageRequire;
    let requires = reqs(&[("provider", &["Tk"])]);
    assert_eq!(
        descendant_requires("main", &[e], &requires),
        [] as [crate::source_graph::PlacedRequirement<std::string::String>; 0]
    );
}

/// A file sourced from two places contributes at both positions, so a
/// caller gating on offset gets the right answer at each.
#[test]
fn descendant_requires_reports_a_twice_sourced_file_at_both_positions() {
    let edges = vec![edge("main", "tk", 7), edge("main", "tk", 90)];
    let requires = reqs(&[("tk", &["Tk"])]);
    let got = descendant_requires("main", &edges, &requires);
    assert_eq!(
        got.iter().map(|p| p.at).collect::<Vec<_>>(),
        [7, 90],
        "{got:?}",
    );
}
