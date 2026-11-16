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

//! Arity advice for logical source calls and callbacks: receipts, occupants, namespace slots and unresolved mutations.

use super::*;

#[test]
fn logical_callback_source_receipts_replace_reporting_count_and_tail_joins() {
    use salsa::Setter as _;
    let mut db = TclDatabase::default();
    let provider = SourceFile::new(&db, "proc helper {} {}".to_owned(), "tcl".to_owned(), None);
    let caller_source = "lsort -command {helper baked} {a b}";
    let caller = SourceFile::new(&db, caller_source.to_owned(), "tcl".to_owned(), None);
    let project = Project::new(&db, vec![provider, caller]);
    let analysis = Analyser::new().analyse(caller_source, "tcl");
    let diagnose = |db: &TclDatabase, analysis: &AnalysisResult| {
        project_callback_diagnostics_for_analysis(db, project, caller_source, analysis, |_| false)
            .into_iter()
            .filter(|diagnostic| diagnostic.code == DiagCode::E003)
            .map(|diagnostic| (diagnostic.span, diagnostic.message))
            .collect::<Vec<_>>()
    };
    let original = diagnose(&db, &analysis);
    assert_eq!(original.len(), 1);
    assert!(original[0].1.contains("got 3"));
    let mut labels = analysis.clone();
    for invocation in &mut labels.command_invocations {
        invocation.name = "foreign_report".to_owned();
        invocation.callback_arity = None;
        invocation.callback_baked_args = 100;
        invocation.resolution_candidates.clear();
    }
    assert_eq!(diagnose(&db, &labels), original);
    let mut missing = analysis.clone();
    missing.resolved_input = None;
    assert_eq!(diagnose(&db, &missing).len(), 0);
    provider
        .set_text(&mut db)
        .to("proc helper {first second third} {}".to_owned());
    assert_eq!(diagnose(&db, &analysis).len(), 0);
}

#[test]
fn logical_callback_source_unknown_mutations_keep_unresolved_alternatives() {
    let db = TclDatabase::default();
    let provider = SourceFile::new(&db, "proc helper {} {}".to_owned(), "tcl".to_owned(), None);
    let source = "unmodelled_registry_mutation; lsort -command {::helper baked} {a b}";
    let caller = SourceFile::new(&db, source.to_owned(), "tcl".to_owned(), None);
    let project = Project::new(&db, vec![provider, caller]);
    let analysis = Analyser::new().analyse(source, "tcl");
    let callbacks =
        tcl_compiler::registry_invocation::source_structure::source_logical_callback_candidates(
            source, &analysis,
        )
        .unwrap();
    assert_eq!(callbacks.len(), 1);
    assert!(callbacks[0].registration().obligations().contains(
        &tcl_compiler::command_binding::SourceCommandTransitionObligation::UnknownEarlierMutation
    ));
    let diagnostics =
        project_callback_diagnostics_for_analysis(&db, project, source, &analysis, |_| false);
    assert!(
        !diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == DiagCode::E003)
    );
}

#[test]
fn logical_callback_project_headers_keep_installer_operand_effects_unknown() {
    let db = TclDatabase::default();
    let provider = SourceFile::new(&db, "proc helper {} {}".to_owned(), "tcl".to_owned(), None);
    let source = "lsort -command helper [set items {a b}]";
    let caller = SourceFile::new(&db, source.to_owned(), "tcl".to_owned(), None);
    let project = Project::new(&db, vec![provider, caller]);
    let analysis = Analyser::new().analyse(source, "tcl");
    let diagnostics =
        project_callback_diagnostics_for_analysis(&db, project, source, &analysis, |_| false);
    assert!(
        diagnostics
            .iter()
            .all(|diagnostic| diagnostic.code != DiagCode::E003)
    );
}

#[test]
fn logical_callback_project_headers_preserve_selected_source_occupants() {
    let db = TclDatabase::default();
    let provider = SourceFile::new(
        &db,
        "proc puts {} {}; proc helper {} {}".to_owned(),
        "tcl".to_owned(),
        None,
    );
    for source in [
        "lsort -command puts {a b}",
        "proc helper {a b} {}; lsort -command helper {a b}",
        "interp alias {} helper {} puts; lsort -command helper {a b}",
        "proc helper {a b} {}; rename helper {}; lsort -command helper {a b}",
    ] {
        let caller = SourceFile::new(&db, source.to_owned(), "tcl".to_owned(), None);
        let project = Project::new(&db, vec![provider, caller]);
        let analysis = Analyser::new().analyse(source, "tcl");
        let callbacks = tcl_compiler::registry_invocation::source_structure::source_logical_callback_candidates(source, &analysis).unwrap();
        assert_eq!(callbacks.len(), 1, "{source}");
        assert!(
            callbacks[0].external_header_coordinates().is_none(),
            "{source}"
        );
        let mut local = analysis.clone();
        local.diagnostics.clear();
        let diagnostics =
            project_callback_diagnostics_for_analysis(&db, project, source, &local, |_| false);
        assert!(diagnostics.is_empty(), "{source}: {diagnostics:?}");
    }
}

#[test]
fn logical_callback_project_headers_use_callback_scope_and_skip_deleted_alternatives() {
    let db = TclDatabase::default();
    let provider = SourceFile::new(
        &db,
        "proc helper {} {}; namespace eval A {proc helper {a b} {}}".to_owned(),
        "tcl".to_owned(),
        None,
    );
    for (source, expected) in [
        (
            "namespace eval A {proc helper {a b} {}; after 1 {helper baked}}",
            1,
        ),
        (
            "proc helper {a b} {}; namespace eval A {lsort -command helper {a b}}",
            0,
        ),
        (
            "namespace eval A {proc helper {a b} {}; rename helper {}; lsort -command helper {a b}}",
            1,
        ),
    ] {
        let caller = SourceFile::new(&db, source.to_owned(), "tcl".to_owned(), None);
        let project = Project::new(&db, vec![provider, caller]);
        let analysis = Analyser::new().analyse(source, "tcl");
        let mut local = analysis.clone();
        local.diagnostics.clear();
        let diagnostics =
            project_callback_diagnostics_for_analysis(&db, project, source, &local, |_| false);
        assert_eq!(
            diagnostics
                .iter()
                .filter(|diagnostic| diagnostic.code == DiagCode::E003)
                .count(),
            expected,
            "{source}: {diagnostics:?}"
        );
    }
}

#[test]
fn logical_direct_project_headers_do_not_depend_on_unknown_command_reporting() {
    let db = TclDatabase::default();
    let provider = SourceFile::new(
        &db,
        "proc helper {a b} {}".to_owned(),
        "tcl".to_owned(),
        None,
    );
    let source = "helper ONE";
    let caller = SourceFile::new(&db, source.to_owned(), "tcl".to_owned(), None);
    let project = Project::new(&db, vec![provider, caller]);
    let mut analysis = Analyser::new().analyse(source, "tcl");
    analysis.unresolved_command_sites.clear();
    analysis.diagnostics.clear();
    analysis.command_invocations.clear();
    let diagnostics = project_source_diagnostics_for_analysis(
        (&db, project, ProjectSourceInputMode::Standalone),
        source,
        &analysis,
        |_| false,
    );
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].code, DiagCode::E002);
}

#[test]
fn logical_callback_source_slots_preserve_namespace_order_and_missing_project_inputs() {
    let db = TclDatabase::default();
    let provider = SourceFile::new(
        &db,
        "namespace eval A {proc helper {first second} {}}; namespace eval B {proc helper {} {}}"
            .to_owned(),
        "tcl".to_owned(),
        None,
    );
    let source = "namespace eval A {proc caller {} {lsort -command helper {a b}}}; namespace eval B {proc caller {} {lsort -command helper {a b}}}";
    let caller = SourceFile::new(&db, source.to_owned(), "tcl".to_owned(), None);
    let project = Project::new(&db, vec![provider, caller]);
    let analysis = Analyser::new().analyse(source, "tcl");
    let diagnostics =
        project_callback_diagnostics_for_analysis(&db, project, source, &analysis, |_| false);
    let errors = diagnostics
        .iter()
        .filter(|diagnostic| diagnostic.code == DiagCode::E003)
        .collect::<Vec<_>>();
    assert_eq!(errors.len(), 1);
    assert!(
        errors[0].span.start() > u32::try_from(source.find("namespace eval B").unwrap()).unwrap()
    );
    let unavailable = Project::with_token_configurations(
        &db,
        vec![provider, caller],
        Some(vec![(caller, cfg(&db))]),
    );
    let supplied = project_callback_diagnostics_for_analysis_with_inputs(
        &db,
        unavailable,
        source,
        &analysis,
        |_| false,
    );
    assert!(
        !supplied
            .iter()
            .any(|diagnostic| diagnostic.code == DiagCode::E003)
    );
}

#[test]
fn project_direct_logical_calls_use_original_occurrences_and_namespace_slots() {
    let db = TclDatabase::default();
    let cfg = AnalyserConfig::new(
        &db,
        Vec::new(),
        NonAsciiMode::Default,
        Vec::new(),
        None,
        None,
        0,
        Vec::new(),
        Vec::new(),
    );
    let definitions = SourceFile::new(
        &db,
        "namespace eval A {proc helper {x} {}}; namespace eval B {proc helper {x y} {}}".to_owned(),
        "tcl".to_owned(),
        None,
    );
    let source = "proc callA {} {namespace eval A {helper ONE}}; proc callB {} {namespace eval B {helper ONE}}";
    let calls = SourceFile::new(&db, source.to_owned(), "tcl".to_owned(), None);
    let project = Project::new(&db, vec![definitions, calls]);
    let diagnostics = project_diagnostics(&db, calls, cfg, project);
    assert!(
        !diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == DiagCode::W123)
    );
    let arity = diagnostics
        .iter()
        .filter(|diagnostic| matches!(diagnostic.code, DiagCode::E002 | DiagCode::E003))
        .collect::<Vec<_>>();
    assert_eq!(arity.len(), 1);
    assert_eq!(arity[0].code, DiagCode::E002);
    assert_eq!(
        arity[0].span.start(),
        u32::try_from(source.rfind("helper").unwrap()).unwrap()
    );
}

#[test]
fn project_direct_logical_headers_preserve_source_occupants() {
    let db = TclDatabase::default();
    let cfg = AnalyserConfig::new(
        &db,
        Vec::new(),
        NonAsciiMode::Default,
        Vec::new(),
        None,
        None,
        0,
        Vec::new(),
        Vec::new(),
    );
    let definitions = SourceFile::new(
        &db,
        "proc puts {x y} {}; proc helper {x y} {}; proc delegated {x y} {}".to_owned(),
        "tcl".to_owned(),
        None,
    );
    for source in [
        "puts ONE",
        "proc helper {x} {}; helper ONE",
        "interp alias {} delegated {} puts; delegated ONE",
    ] {
        let calls = SourceFile::new(&db, source.to_owned(), "tcl".to_owned(), None);
        let project = Project::new(&db, vec![definitions, calls]);
        let diagnostics = project_diagnostics(&db, calls, cfg, project);
        assert!(
            !diagnostics
                .iter()
                .any(|diagnostic| matches!(diagnostic.code, DiagCode::E002 | DiagCode::E003)),
            "{source}: {diagnostics:?}"
        );
    }
}

#[test]
fn project_direct_logical_headers_do_not_restore_deleted_local_slots() {
    let db = TclDatabase::default();
    let cfg = AnalyserConfig::new(
        &db,
        Vec::new(),
        NonAsciiMode::Default,
        Vec::new(),
        None,
        None,
        0,
        Vec::new(),
        Vec::new(),
    );
    let source = "namespace eval A {proc helper {x} {}; rename helper {}; helper ONE}";
    let calls = SourceFile::new(&db, source.to_owned(), "tcl".to_owned(), None);
    for (headers, expected) in [
        (
            "namespace eval A {proc helper {x y} {}}; proc helper {x} {}",
            0,
        ),
        (
            "namespace eval A {proc helper {x} {}}; proc helper {x y} {}",
            1,
        ),
    ] {
        let definitions = SourceFile::new(&db, headers.to_owned(), "tcl".to_owned(), None);
        let project = Project::new(&db, vec![definitions, calls]);
        let diagnostics = project_diagnostics(&db, calls, cfg, project);
        let arity = diagnostics
            .iter()
            .filter(|diagnostic| matches!(diagnostic.code, DiagCode::E002 | DiagCode::E003))
            .collect::<Vec<_>>();
        assert_eq!(arity.len(), expected, "{headers}: {diagnostics:?}");
        if let Some(diagnostic) = arity.first() {
            assert_eq!(diagnostic.code, DiagCode::E002);
            assert_eq!(
                diagnostic.span.start(),
                u32::try_from(source.find("helper ONE").unwrap()).unwrap()
            );
        }
    }
}

#[test]
fn project_direct_logical_calls_keep_operand_mutations_unresolved() {
    let db = TclDatabase::default();
    let cfg = AnalyserConfig::new(
        &db,
        Vec::new(),
        NonAsciiMode::Default,
        Vec::new(),
        None,
        None,
        0,
        Vec::new(),
        Vec::new(),
    );
    let definitions = SourceFile::new(
        &db,
        "proc rooted {x y} {}".to_owned(),
        "tcl".to_owned(),
        None,
    );
    let source = "rooted [set value VALUE]";
    let calls = SourceFile::new(&db, source.to_owned(), "tcl".to_owned(), None);
    let project = Project::new(&db, vec![definitions, calls]);
    let diagnostics = project_diagnostics(&db, calls, cfg, project);
    assert!(
        diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == DiagCode::W123 && diagnostic.span.start() == 0)
    );
    assert!(
        !diagnostics
            .iter()
            .any(|diagnostic| matches!(diagnostic.code, DiagCode::E002 | DiagCode::E003))
    );
}

#[test]
fn project_direct_logical_calls_keep_root_recipe_and_missing_namespace_unknown() {
    let db = TclDatabase::default();
    let cfg = AnalyserConfig::new(
        &db,
        Vec::new(),
        NonAsciiMode::Default,
        Vec::new(),
        None,
        None,
        0,
        Vec::new(),
        Vec::new(),
    );
    let definitions = SourceFile::new(
        &db,
        "proc rooted {x} {}; namespace eval A {proc helper {x} {}}".to_owned(),
        "tcl".to_owned(),
        None,
    );
    let source = "rooted ONE; namespace eval Missing {helper ONE}";
    let calls = SourceFile::new(&db, source.to_owned(), "tcl".to_owned(), None);
    let project = Project::new(&db, vec![definitions, calls]);
    let diagnostics = project_diagnostics(&db, calls, cfg, project);
    let unknown = diagnostics
        .iter()
        .filter(|diagnostic| diagnostic.code == DiagCode::W123)
        .collect::<Vec<_>>();
    assert_eq!(unknown.len(), 1);
    assert_eq!(
        unknown[0].span.start(),
        u32::try_from(source.find("helper").unwrap()).unwrap()
    );
    assert!(
        !diagnostics
            .iter()
            .any(|diagnostic| matches!(diagnostic.code, DiagCode::E002 | DiagCode::E003))
    );
}
