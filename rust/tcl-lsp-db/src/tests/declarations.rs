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

//! Declaration queries: original procedure facts, `file_decls`, item signatures, project procedure names and the procedure stubs a document publishes.

use super::*;

#[test]
fn original_procedure_facts_retain_each_current_declaration_site() {
    use salsa::Setter as _;
    let mut db = TclDatabase::default();
    let source = "proc repeated {} {return FIRST}\nproc repeated {} {return SECOND}";
    let file = SourceFile::new(&db, source.to_owned(), "tcl8.6".to_owned(), None);
    let config = cfg(&db);
    let first = file_original_procedure_facts(&db, file, config).unwrap();
    assert_eq!(first.declarations().len(), 2);
    assert_ne!(
        first.declarations()[0].declaration_site(),
        first.declarations()[1].declaration_site()
    );
    for declaration in first.declarations() {
        assert_eq!(declaration.name_input().source_image(), first.image());
        assert_eq!(
            declaration.name_input().lexer_config(),
            first.input().lexer_config()
        );
    }
    assert!(Arc::ptr_eq(
        &first,
        &file_original_procedure_facts(&db, file, config).unwrap()
    ));
    file.set_text(&mut db).to(format!("{source}\n# new image"));
    let changed = file_original_procedure_facts(&db, file, config).unwrap();
    assert_ne!(first.image(), changed.image());
    assert!(
        changed
            .declarations()
            .iter()
            .all(|declaration| { declaration.name_input().source_image() == changed.image() })
    );
}

#[test]
fn project_procedure_facts_keep_missing_file_inputs_unavailable() {
    let db = TclDatabase::default();
    let first = SourceFile::new(
        &db,
        "proc first {} {return FIRST}".to_owned(),
        "tcl8.6".to_owned(),
        None,
    );
    let other = SourceFile::new(
        &db,
        "proc other {} {return OTHER}".to_owned(),
        "tcl8.6".to_owned(),
        None,
    );
    let config = cfg(&db);
    let standalone = Project::new(&db, vec![first, other]);
    assert!(project_original_procedure_facts(&db, standalone).is_empty());
    let partial =
        Project::with_token_configurations(&db, vec![first, other], Some(vec![(first, config)]));
    let facts = project_original_procedure_facts(&db, partial);
    assert_eq!(facts.len(), 1);
    assert!(facts[0].0 == first);
    assert_eq!(facts[0].1.declarations().len(), 1);
    assert!(facts.iter().all(|(file, _)| *file != other));
}

#[test]
fn file_decls_match_file_analysis() {
    use std::collections::BTreeSet;
    let db = TclDatabase::default();
    let src = "proc p {} {}\noo::class create K {}\nnamespace eval z { proc q {} {} }\n";
    let file = SourceFile::new(&db, src.to_owned(), "tcl".to_owned(), None);
    let decls = file_decls(&db, file);
    let analysis = file_analysis(&db, file, cfg(&db));
    let want_procs: BTreeSet<String> = analysis.all_procs.keys().cloned().collect();
    let want_classes: BTreeSet<String> = analysis.all_classes.keys().cloned().collect();
    let want_aliases: BTreeSet<String> = analysis.command_aliases.keys().cloned().collect();
    assert_eq!(decls.procs, want_procs);
    assert_eq!(decls.classes, want_classes);
    assert_eq!(decls.aliases, want_aliases);
    assert!(decls.namespaces.contains("::z"));
}

#[test]
fn item_sigs_track_signatures() {
    use tcl_compiler::analyser::ItemKind;
    let db = TclDatabase::default();
    let file = SourceFile::new(
        &db,
        "proc greet {name} {}\n".to_owned(),
        "tcl".to_owned(),
        None,
    );
    let sigs = item_sigs(&db, file);
    let greet = sigs
        .iter()
        .find(|s| s.id.kind == ItemKind::Proc && s.id.key == "::greet")
        .expect("greet item");
    assert_eq!(greet.params.len(), 1);
    assert_eq!(greet.params[0].name, "name");
    assert_eq!(greet.namespace, "::");
}

#[test]
fn item_tree_recomputes_on_edit() {
    use salsa::Setter as _;
    let mut db = TclDatabase::default();
    let file = SourceFile::new(&db, "proc a {} {}\n".to_owned(), "tcl".to_owned(), None);
    assert!(file_decls(&db, file).procs.contains("::a"));
    file.set_text(&mut db).to("proc b {} {}\n".to_owned());
    let after = file_decls(&db, file);
    assert!(after.procs.contains("::b"));
    assert!(!after.procs.contains("::a"));
}

/// Published argument counts follow the document's parameter grammar, as
/// C Tcl 8.4–9.1 and Jim bind them: `args` is a rest formal only when last
/// in C Tcl and anywhere in Jim, and C Tcl ignores a default before a
/// required formal.
#[test]
fn procedure_stubs_follow_the_documents_parameter_grammar() {
    let source = "proc last {a args} {}\nproc middle {name args body} {}\n\
                      proc named {a ARGS} {}\nproc gap {a {b B} c} {}\nproc tail {a {b B}} {}\n";
    let db = TclDatabase::default();
    let stubs = |dialect: &str| {
        let file = SourceFile::new(&db, source.to_owned(), dialect.to_owned(), None);
        (*world_preserving_procedure_stubs(&db, file)).clone()
    };
    assert_eq!(
        stubs("tcl8.6"),
        [
            "stub gap {a b c}",
            "stub middle {name args body}",
            "stub named {a ARGS}",
            "stub tail {a ?b?}"
        ]
    );
    assert_eq!(
        stubs("jim"),
        [
            "stub gap {a ?b? c}",
            "stub named {a ARGS}",
            "stub tail {a ?b?}"
        ]
    );
}

#[test]
fn a_sibling_procedure_declared_to_the_caller_keeps_it_proved() {
    use salsa::Setter as _;

    let mut db = TclDatabase::default();
    let cfg = cfg(&db);
    let lib = SourceFile::new(
        &db,
        "proc putz {a} { return $a }\n".to_owned(),
        "tcl8.6".to_owned(),
        None,
    );
    let caller = SourceFile::new(
        &db,
        "putz hi\nproc user {} { putz x; putb y }\n".to_owned(),
        "tcl8.6".to_owned(),
        None,
    );
    let stubs = world_preserving_procedure_stubs(&db, lib);
    assert_eq!(*stubs, vec!["stub putz {a}".to_owned()]);
    let surface =
        tcl_compiler::analyser::utils::workspace_procedure_declarations(&stubs.join("\n"));
    caller
        .set_workspace_declared_commands(&mut db)
        .to(Some(Arc::new(surface)));
    let unknown = |analysis: &AnalysisResult| {
        analysis
            .diagnostics
            .iter()
            .filter(|d| d.code == DiagCode::W123)
            .map(|d| {
                String::from_utf8(
                    d.unresolved_command()
                        .unwrap()
                        .name_input()
                        .bytes()
                        .to_vec(),
                )
                .unwrap()
            })
            .collect::<Vec<_>>()
    };
    let expected = vec!["putb".to_owned()];
    assert_eq!(unknown(&file_analysis(&db, caller, cfg)), expected);
    assert_eq!(
        unknown(&file_analysis_incremental(&db, caller, cfg)),
        expected
    );
}

/// The project proc-name set lifted into salsa
/// over `file_decls` must extend the signature firewall *across files* — a
/// body edit in any file recomputes **zero** `project_proc_names` (its
/// `file_decls` backdates), while a decl change (a new proc) recomputes it
/// exactly once.  This is the property that stops a keystroke in one file from
/// waking the whole workspace.
#[test]
fn project_proc_names_firewall() {
    use salsa::Setter as _;
    let log: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(Vec::new()));
    let sink = {
        let l = Arc::clone(&log);
        move |ev: salsa::Event| {
            if let salsa::EventKind::WillExecute { database_key } = ev.kind {
                l.lock().unwrap().push(format!("{database_key:?}"));
            }
        }
    };
    let runs = |log: &Arc<Mutex<Vec<String>>>| {
        std::mem::take(&mut *log.lock().unwrap())
            .into_iter()
            .filter(|s| s.contains("project_proc_names"))
            .count()
    };
    let mut db = TclDatabase {
        storage: salsa::Storage::new(Some(Box::new(sink))),
    };
    let a = SourceFile::new(
        &db,
        "proc a {} { set x 1 }\n".to_owned(),
        "tcl8.6".to_owned(),
        None,
    );
    let b = SourceFile::new(
        &db,
        "proc b {} { set y 2 }\n".to_owned(),
        "tcl8.6".to_owned(),
        None,
    );
    let project = Project::new(&db, vec![a, b]);

    assert_eq!(
        project_proc_names(&db, project).len(),
        2,
        "cold: union of both files' procs"
    );
    let _ = runs(&log);

    // BODY edit to `a` — `file_decls(a)` is byte-identical, so it backdates and
    // the project set does NOT recompute (the firewall, extended across files).
    a.set_text(&mut db)
        .to("proc a {} { set x 999 }\n".to_owned());
    let _ = project_proc_names(&db, project);
    assert_eq!(
        runs(&log),
        0,
        "body edit in one file must not recompute project_proc_names"
    );

    // DECL change to `a` (add `proc c`) — `file_decls(a)` changes, so the
    // project set recomputes exactly once and gains the new proc.
    a.set_text(&mut db)
        .to("proc a {} { set x 999 }\nproc c {} {}\n".to_owned());
    let names = project_proc_names(&db, project);
    assert_eq!(
        runs(&log),
        1,
        "decl change must recompute project_proc_names exactly once"
    );
    assert_eq!(names.len(), 3, "the new proc joins the project set");
}
