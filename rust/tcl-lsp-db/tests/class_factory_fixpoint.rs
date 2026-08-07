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

//! The workspace class-factory index is a **fixpoint**, not a single round
//! (issue #1296).
//!
//! `project_class_factories` merges every file's `file_class_factories`, which
//! reads `item_tree`, which reads the per-file `SourceFile::workspace_class_factories`
//! input the host computes *from* that merge.  The query is therefore a
//! function of its own published answer, and one publish only ever advances the
//! metaclass chain by one link: `MetaA` is provable with no index at all, but
//! the file holding `MetaA create MetaB` proves `MetaB` only on a round whose
//! published index already names `MetaA`.  A host that publishes once leaves
//! every class `MetaB` manufactures unknown — which is what made a cross-file
//! three-level chain resolve to nothing from a call site.
//!
//! These tests drive the same loop `tcl-lsp-server`'s
//! `sync_workspace_class_factories` drives, so a regression in the salsa layer
//! (a query that stops seeing the published input, a merge that stops being
//! monotone) fails here without needing a live server.
//!
//! C-Tcl ground truth for the shapes below (tclsh 8.6.16 and 9.0.4 agree):
//!
//! ```text
//! namespace eval ::MC {}
//! oo::class create ::MC::MetaA { superclass oo::class }
//! ::MC::MetaA create ::MC::MetaB { superclass oo::class }
//! ::MC::MetaB create ::MC::Widget { method go {} { return went } }
//! puts [[::MC::Widget new] go]              -> went
//! info object isa class ::MC::Widget        -> 1
//! info class methods ::MC::Widget           -> go
//! ```

use salsa::Setter as _;
use std::sync::Arc;
use tcl_compiler::analyser::ClassFactoryIndex;
use tcl_lsp_db::{
    Project, SourceFile, TclDatabase, file_class_factories, file_decls, project_class_factories,
};

const DIALECT: &str = "tcl9.0";

/// Same bound the host applies, and for the same reason: convergence is
/// expected, not assumed.
const ROUND_CAP: usize = 16;

/// What one fixpoint run observed.
struct Fixpoint {
    /// Rounds run, including the final one that only confirmed the fixpoint.
    /// A workspace with no metaclass settles in 1.
    rounds: usize,
    /// The converged project-wide index.
    index: Arc<ClassFactoryIndex>,
    /// Whether the loop hit [`ROUND_CAP`] without settling.
    hit_cap: bool,
}

/// Drive the class-factory publish to a fixpoint exactly as the host does:
/// merge the project, compare-then-set the merged index on every file, and stop
/// as soon as a round moves nothing.
fn publish_to_fixpoint(db: &mut TclDatabase, project: Project, files: &[SourceFile]) -> Fixpoint {
    for round in 1..=ROUND_CAP {
        let index = project_class_factories(db, project);
        let want = (!index.is_empty()).then(|| Arc::clone(&index));
        let mut moved = false;
        for &file in files {
            if file.workspace_class_factories(db).as_deref() != want.as_deref() {
                file.set_workspace_class_factories(db).to(want.clone());
                moved = true;
            }
        }
        if !moved {
            return Fixpoint {
                rounds: round,
                index,
                hit_cap: false,
            };
        }
    }
    Fixpoint {
        rounds: ROUND_CAP,
        index: project_class_factories(db, project),
        hit_cap: true,
    }
}

/// Build a project out of `sources` and publish its factory index to a
/// fixpoint, returning the files (in the order given) and what the loop saw.
fn project_of(db: &mut TclDatabase, sources: &[&str]) -> (Vec<SourceFile>, Fixpoint) {
    let files: Vec<SourceFile> = sources
        .iter()
        .enumerate()
        .map(|(i, src)| {
            SourceFile::new(
                db,
                (*src).to_owned(),
                DIALECT.to_owned(),
                Some(format!("/w/f{i}.tcl")),
            )
        })
        .collect();
    let project = Project::new(&*db, files.clone());
    let settled = publish_to_fixpoint(db, project, &files);
    (files, settled)
}

fn factory_names(index: &ClassFactoryIndex) -> Vec<&str> {
    index.keys().map(String::as_str).collect()
}

fn classes_of(db: &TclDatabase, file: SourceFile) -> Vec<String> {
    file_decls(db, file).classes.iter().cloned().collect()
}

// The three files of the ticket's reproduction, one metaclass link each.
const META_A: &str =
    "namespace eval ::MC {}\noo::class create ::MC::MetaA { superclass oo::class }\n";
const META_B: &str = "::MC::MetaA create ::MC::MetaB { superclass oo::class }\n";
const WIDGET: &str = "::MC::MetaB create ::MC::Widget { method go {} { return went } }\n";

#[test]
fn a_cross_file_three_level_chain_converges_and_records_the_class() {
    // TP, the ticket itself. Three files, three links: the class the innermost
    // metaclass manufactures must come out of the third file's declarations.
    let mut db = TclDatabase::default();
    let (files, settled) = project_of(&mut db, &[META_A, META_B, WIDGET]);
    assert!(!settled.hit_cap, "must settle within the cap");
    assert_eq!(
        factory_names(&settled.index),
        ["::MC::MetaA", "::MC::MetaB"],
        "both metaclasses are proved, each by the file that writes it"
    );
    assert_eq!(
        classes_of(&db, files[2]),
        ["::MC::Widget"],
        "the manufactured class must be a declaration of the consuming file"
    );
    // Three links, so the loop needs one round per link plus a confirming one.
    assert_eq!(
        settled.rounds, 3,
        "one round per link, plus the confirmation"
    );
}

#[test]
fn a_single_publish_is_one_link_deep() {
    // The regression itself, stated as an assertion rather than a story: run
    // exactly one round of the same loop and the deepest link is missing, so a
    // host that publishes once cannot see `::MC::Widget`. This is what fails if
    // someone replaces the fixpoint with a single publish again.
    let mut db = TclDatabase::default();
    let files: Vec<SourceFile> = [META_A, META_B, WIDGET]
        .iter()
        .enumerate()
        .map(|(i, src)| {
            SourceFile::new(
                &db,
                (*src).to_owned(),
                DIALECT.to_owned(),
                Some(format!("/w/f{i}.tcl")),
            )
        })
        .collect();
    let project = Project::new(&db, files.clone());
    let index = project_class_factories(&db, project);
    let want = (!index.is_empty()).then(|| Arc::clone(&index));
    for &file in &files {
        file.set_workspace_class_factories(&mut db).to(want.clone());
    }
    assert_eq!(
        factory_names(&index),
        ["::MC::MetaA"],
        "one publish proves only the link that needed no index"
    );
    assert!(
        classes_of(&db, files[2]).is_empty(),
        "the class made by the second-link metaclass is still invisible: {:?}",
        classes_of(&db, files[2])
    );
}

#[test]
fn a_chain_deeper_than_three_still_converges() {
    // The bound is the chain depth, not a hardcoded three: five links across
    // five files, one per file, must all come out.
    let mut db = TclDatabase::default();
    let sources = [
        "namespace eval ::D {}\noo::class create ::D::M1 { superclass oo::class }\n",
        "::D::M1 create ::D::M2 { superclass oo::class }\n",
        "::D::M2 create ::D::M3 { superclass oo::class }\n",
        "::D::M3 create ::D::M4 { superclass oo::class }\n",
        "::D::M4 create ::D::Leaf { method go {} { return went } }\n",
    ];
    let (files, settled) = project_of(&mut db, &sources);
    assert!(!settled.hit_cap, "must settle within the cap");
    assert_eq!(
        factory_names(&settled.index),
        ["::D::M1", "::D::M2", "::D::M3", "::D::M4"],
        "every metaclass in the chain is a factory; the leaf class is not"
    );
    assert_eq!(classes_of(&db, files[4]), ["::D::Leaf"]);
}

#[test]
fn the_file_order_does_not_change_the_answer() {
    // The merge is unordered by design (a metaclass either exists in the
    // project or it does not), so presenting the chain leaf-first must produce
    // the same converged index and the same recorded class.
    let mut db = TclDatabase::default();
    let (files, settled) = project_of(&mut db, &[WIDGET, META_B, META_A]);
    assert!(!settled.hit_cap);
    assert_eq!(
        factory_names(&settled.index),
        ["::MC::MetaA", "::MC::MetaB"]
    );
    assert_eq!(
        classes_of(&db, files[0]),
        ["::MC::Widget"],
        "the consumer is the first file this time"
    );
}

#[test]
fn a_metaclass_reached_through_namespace_eval_converges() {
    // The chain's links are written inside `namespace eval` bodies and named
    // relatively, so the resolution the walk does is the real
    // current-namespace-then-global one rather than a global-name match.
    //
    // Oracle (tclsh 9.0.4 / 8.6.16): `[::N::Widget new] go` -> went, and
    // `info class superclasses ::N::MetaB` -> ::oo::class.
    let mut db = TclDatabase::default();
    let sources = [
        "namespace eval ::N {\n    oo::class create MetaA { superclass oo::class }\n}\n",
        "namespace eval ::N {\n    MetaA create MetaB { superclass oo::class }\n}\n",
        "namespace eval ::N {\n    MetaB create Widget { method go {} { return went } }\n}\n",
    ];
    let (files, settled) = project_of(&mut db, &sources);
    assert!(!settled.hit_cap);
    assert_eq!(factory_names(&settled.index), ["::N::MetaA", "::N::MetaB"]);
    assert_eq!(classes_of(&db, files[2]), ["::N::Widget"]);
}

#[test]
fn a_cycle_terminates_and_proves_nothing() {
    // TN + termination. `A` made by `B` made by `A` is not a program real Tcl
    // can run (neither command exists when the other is defined), and neither
    // link is ever proved, so the loop must settle immediately on an empty
    // index rather than oscillating up to the cap.
    let mut db = TclDatabase::default();
    let sources = [
        "namespace eval ::C {}\n::C::B create ::C::A { superclass oo::class }\n",
        "::C::A create ::C::B { superclass oo::class }\n",
    ];
    let (files, settled) = project_of(&mut db, &sources);
    assert!(!settled.hit_cap, "a cycle must not run to the cap");
    assert_eq!(settled.rounds, 1, "nothing is proved, so nothing moves");
    assert!(
        settled.index.is_empty(),
        "no guess: {:?}",
        factory_names(&settled.index)
    );
    for &file in &files {
        assert!(
            classes_of(&db, file).is_empty(),
            "a cyclic declaration manufactures nothing: {:?}",
            classes_of(&db, file)
        );
    }
}

#[test]
fn an_unproved_metaclass_still_abstains() {
    // TN. `Meta create X …` where nothing in the project proves `::U::Meta` is
    // a class factory is, on shape alone, indistinguishable from `interp
    // create` — so no class may be recorded and no index entry invented.
    let mut db = TclDatabase::default();
    let sources = [
        "namespace eval ::U {}\nputs hello\n",
        "::U::Meta create ::U::Widget { method go {} { return went } }\n",
    ];
    let (files, settled) = project_of(&mut db, &sources);
    assert_eq!(settled.rounds, 1);
    assert!(
        settled.index.is_empty(),
        "nothing proved a factory: {:?}",
        factory_names(&settled.index)
    );
    assert!(
        classes_of(&db, files[1]).is_empty(),
        "abstain rather than guess a class: {:?}",
        classes_of(&db, files[1])
    );
}

#[test]
fn a_same_named_proc_is_not_mistaken_for_the_metaclass() {
    // FP guard. A `proc ::MC::MetaB {args} {…}` in the project declares a
    // command with the metaclass's name but is not a class factory, so it must
    // not put `::MC::MetaB` in the index and must not turn
    // `::MC::MetaB create …` into a class. The chain is deliberately cut one
    // link short (no `MetaA create MetaB`) so the *only* candidate for the name
    // is the proc.
    let mut db = TclDatabase::default();
    let sources = [META_A, "proc ::MC::MetaB {args} { return $args }\n", WIDGET];
    let (files, settled) = project_of(&mut db, &sources);
    assert_eq!(
        factory_names(&settled.index),
        ["::MC::MetaA"],
        "a proc of that name proves nothing about class manufacture"
    );
    assert!(
        classes_of(&db, files[2]).is_empty(),
        "`::MC::MetaB create …` must stay unclassified: {:?}",
        classes_of(&db, files[2])
    );
}

#[test]
fn a_local_class_of_the_same_name_shadows_the_workspace_one() {
    // FP guard, the other direction: the consuming file writes its own
    // `::MC::MetaB` that is an ordinary class (no metaclass superclass), so the
    // workspace entry must not reach past it. Real Tcl resolves the command to
    // whatever the interpreter last defined; the walk's rule is that the file's
    // own declaration wins, exactly as it does for a proc.
    let mut db = TclDatabase::default();
    let sources = [
        META_A,
        META_B,
        "oo::class create ::MC::MetaB { method plain {} { return p } }\n\
         ::MC::MetaB create ::MC::Widget { method go {} { return went } }\n",
    ];
    let (files, _) = project_of(&mut db, &sources);
    let classes = classes_of(&db, files[2]);
    assert!(
        classes.contains(&"::MC::MetaB".to_owned()),
        "the file's own class is a declaration: {classes:?}"
    );
    assert!(
        !classes.contains(&"::MC::Widget".to_owned()),
        "the local, non-factory `::MC::MetaB` manufactures nothing: {classes:?}"
    );
}

#[test]
fn a_workspace_with_no_metaclass_costs_no_extra_round() {
    // The overwhelmingly common case, and the one the whole mechanism promises
    // to leave alone: an empty index is stored as `None`, the input never
    // leaves its default, so the loop settles on its first round having written
    // nothing at all.
    let mut db = TclDatabase::default();
    let sources = [
        "proc greet {name} { puts \"hi $name\" }\n",
        "oo::class create Dog {\n    method speak {} { return woof }\n}\ngreet world\n",
    ];
    let (files, settled) = project_of(&mut db, &sources);
    assert_eq!(settled.rounds, 1, "no metaclass ⇒ exactly one round");
    assert!(settled.index.is_empty());
    for &file in &files {
        assert!(
            file.workspace_class_factories(&db).is_none(),
            "the input must never leave its `None` default"
        );
        assert!(
            file_class_factories(&db, file).is_empty(),
            "and no file publishes a factory"
        );
    }
    assert_eq!(classes_of(&db, files[1]), ["::Dog"]);
}

#[test]
fn adding_the_middle_link_later_converges_from_the_settled_state() {
    // The edit path, not just the cold-start path: a project that has already
    // settled without the middle link must converge again — to the deeper
    // answer — when that link is typed in, starting from the previously
    // published index rather than from scratch.
    let mut db = TclDatabase::default();
    let files: Vec<SourceFile> = ["", META_A, WIDGET]
        .iter()
        .enumerate()
        .map(|(i, src)| {
            SourceFile::new(
                &db,
                (*src).to_owned(),
                DIALECT.to_owned(),
                Some(format!("/w/f{i}.tcl")),
            )
        })
        .collect();
    let project = Project::new(&db, files.clone());
    let first = publish_to_fixpoint(&mut db, project, &files);
    assert_eq!(factory_names(&first.index), ["::MC::MetaA"]);
    assert!(
        classes_of(&db, files[2]).is_empty(),
        "without the middle link there is nothing to prove `::MC::MetaB`"
    );

    files[0].set_text(&mut db).to(META_B.to_owned());
    let second = publish_to_fixpoint(&mut db, project, &files);
    assert!(!second.hit_cap);
    assert_eq!(
        factory_names(&second.index),
        ["::MC::MetaA", "::MC::MetaB"],
        "the edit's link is picked up and published"
    );
    assert_eq!(classes_of(&db, files[2]), ["::MC::Widget"]);
}

#[test]
fn removing_a_link_retracts_the_classes_it_manufactured() {
    // The monotone story is per-round, not across edits: deleting the middle
    // link must take `::MC::MetaB` — and the class it made — back out, rather
    // than leaving a published entry nothing proves any more.
    let mut db = TclDatabase::default();
    let (files, settled) = project_of(&mut db, &[META_A, META_B, WIDGET]);
    assert_eq!(
        factory_names(&settled.index),
        ["::MC::MetaA", "::MC::MetaB"]
    );

    files[1].set_text(&mut db).to(String::new());
    let project = Project::new(&db, files.clone());
    let after = publish_to_fixpoint(&mut db, project, &files);
    assert!(!after.hit_cap);
    assert_eq!(factory_names(&after.index), ["::MC::MetaA"]);
    assert!(
        classes_of(&db, files[2]).is_empty(),
        "the class its metaclass made goes with it: {:?}",
        classes_of(&db, files[2])
    );
}

#[test]
fn an_overriding_manufacturer_survives_the_extra_link() {
    // The Tk shape (issue #1276) with one more link in front of it: the
    // metaclass that overrides `create {name superclasses body}` is *itself*
    // manufactured cross-file, so the override's argument layout has to survive
    // being read on a later round rather than the first.
    //
    // Oracle (tclsh 9.0.4 / 8.6.16):
    //   info class superclasses ::IconList -> ::Root ::Focusable
    //   info class methods ::IconList      -> (its own)
    let mut db = TclDatabase::default();
    let sources = [
        "oo::class create ::Root { method trace {} { return traced } }\n\
         oo::class create ::Base { superclass oo::class }\n",
        "::Base create ::Mega {\n    superclass oo::class\n\
         \x20   self method create {name superclasses body} {\n\
         \x20       next $name [list superclass ::Root {*}$superclasses]\\;$body\n\
         \x20   }\n}\n",
        "::Mega create ::IconList ::Focusable {\n    method GetSpecs {} { return specs }\n}\n",
    ];
    let (files, settled) = project_of(&mut db, &sources);
    assert!(!settled.hit_cap);
    assert_eq!(factory_names(&settled.index), ["::Base", "::Mega"]);
    assert_eq!(classes_of(&db, files[2]), ["::IconList"]);
    let spec = &settled.index["::Mega"].overrides["create"];
    assert_eq!(
        (spec.name_arg, spec.body_arg),
        (1, 3),
        "the override's own layout is read, not guessed: {spec:?}"
    );
}
