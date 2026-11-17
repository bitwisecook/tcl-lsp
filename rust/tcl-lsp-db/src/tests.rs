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
use std::sync::Mutex;

mod arity_diagnostics;
mod callback_arity;
mod check_solve;
mod command_signatures;
mod compilation_units;
mod compiler_diagnostics;
mod declarations;
mod display_queries;
mod document_analysis;
mod function_lattices;
mod logical_source_arity;
mod optimisation_solve;

fn original_test_body_id(unit: &CompilationUnit, offset: usize) -> tcl_compiler::ir::ProcedureId {
    let offset = u32::try_from(offset).expect("finite original fixture offset");
    let mut bodies = unit
        .ir_module
        .procedures
        .iter()
        .chain(unit.ir_module.body_units.iter())
        .filter(|(id, _)| unit.original_procedure_for_body(**id).is_some())
        .filter(|(_, procedure)| {
            procedure.declaration_site.as_ref().is_some_and(|site| {
                site.offset == offset
                    && site.source.source_image().bytes() == unit.source.as_bytes()
            })
        });
    let (&id, _) = bodies.next().expect("actual original fixture declaration");
    assert!(
        bodies.next().is_none(),
        "one body at the original fixture site"
    );
    assert!(unit.ir_module.procedure_ids.owns(id));
    id
}

/// A key that misses, installs, retires and misses again is reported each
/// time it misses: the log keeps a miss once while it stands, and forgets
/// it when the key installs.
#[test]
fn a_key_that_installs_is_reported_again_if_it_misses_later() {
    let generations = OverlayGenerations::default();
    let key: OverlayKey = ("tcl9.0".to_owned(), 0xAB);
    let miss = OverlayMiss {
        environment: key.0.clone(),
        overlay: key.1,
    };
    let take = |generations: &OverlayGenerations| {
        std::mem::take(&mut generations.misses.lock().expect("miss log").unreported)
    };
    generations.record(&miss);
    generations.record(&miss);
    assert_eq!(
        take(&generations),
        vec![miss.clone()],
        "one record while it stands"
    );

    let registry = Arc::new(CommandRegistry::build_default());
    generations.hold(key.clone(), Arc::clone(&registry));
    // Retired: later keys push it out of the held generations.
    let later = u64::try_from(HELD_OVERLAYS).expect("a small count");
    for overlay in 0..=later {
        generations.hold(
            ("tcl9.0".to_owned(), 0x1000 + overlay),
            Arc::clone(&registry),
        );
    }
    assert!(generations.held(&key).is_none(), "the key was retired");
    generations.record(&miss);
    assert_eq!(
        take(&generations),
        vec![miss],
        "a miss after the key installed is a new one"
    );
}

fn cfg(db: &TclDatabase) -> AnalyserConfig {
    AnalyserConfig::new(
        db,
        Vec::new(),
        NonAsciiMode::Default,
        Vec::new(),
        None,
        None,
        0,
        Vec::new(),
        Vec::new(),
    )
}

const SRC: &str = "proc greet {name} {\n    puts \"hi $name\"\n}\n# c\nset x 1\n";

/// End-to-end through the query graph: `lib.tcl` has no
/// `package provide`, its two in-file callers agree on `"prod"`, and
/// `main.tcl` — which the single-file compilation unit can never see —
/// calls `helper dev`.  With the project's evidence set on the file, the
/// I230 "condition is always true/false" fold must not happen.
mod cross_file_call_sites {
    use super::*;

    const LIB: &str = "proc helper {mode} {\n\
                           if {$mode eq \"prod\"} { set r 1 } else { set r 2 }\n\
                           }\n\
                           helper prod\n\
                           helper prod\n";

    /// Build a two-file project and return `lib`'s compilation unit with
    /// the project evidence applied, exactly as the server's
    /// `sync_cross_file_evidence` does.
    fn lib_unit_with_project(main_src: &str) -> Arc<CompilationUnit> {
        use salsa::Setter as _;
        let mut db = TclDatabase::default();
        let lib = SourceFile::new(&db, LIB.to_owned(), "tcl8.6".to_owned(), None);
        let main = SourceFile::new(&db, main_src.to_owned(), "tcl8.6".to_owned(), None);
        let project = Project::new(&db, vec![lib, main]);
        let evidence = file_external_call_sites(&db, lib, project);
        lib.set_external_call_sites(&mut db).to(Some(evidence));
        document_compilation_unit(&db, lib)
    }

    /// Build a project from `(path, source)` pairs and return the
    /// compilation unit of the first, with project evidence applied.
    fn unit_with_paths(files: &[(&str, &str)]) -> Arc<CompilationUnit> {
        use salsa::Setter as _;
        let mut db = TclDatabase::default();
        let handles: Vec<SourceFile> = files
            .iter()
            .map(|(path, src)| {
                SourceFile::new(
                    &db,
                    (*src).to_owned(),
                    "tcl8.6".to_owned(),
                    Some((*path).to_owned()),
                )
            })
            .collect();
        let project = Project::new(&db, handles.clone());
        let target = handles[0];
        let evidence = file_external_call_sites(&db, target, project);
        target.set_external_call_sites(&mut db).to(Some(evidence));
        document_compilation_unit(&db, target)
    }

    /// A sourced library's *unreadable* dispatch reaches the procedures of
    /// the file that sources it: `lib.tcl`'s script runs in `main.tcl`'s
    /// interpreter, so `$cmd` can name `::helper`.
    ///
    /// The inbound direction. Bounding an unenumerable dispatch by what
    /// the scanning file itself loads gets this wrong — `lib.tcl` declares
    /// no linkage at all — which is why the bound is the `source`-
    /// connected component (`file_dispatch_reach`), not the file's own
    /// linkage traits.
    #[test]
    fn an_unreadable_dispatch_in_a_sourced_library_clears_the_sourcing_files_fold() {
        let cu = unit_with_paths(&[
            (
                "/w/main.tcl",
                "proc helper {mode} {\n\
                     if {$mode eq \"prod\"} { set r 1 } else { set r 2 }\n\
                     }\n\
                     source lib.tcl\n\
                     helper prod\n\
                     helper prod\n",
            ),
            ("/w/lib.tcl", "set cmd [gets stdin]\n$cmd dev\n"),
        ]);
        assert!(
            !folds_mode(&cu),
            "lib.tcl runs in main.tcl's interpreter; $cmd may name ::helper",
        );
    }

    /// TN control: the same unreadable dispatch in a file nothing sources
    /// and which sources nothing shares no interpreter, so it must leave
    /// an unrelated file's sound seed alone. Without the component bound
    /// this is the case that breaks — one `eval $script` anywhere in a
    /// workspace would withdraw every seed in every file.
    #[test]
    fn an_unreadable_dispatch_in_an_unlinked_file_leaves_the_fold_alone() {
        let cu = unit_with_paths(&[
            (
                "/w/main.tcl",
                "proc helper {mode} {\n\
                     if {$mode eq \"prod\"} { set r 1 } else { set r 2 }\n\
                     }\n\
                     helper prod\n\
                     helper prod\n",
            ),
            ("/w/unrelated.tcl", "set cmd [gets stdin]\n$cmd dev\n"),
        ]);
        assert!(
            folds_mode(&cu),
            "an unlinked file shares no interpreter with main.tcl",
        );
    }

    fn folds_mode(cu: &CompilationUnit) -> bool {
        let body_id = original_test_body_id(cu, 0);
        !cu.procedures
            .get(&body_id)
            .expect("helper analysed")
            .sccp
            .constant_branches
            .is_empty()
    }

    #[test]
    fn caller_in_another_file_with_a_differing_literal_retracts_the_fold() {
        let cu = lib_unit_with_project("source lib.tcl\nhelper dev\n");
        assert!(
            !folds_mode(&cu),
            "main.tcl calls helper with \"dev\"; the seed is unsound"
        );
    }

    #[test]
    fn caller_in_another_file_agreeing_still_folds() {
        let cu = lib_unit_with_project("source lib.tcl\nhelper prod\n");
        assert!(
            folds_mode(&cu),
            "every caller in the project passes \"prod\""
        );
    }

    /// The per-file slice is keyed on the file's own declarations *and*
    /// carries only calls from **other** files.
    ///
    /// `main` both declares and calls `::other`, so that call is in-unit
    /// evidence for `main` and never reaches its external slice — leaving
    /// it empty, which is still the meaningful "the project was
    /// enumerated, nobody else calls in" claim. `lib`'s slice does carry
    /// `::helper`, because `main` calls it without declaring it.
    #[test]
    fn slice_carries_only_other_files_calls_to_this_files_procs() {
        let db = TclDatabase::default();
        let lib = SourceFile::new(
            &db,
            LIB.to_owned(),
            "tcl8.6".to_owned(),
            Some("/w/lib.tcl".to_owned()),
        );
        let main = SourceFile::new(
            &db,
            "source lib.tcl\nproc other {x} { return $x }\nother 1\nhelper dev\n".to_owned(),
            "tcl8.6".to_owned(),
            Some("/w/main.tcl".to_owned()),
        );
        let project = Project::new(&db, vec![lib, main]);
        let lib_slice = file_external_call_sites(&db, lib, project);
        assert_eq!(lib_slice.callees().collect::<Vec<_>>(), vec!["::helper"]);
        let main_slice = file_external_call_sites(&db, main, project);
        assert!(
            main_slice.is_empty(),
            "main's call to its own `other` is in-unit evidence: {main_slice:?}"
        );
    }

    /// A workspace pools every file into one project, so two *unrelated*
    /// files reusing a common helper name (`helper`, `init`, `run` — endemic
    /// in Tcl) must not pool their call sites.  Here the second file declares
    /// its own zero-arity `::helper`; without the self-declared-callee
    /// exclusion its call contributes `arg_counts {0}`, `binds_position(0)`
    /// fails, and the first file loses a fold it is entitled to — and would
    /// regain only when the unrelated file changed.
    ///
    /// This exact collision surfaces in the VS Code suite, where ~200
    /// fixtures share one workspace folder.
    #[test]
    fn an_unrelated_file_reusing_a_proc_name_does_not_poison_the_seed() {
        use salsa::Setter as _;
        let mut db = TclDatabase::default();
        let ctl = SourceFile::new(
            &db,
            "proc helper {mode} {\n if {$mode eq \"prod\"} { set r 1 } else { set r 2 }\n}\n\
             proc c1 {} { helper prod }\nproc c2 {} { helper prod }\n"
                .to_owned(),
            "tcl8.6".to_owned(),
            None,
        );
        let unrelated = SourceFile::new(
            &db,
            "proc helper {} {}\nproc caller {} { helper }\n".to_owned(),
            "tcl8.6".to_owned(),
            None,
        );
        let project = Project::new(&db, vec![ctl, unrelated]);
        let evidence = file_external_call_sites(&db, ctl, project);
        assert!(
            evidence.get("::helper").is_none(),
            "the unrelated file's own `helper` is in-unit evidence there, not \
             evidence about this file's proc: {evidence:?}"
        );
        ctl.set_external_call_sites(&mut db).to(Some(evidence));
        let cu = document_compilation_unit(&db, ctl);
        let body_id = original_test_body_id(&cu, 0);
        assert!(
            !cu.procedures
                .get(&body_id)
                .expect("helper analysed")
                .sccp
                .constant_branches
                .is_empty(),
            "every caller that can actually reach this proc passes \"prod\""
        );
    }

    /// A body edit that moves no call-site literal must leave
    /// `project_call_site_evidence` equal, so it backdates and no
    /// compilation unit downstream of it is invalidated.
    #[test]
    fn evidence_backdates_across_an_unrelated_body_edit() {
        use salsa::Setter as _;
        let mut db = TclDatabase::default();
        let lib = SourceFile::new(&db, LIB.to_owned(), "tcl8.6".to_owned(), None);
        let main = SourceFile::new(
            &db,
            "source lib.tcl\nhelper dev\n".to_owned(),
            "tcl8.6".to_owned(),
            None,
        );
        let project = Project::new(&db, vec![lib, main]);
        let before = file_external_call_sites(&db, lib, project);
        main.set_text(&mut db)
            .to("source lib.tcl\nset unrelated 1\nhelper dev\n".to_owned());
        let after = file_external_call_sites(&db, lib, project);
        assert_eq!(*before, *after, "an unrelated edit must not move evidence");
    }

    /// The component bound itself: both ends of a `source` edge see the
    /// same merged set — and see the *same* `Arc`, so a component is
    /// stored once however many files belong to it.  An unlinked file
    /// shares no interpreter, so it reaches only its own declarations.
    #[test]
    fn dispatch_reach_is_the_shared_source_component_merge() {
        let db = TclDatabase::default();
        let a = SourceFile::new(
            &db,
            "source b.tcl\nproc a {} {}\n".to_owned(),
            "tcl8.6".to_owned(),
            Some("/w/a.tcl".to_owned()),
        );
        let b = SourceFile::new(
            &db,
            "proc b {} {}\n".to_owned(),
            "tcl8.6".to_owned(),
            Some("/w/b.tcl".to_owned()),
        );
        let unlinked = SourceFile::new(
            &db,
            "proc c {} {}\n".to_owned(),
            "tcl8.6".to_owned(),
            Some("/w/c.tcl".to_owned()),
        );
        let project = Project::new(&db, vec![a, b, unlinked]);
        let reach_a =
            file_dispatch_reach(&db, a, project).expect("complete original project source paths");
        let reach_b =
            file_dispatch_reach(&db, b, project).expect("complete original project source paths");
        let reach_c = file_dispatch_reach(&db, unlinked, project)
            .expect("complete original project source paths");
        assert_eq!(
            reach_a.iter().map(String::as_str).collect::<Vec<_>>(),
            vec!["::a", "::b"],
            "the sourcing file reaches its own and the sourced file's procs"
        );
        assert!(
            Arc::ptr_eq(&reach_a, &reach_b),
            "component members share one merged set: {reach_b:?}"
        );
        assert_eq!(
            reach_c.iter().map(String::as_str).collect::<Vec<_>>(),
            vec!["::c"],
            "an unlinked file shares no interpreter"
        );
    }

    /// The `source`-graph decomposition is *project* work, not
    /// per-file work.  Demanding every file's reach must execute
    /// [`project_dispatch_components`] exactly **once** — the property that
    /// keeps the union-find-and-merge at `O(N)` overall rather than paying
    /// it once per file — and a decl change must recompute it once for the
    /// project rather than once per file.  A body edit backdates its
    /// inputs and recomputes nothing.
    #[test]
    fn dispatch_components_are_computed_once_per_project_revision() {
        use salsa::Setter as _;
        const FILES: usize = 6;
        let log: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(Vec::new()));
        let mut db = {
            let sink = Arc::clone(&log);
            TclDatabase::with_event_logger(move |key| sink.lock().unwrap().push(key))
        };
        // A single `source` chain, so every file lands in one component —
        // the shape a per-file union-find-and-merge would re-pay for
        // every file.
        let body = |i: usize| {
            if i + 1 < FILES {
                format!("source f{}.tcl\nproc p{i} {{}} {{ set r 1 }}\n", i + 1)
            } else {
                format!("proc p{i} {{}} {{ set r 1 }}\n")
            }
        };
        let files: Vec<SourceFile> = (0..FILES)
            .map(|i| {
                SourceFile::new(
                    &db,
                    body(i),
                    "tcl8.6".to_owned(),
                    Some(format!("/w/f{i}.tcl")),
                )
            })
            .collect();
        let project = Project::new(&db, files.clone());
        let drain = || std::mem::take(&mut *log.lock().unwrap());
        let decompositions = |events: &[String]| {
            events
                .iter()
                .filter(|key| key.contains("project_dispatch_components"))
                .count()
        };

        for &file in &files {
            let _ = file_dispatch_reach(&db, file, project);
        }
        assert_eq!(
            decompositions(&drain()),
            1,
            "cold: one decomposition serves every file's reach"
        );

        files[3]
            .set_text(&mut db)
            .to("source f4.tcl\nproc p3 {} { set r 999 }\n".to_owned());
        for &file in &files {
            let _ = file_dispatch_reach(&db, file, project);
        }
        assert_eq!(
            decompositions(&drain()),
            0,
            "a body edit leaves file_decls / file_link_targets equal"
        );

        files[3]
            .set_text(&mut db)
            .to("source f4.tcl\nproc p3 {} { set r 999 }\nproc extra {} {}\n".to_owned());
        for &file in &files {
            let _ = file_dispatch_reach(&db, file, project);
        }
        assert_eq!(
            decompositions(&drain()),
            1,
            "a decl change recomputes the decomposition once, not once per file"
        );
        assert!(
            file_dispatch_reach(&db, files[0], project)
                .unwrap()
                .contains("::extra"),
            "the new proc joins every component member's reach"
        );
    }
}
