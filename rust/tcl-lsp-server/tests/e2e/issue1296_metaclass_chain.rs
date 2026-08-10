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

//! Issue #1296 — a class made by a cross-file metaclass that was itself made
//! by *another* file's metaclass.
//!
//! The workspace class-factory index (issue #1276) is computed from a query
//! that reads the index, so one publish only ever advances the metaclass chain
//! by one link. Four files — `MetaA`, `MetaA create MetaB`,
//! `MetaB create Widget`, and a call site on `Widget`'s method — need three
//! links, and go-to-definition on the method came back with nothing. Collapsing
//! the first two files into one (the same three-level chain, two documents)
//! resolved fine, which is what pins the fault on publish depth rather than on
//! the analyser.
//!
//! C-Tcl ground truth (tclsh 8.6.16 and 9.0.4, identical), sourcing the four
//! files in order:
//!
//! ```text
//! info object isa class ::MC::MetaB   -> 1
//! info class superclasses ::MC::MetaB -> ::oo::class
//! info object isa class ::MC::Widget  -> 1
//! info class methods ::MC::Widget     -> go
//! puts [$w go]                        -> went
//! ```
//!
//! so `::MC::Widget` genuinely has `go`, and a definition request on it has a
//! real answer to find.
//!
//! The same suite now covers the related factory facts that have to survive
//! the workspace boundary: unopened factory files (#1304), a metaclass whose
//! `unknown` method proves Tk-style bare-word construction (#1303), and a
//! metaclass name composed from a proc parameter whose top-level caller passes
//! a literal (#1306). Dynamic names and unproved factories remain abstentions;
//! the analyser never invents a class to make navigation succeed.
//!
//! Beware when probing it by hand: `.claude/skills/lsp-client/lsp_client.py`
//! sets `rootUri` from `--server-dir`, so a probe that points that at the repo
//! puts its fixtures outside the scanned workspace and *everything* cross-file
//! fails, a plain `proc` included. Pass `--server-dir <fixture dir>` with
//! `--server-bin <binary>` to separate the two.

use crate::common::helpers::*;
use crate::common::{Lsp, scaled_timeout, unique_uri};
use serde_json::Value;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};

static WORKSPACE_SEQ: AtomicU64 = AtomicU64::new(0);

/// The chain, one link per document.
const META_A: &str =
    "namespace eval ::MC {}\noo::class create ::MC::MetaA { superclass oo::class }\n";
const META_B: &str = "::MC::MetaA create ::MC::MetaB { superclass oo::class }\n";
const WIDGET: &str = "::MC::MetaB create ::MC::Widget { method go {} { return went } }\n";
const CALL: &str = "set w [::MC::Widget new]\nputs [$w go]\n";

/// How long to let the workspace's class-factory index converge.
///
/// The index is published by the diagnostics worker after each publish, and
/// each round of the publish invalidates every file's item tree — so the deeper
/// links of a chain land a few debounced passes after the last document opens,
/// not synchronously with it. This is the same "wait for the converged state
/// rather than the first publish" discipline the cross-file call-site tests use
/// (`caller_in_a_sourcing_file_with_a_differing_literal_clears_i230`), with the
/// same load scaling.
const CONVERGE: Duration = Duration::from_secs(25);

/// Poll `definition` at `(line, ch)` until it reports at least one location,
/// bounded. Returns the locations, or the last (empty) result at the deadline
/// so the assertion — not the helper — reports the failure.
fn definition_settled(lsp: &mut Lsp, uri: &str, line: u32, ch: u32) -> Vec<Loc> {
    let deadline = Instant::now() + scaled_timeout(CONVERGE);
    loop {
        let found = locations(&lsp.definition(uri, line, ch));
        if !found.is_empty() || Instant::now() >= deadline {
            return found;
        }
        std::thread::sleep(Duration::from_millis(100));
    }
}

/// TP — the ticket. Four documents, a three-link chain, and the call site's
/// method resolves to its declaration in the document that writes the class.
#[test]
fn a_three_level_cross_file_metaclass_chain_resolves_from_a_call_site() {
    let mut lsp = Lsp::tcl();
    let meta_a = unique_uri("tcl");
    let meta_b = unique_uri("tcl");
    let widget = unique_uri("tcl");
    let call = unique_uri("tcl");
    lsp.open_ready(&meta_a, META_A);
    lsp.open_ready(&meta_b, META_B);
    lsp.open_ready(&widget, WIDGET);
    lsp.open_ready(&call, CALL);

    // `puts [$w go]` — `go` starts at character 10 of line 1.
    let found = definition_settled(&mut lsp, &call, 1, 10);
    assert_eq!(
        found.len(),
        1,
        "`$w go` must resolve to `::MC::Widget`'s method: {found:?}"
    );
    assert_eq!(
        found[0].uri, widget,
        "the target is the document that writes the class: {found:?}"
    );
    assert_eq!(
        found[0]
            .range
            .get("start")
            .and_then(|s| s.get("line"))
            .and_then(Value::as_i64),
        Some(0),
        "and the method's own line: {found:?}"
    );
}

/// TP control — the two-document form of the same three-level chain (the
/// ticket's "collapse `b_meta.tcl` into `a_meta.tcl`" row, which resolved
/// before the fix). It must keep resolving, and it is what tells a regression
/// in publish *depth* apart from one in the analyser.
#[test]
fn the_same_chain_in_two_documents_still_resolves() {
    let mut lsp = Lsp::tcl();
    let meta = unique_uri("tcl");
    let widget = unique_uri("tcl");
    let call = unique_uri("tcl");
    lsp.open_ready(&meta, &format!("{META_A}{META_B}"));
    lsp.open_ready(&widget, WIDGET);
    lsp.open_ready(&call, CALL);

    let found = definition_settled(&mut lsp, &call, 1, 10);
    assert_eq!(found.len(), 1, "{found:?}");
    assert_eq!(found[0].uri, widget, "{found:?}");
}

/// TP — a chain deeper than the ticket's, to show the publish follows the chain
/// rather than a fixed number of links: five documents, four metaclasses.
///
/// Oracle (tclsh 9.0.4 / 8.6.16): `[::D::Leaf new] go` -> went.
#[test]
fn a_five_level_chain_resolves_too() {
    let mut lsp = Lsp::tcl();
    let uris: Vec<String> = (0..5).map(|_| unique_uri("tcl")).collect();
    lsp.open_ready(
        &uris[0],
        "namespace eval ::D {}\noo::class create ::D::M1 { superclass oo::class }\n",
    );
    lsp.open_ready(
        &uris[1],
        "::D::M1 create ::D::M2 { superclass oo::class }\n",
    );
    lsp.open_ready(
        &uris[2],
        "::D::M2 create ::D::M3 { superclass oo::class }\n",
    );
    lsp.open_ready(
        &uris[3],
        "::D::M3 create ::D::Leaf { method go {} { return went } }\n",
    );
    lsp.open_ready(&uris[4], "set d [::D::Leaf new]\nputs [$d go]\n");

    let found = definition_settled(&mut lsp, &uris[4], 1, 10);
    assert_eq!(found.len(), 1, "{found:?}");
    assert_eq!(found[0].uri, uris[3], "{found:?}");
}

/// TN — nothing in the project proves `::U::Meta` manufactures classes, so the
/// call site has no answer and the server must report none rather than invent a
/// class. Bounded the same way, so "abstains" is a settled answer and not a
/// race that happened to be read early.
#[test]
fn an_unproved_metaclass_resolves_to_nothing() {
    let mut lsp = Lsp::tcl();
    let other = unique_uri("tcl");
    let widget = unique_uri("tcl");
    let call = unique_uri("tcl");
    lsp.open_ready(&other, "namespace eval ::U {}\nputs hello\n");
    lsp.open_ready(
        &widget,
        "::U::Meta create ::U::Widget { method go {} { return went } }\n",
    );
    lsp.open_ready(&call, "set w [::U::Widget new]\nputs [$w go]\n");

    // Give the index the same chance to converge it gets in the TP cases, then
    // require it to still have found nothing.
    let deadline = Instant::now() + scaled_timeout(Duration::from_secs(3));
    while Instant::now() < deadline {
        assert!(
            locations(&lsp.definition(&call, 1, 10)).is_empty(),
            "no file proves `::U::Meta` is a class factory; guessing is worse \
             than abstaining"
        );
        std::thread::sleep(Duration::from_millis(250));
    }
}

/// TP, the edit path — the middle link is typed in *after* the project has
/// already settled without it. The publish has to converge again to the deeper
/// answer rather than staying at the depth it had reached.
#[test]
fn typing_the_middle_link_makes_the_chain_resolve() {
    let mut lsp = Lsp::tcl();
    let meta_a = unique_uri("tcl");
    let meta_b = unique_uri("tcl");
    let widget = unique_uri("tcl");
    let call = unique_uri("tcl");
    lsp.open_ready(&meta_a, META_A);
    lsp.open_ready(&meta_b, "# the middle link is not written yet\n");
    lsp.open_ready(&widget, WIDGET);
    lsp.open_ready(&call, CALL);
    assert!(
        locations(&lsp.definition(&call, 1, 10)).is_empty(),
        "without the middle link there is nothing to resolve through"
    );

    lsp.settle_analysis(&meta_b, 2, META_B);
    let found = definition_settled(&mut lsp, &call, 1, 10);
    assert_eq!(
        found.len(),
        1,
        "the typed-in link completes the chain: {found:?}"
    );
    assert_eq!(found[0].uri, widget, "{found:?}");
}

/// TP (#1303) — a `TclOO` metaclass can deliberately route an unrecognised
/// first word through `unknown`, construct an object named by that word, and
/// return it. The proof is registry-gated and body-sensitive; once published,
/// a consumer document treats `set w [Widget .w]` as a typed object handle.
#[test]
fn unknown_dispatch_factory_result_resolves_its_instance_method() {
    let mut lsp = Lsp::tcl();
    let meta = unique_uri("tcl");
    let widget = unique_uri("tcl");
    let call = unique_uri("tcl");
    lsp.open_ready(
        &meta,
        concat!(
            "oo::class create Meta {\n",
            "  superclass oo::class\n",
            "  method unknown {w args} {\n",
            "    [self] create ::$w {*}$args\n",
            "    return $w\n",
            "  }\n",
            "  unexport new unknown\n",
            "}\n",
        ),
    );
    lsp.open_ready(
        &widget,
        "Meta create Widget { method go {} { return went } }\n",
    );
    lsp.open_ready(&call, "set w [Widget .w]\nputs [$w go]\n");

    let found = definition_settled(&mut lsp, &call, 1, 10);
    assert_eq!(found.len(), 1, "{found:?}");
    assert_eq!(found[0].uri, widget, "{found:?}");
}

/// TN/FP (#1303) — returning the requested word without constructing it does
/// not prove an object handle. This guards the body proof from degrading into
/// a name-shape heuristic.
#[test]
fn unknown_dispatch_that_only_echoes_the_word_resolves_nothing() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    lsp.open_ready(
        &uri,
        concat!(
            "oo::class create Meta {\n",
            "  superclass oo::class\n",
            "  method unknown {w args} { return $w }\n",
            "}\n",
            "Meta create Widget { method go {} { return went } }\n",
            "set w [Widget .w]\n",
            "puts [$w go]\n",
        ),
    );
    assert!(
        locations(&lsp.definition(&uri, 6, 10)).is_empty(),
        "an echoing unknown creates no object and must not fabricate a target"
    );
}

/// TP (#1306) — the top-level literal call binds the proc parameter used in
/// the metaclass's computed name. That resolved metaclass then manufactures a
/// navigable class in another document.
#[test]
fn literal_call_resolves_a_computed_metaclass_name() {
    let mut lsp = Lsp::tcl();
    let dialect = unique_uri("tcl");
    let widget = unique_uri("tcl");
    let call = unique_uri("tcl");
    lsp.open_ready(
        &dialect,
        concat!(
            "namespace eval ::T {}\n",
            "oo::class create ::T::Mother { superclass oo::class }\n",
            "proc ::T::NSNormalize {namespace qualname} {\n",
            "  if {![string match ::* $qualname]} {\n",
            "    set qualname ${namespace}::$qualname\n",
            "  }\n",
            "  regsub -all {::+} $qualname \"::\"\n",
            "}\n",
            "proc ::T::mkdialect {name} {\n",
            "  set NSPACE [NSNormalize [uplevel 1 {namespace current}] $name]\n",
            "  ::T::Mother create ${NSPACE}::class { superclass ::T::Mother }\n",
            "}\n",
            "::T::mkdialect ::T::D\n",
        ),
    );
    lsp.open_ready(
        &widget,
        "::T::D::class create ::T::Widget { method go {} { return went } }\n",
    );
    lsp.open_ready(&call, "set w [::T::Widget new]\nputs [$w go]\n");

    let found = definition_settled(&mut lsp, &call, 1, 10);
    assert_eq!(found.len(), 1, "{found:?}");
    assert_eq!(found[0].uri, widget, "{found:?}");
}

/// FP (#1306) — a literal call does not make a computed creation
/// unconditional.  The registry-selected false `if` path never creates the
/// metaclass, so navigation must not invent the downstream method.
#[test]
fn computed_metaclass_in_a_false_branch_resolves_nothing() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    lsp.open_ready(
        &uri,
        concat!(
            "namespace eval ::T {}\n",
            "oo::class create ::T::Mother { superclass oo::class }\n",
            "proc ::T::mk {name enabled} {\n",
            "  if {$enabled} {\n",
            "    ::T::Mother create ${name}::class { superclass ::T::Mother }\n",
            "  }\n",
            "}\n",
            "::T::mk ::T::D 0\n",
            "::T::D::class create ::T::Widget { method go {} { return went } }\n",
            "set w [::T::Widget new]\n",
            "puts [$w go]\n",
        ),
    );
    assert!(
        locations(&lsp.definition(&uri, 10, 10)).is_empty(),
        "a creation in an unselected arm must not publish a method target"
    );
}

/// TP (#1304) — the metaclass and the class it manufactures are discovered by
/// the startup scan and remain unopened. Opening only the consumer must still
/// resolve its method through the settled workspace factory index.
#[test]
fn unopened_factory_documents_feed_an_open_consumer() {
    let seq = WORKSPACE_SEQ.fetch_add(1, Ordering::Relaxed);
    let root = std::env::temp_dir().join(format!(
        "tcl-lsp-e2e-unopened-factory-{}-{seq}",
        std::process::id()
    ));
    std::fs::create_dir_all(&root).expect("create workspace root");
    let meta = root.join("meta.tcl");
    let widget = root.join("widget.tcl");
    let call = root.join("call.tcl");
    std::fs::write(&meta, "oo::class create Meta { superclass oo::class }\n")
        .expect("write metaclass");
    std::fs::write(
        &widget,
        "Meta create Widget { method go {} { return went } }\n",
    )
    .expect("write widget");
    let call_src = "set w [Widget new]\nputs [$w go]\n";
    std::fs::write(&call, call_src).expect("write consumer");

    let mut lsp = Lsp::at_workspace_root(&root);
    let call_uri = format!("file://{}", call.to_string_lossy());
    lsp.open_ready(&call_uri, call_src);
    let found = definition_settled(&mut lsp, &call_uri, 1, 10);
    assert_eq!(found.len(), 1, "{found:?}");
    assert!(found[0].uri.ends_with("widget.tcl"), "{found:?}");

    std::fs::remove_dir_all(root).ok();
}
