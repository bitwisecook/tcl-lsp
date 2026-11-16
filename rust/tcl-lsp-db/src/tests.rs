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
fn original_callback_missing_future_lookup_declines_reporting_candidates() {
    // Implementation contract naming.database.original-callback-future-lookup:
    // docs/design/analysis/name-resolution-proofs/database-original-callback-future-lookup.md
    let db = TclDatabase::default();
    let file = SourceFile::new(
        &db,
        "proc cb {} {}\ncb\n".to_owned(),
        "tcl8.6".to_owned(),
        None,
    );
    let analysis = file_analysis_incremental(&db, file, cfg(&db));
    let mut invocation = analysis
        .command_invocations
        .iter()
        .find(|invocation| invocation.name == "cb" && invocation.original_name_input.is_some())
        .expect("an actual original command producer")
        .clone();
    // This checks missing-purpose refusal, not callback context inference:
    // the independently retained name cannot manufacture a future lookup.
    invocation.original_lookup = None;
    invocation.resolved_command_reference = None;
    invocation.callback_arity = Some(tcl_registry::AppendedArity::Exactly(2));
    assert_eq!(invocation.logical_callback_candidate_keys().len(), 0);
    let mut missing = (*analysis).clone();
    missing.command_invocations = vec![invocation];
    let library = SourceFile::new(&db, "proc cb {} {}".to_owned(), "tcl8.6".to_owned(), None);
    let project = Project::new(&db, vec![library, file]);
    assert!(
        project_callback_diagnostics_for_analysis(&db, project, file.text(&db), &missing, |_| {
            false
        },)
        .iter()
        .all(|diagnostic| !matches!(
            diagnostic.code,
            DiagCode::E002 | DiagCode::E003 | DiagCode::E005
        ))
    );
}

#[test]
fn formal_count_project_headers_keep_the_selected_c_and_jim_contract() {
    // naming.database.original-formal-count-header
    // docs/design/analysis/name-resolution-proofs/database-original-formal-count-header.md
    use salsa::Setter as _;
    use tcl_compiler::signature_scan::formal_count::SourceFormalCountOrigin;
    for (dialect, minimum, grammar) in [
        ("tcl8.4", 3, tcl_dialect::ParameterGrammar::Tcl),
        ("tcl8.5", 3, tcl_dialect::ParameterGrammar::Tcl),
        ("tcl8.6", 3, tcl_dialect::ParameterGrammar::Tcl),
        ("tcl9.0", 3, tcl_dialect::ParameterGrammar::Tcl),
        ("tcl9.1", 3, tcl_dialect::ParameterGrammar::Tcl),
        ("jim", 2, tcl_dialect::ParameterGrammar::Jim),
    ] {
        let mut db = TclDatabase::default();
        let file = SourceFile::new(
            &db,
            "proc mixed {a {b B} c} {return FIRST}".to_owned(),
            dialect.to_owned(),
            None,
        );
        let project = Project::new(&db, vec![file]);
        let headers = item_sigs(&db, file);
        let signature = &headers[0];
        let header = signature.original_declaration.as_ref().unwrap();
        let count = header.formal_count_projection();
        assert_eq!(
            count.origin(),
            SourceFormalCountOrigin::OriginalSource,
            "{dialect}"
        );
        assert_eq!(count.parameter_grammar(), Some(grammar), "{dialect}");
        let key = header.name().clone();
        let original = project_original_command_arities(&db, project);
        assert_eq!(original.get(&key), Some(&vec![(minimum, 3)]), "{dialect}");
        let selected = original_command_arity(
            &db,
            project,
            OriginalCommandSlot::new(&db, key.policy(), key.slot().clone()),
        );
        assert_eq!(selected.as_deref(), Some(&vec![(minimum, 3)]), "{dialect}");
        file.set_text(&mut db)
            .to("proc mixed {a {b B} c} {return A_LONGER_BODY}".to_owned());
        assert_eq!(
            project_original_command_arities(&db, project),
            original,
            "{dialect}"
        );
        assert_eq!(
            original_command_arity(
                &db,
                project,
                OriginalCommandSlot::new(&db, key.policy(), key.slot().clone()),
            ),
            selected,
            "{dialect}"
        );
        assert_eq!(item_sigs(&db, file), headers, "{dialect}");
    }
}

#[test]
fn original_project_headers_keep_opaque_slots_and_body_free_arity() {
    // Proof naming.database.original-header-arity:
    // docs/design/analysis/name-resolution-proofs/database-original-header-arity.md
    use salsa::Setter as _;
    let mut db = TclDatabase::default();
    let file = SourceFile::new(
        &db,
        r"proc p\uD800 {a} {return FIRST}
proc p\uD801 {a b} {return SECOND}"
            .to_owned(),
        "tcl8.6".to_owned(),
        None,
    );
    let project = Project::new(&db, vec![file]);
    let declarations = file_decls(&db, file);
    let first = declarations
        .original_declarations
        .iter()
        .find(|header| header.name().slot().simple.as_bytes() == b"p\xed\xa0\x80")
        .expect("first original byte declaration")
        .name()
        .clone();
    let second = declarations
        .original_declarations
        .iter()
        .find(|header| header.name().slot().simple.as_bytes() == b"p\xed\xa0\x81")
        .expect("second original byte declaration")
        .name()
        .clone();
    let first_key = OriginalCommandSlot::new(&db, first.policy(), first.slot().clone());
    let second_key = OriginalCommandSlot::new(&db, second.policy(), second.slot().clone());
    assert_eq!(
        original_command_arity(&db, project, first_key)
            .unwrap()
            .as_ref(),
        &[(1, 1)]
    );
    assert_eq!(
        original_command_arity(&db, project, second_key)
            .unwrap()
            .as_ref(),
        &[(2, 2)]
    );
    let wrong_provider = OriginalCommandSlot::new(
        &db,
        tcl_syntax::naming::NamePolicyProtocol::authored_tcl(tcl_dialect::TclVersion::V8_5),
        first.slot().clone(),
    );
    assert!(original_command_arity(&db, project, wrong_provider).is_none());
    let before = project_original_command_arities(&db, project);
    file.set_text(&mut db)
        .to(r"proc p\uD800 {a} {return A_LONGER_BODY}
proc p\uD801 {a b} {return ANOTHER_BODY}"
            .to_owned());
    assert_eq!(project_original_command_arities(&db, project), before);
    assert_eq!(
        original_command_arity(
            &db,
            project,
            OriginalCommandSlot::new(&db, first.policy(), first.slot().clone())
        )
        .unwrap()
        .as_ref(),
        &[(1, 1)]
    );
    file.set_text(&mut db)
        .to(r"proc p\uD800 {a b c} {return FIRST}
proc p\uD801 {a b} {return SECOND}"
            .to_owned());
    assert_eq!(
        original_command_arity(
            &db,
            project,
            OriginalCommandSlot::new(&db, first.policy(), first.slot().clone())
        )
        .unwrap()
        .as_ref(),
        &[(3, 3)]
    );
    assert_eq!(
        original_command_arity(
            &db,
            project,
            OriginalCommandSlot::new(&db, second.policy(), second.slot().clone())
        )
        .unwrap()
        .as_ref(),
        &[(2, 2)]
    );
}

#[test]
fn original_project_diagnostics_select_opaque_call_slots_and_namespace_priority() {
    // Implementation contract: naming.database.original-call-arity-diagnostics
    // docs/design/analysis/name-resolution-proofs/database-original-call-arity-diagnostics.md
    let db = TclDatabase::default();
    let config = cfg(&db);
    let provider = SourceFile::new(
        &db,
        r"proc p\uD800 {a} {}; proc p\uD801 {a b} {};
namespace eval N {proc only_here {a} {}}"
            .to_owned(),
        "tcl8.6".to_owned(),
        None,
    );
    let caller = SourceFile::new(
        &db,
        r"p\uD800 A B
p\uD801 A B"
            .to_owned(),
        "tcl8.6".to_owned(),
        None,
    );
    let project = Project::new(&db, vec![provider, caller]);
    let analysis = file_analysis_incremental(&db, caller, config);
    let calls = analysis
        .command_invocations
        .iter()
        .filter(|invocation| invocation.original_name_input.is_some())
        .collect::<Vec<_>>();
    let first = calls
        .iter()
        .find(|invocation| {
            invocation
                .original_name_input
                .as_ref()
                .is_some_and(|input| input.bytes() == b"p\xed\xa0\x80")
        })
        .expect("original first call");
    let second = calls
        .iter()
        .find(|invocation| {
            invocation
                .original_name_input
                .as_ref()
                .is_some_and(|input| input.bytes() == b"p\xed\xa0\x81")
        })
        .expect("original second call");
    let diagnostics = project_diagnostics(&db, caller, config, project);
    assert!(
        diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == DiagCode::E003 && diagnostic.span == first.range),
        "first byte name accepts exactly one argument"
    );
    assert!(
        !diagnostics
            .iter()
            .any(|diagnostic| diagnostic.span == second.range),
        "distinct second byte name accepts two arguments"
    );
    // Namespace advice is an independent source point. The external
    // calls above do not close the world for a following command.
    let unrelated = SourceFile::new(&db, "only_here A".to_owned(), "tcl8.6".to_owned(), None);
    let unrelated_project = Project::new(&db, vec![provider, unrelated]);
    let unrelated_diagnostics = project_diagnostics(&db, unrelated, config, unrelated_project);
    let unresolved = unrelated_diagnostics
        .iter()
        .filter(|diagnostic| diagnostic.code == DiagCode::W123)
        .collect::<Vec<_>>();
    assert_eq!(
        unresolved.len(),
        1,
        "unrelated namespace tails cannot suppress this call"
    );
    assert_eq!(
        unresolved[0]
            .unresolved_command()
            .unwrap()
            .name_input()
            .bytes(),
        b"only_here"
    );
}

fn original_db_token_is_variable(
    tokens: &SemanticTokens,
    image: &str,
    needle: &str,
    variable: u32,
) -> bool {
    let offset = u32::try_from(image.rfind(needle).unwrap()).unwrap();
    let target = tcl_lexer::LineIndex::new(image).position_at_utf16(offset, image);
    let mut line = 0;
    let mut column = 0;
    tokens.data.as_chunks::<5>().0.iter().any(|entry| {
        if entry[0] == 0 {
            column += entry[1];
        } else {
            line += entry[0];
            column = entry[1];
        }
        line == target.line
            && column <= target.character.get()
            && target.character.get() < column + entry[2]
            && entry[3] == variable
    })
}

#[test]
fn original_db_token_roles_keep_opaque_declarations_and_full_caller_currency() {
    // Implementation contract: naming.database.original-token-role-consumer
    // docs/design/analysis/name-resolution-proofs/database-original-token-role-consumer.md
    use tcl_lsp_core::semantic_tokens::{
        WorkspaceTokenFacts, full_with_cu_and_facts, legend_token_types,
    };
    let source = r"proc p\uD800 {target} {upvar 1 $target cell; set cell 1}
proc p\uD801 {target} {return $target}
p\uD800 written
p\uD801 ordinary";
    let db = TclDatabase::default();
    let file = SourceFile::new(&db, source.to_owned(), "tcl8.6".to_owned(), None);
    let project = Project::new(&db, vec![file]);
    let facts = file_token_facts(&db, file);
    let roles = project_proc_var_index(&db, project);
    assert!(
        !facts.proc_roles.is_empty(),
        "the retained writer declaration has a role"
    );
    assert_eq!(facts.proc_roles, *roles);
    let mut analysis = (*file_analysis_incremental(&db, file, cfg(&db))).clone();
    analysis.all_procs.clear();
    analysis.superseded_procs.clear();
    for invocation in &mut analysis.command_invocations {
        invocation.name = "REPORT-COLLISION".to_owned();
        invocation.resolution_candidates = vec!["REPORT-COLLISION".to_owned()];
    }
    let registry = db.registry(file.dialect(&db));
    let profile = tcl_lsp_core::profile_for_dialect(file.dialect(&db));
    let unit = document_compilation_unit(&db, file);
    let render = |image: &str, current: &AnalysisResult| {
        full_with_cu_and_facts(
            image,
            profile,
            registry,
            Some(&unit),
            WorkspaceTokenFacts {
                proc_roles: Some(&roles),
                analysis: Some(current),
                ..Default::default()
            },
        )
    };
    let variable = u32::try_from(
        legend_token_types()
            .iter()
            .position(|kind| *kind == "variable")
            .unwrap(),
    )
    .unwrap();
    let is_variable = |tokens: &SemanticTokens, image: &str, needle: &str| {
        original_db_token_is_variable(tokens, image, needle, variable)
    };
    let tokens = render(source, &analysis);
    assert!(
        is_variable(&tokens, source, "written"),
        "the exact opaque writer retains its caller argument role after reporting clear"
    );
    assert!(
        !is_variable(&tokens, source, "ordinary"),
        "the distinct opaque ordinary procedure does not borrow the writer role"
    );
    let changed = format!("{source} ");
    assert!(!is_variable(
        &render(&changed, &analysis),
        &changed,
        "written"
    ));
    let mut changed_config = analysis.clone();
    changed_config
        .body_lexer_config
        .as_mut()
        .unwrap()
        .strict_quoting ^= true;
    assert!(!is_variable(
        &render(source, &changed_config),
        source,
        "written"
    ));
    let mut missing_input = analysis;
    let call = u32::try_from(source.rfind(r"p\uD800 written").unwrap()).unwrap();
    missing_input
        .command_invocations
        .iter_mut()
        .find(|invocation| invocation.range.start() == call)
        .unwrap()
        .original_name_input = None;
    assert!(!is_variable(
        &render(source, &missing_input),
        source,
        "written"
    ));
}

#[test]
fn body_cache_gate_is_per_body_and_whitespace_aware() {
    use tcl_compiler::lowering::body_cache_eligible;
    // A plain, context-free body is eligible.
    assert!(body_cache_eligible(" set x 1 "));
    assert!(body_cache_eligible(" puts hi "));
    // A body carrying a cross-item construct is not — including the tab-
    // separated forms: the isolated lowerer drops the effect.
    assert!(!body_cache_eligible(" interp\talias {} x {} y "));
    assert!(!body_cache_eligible(" namespace\timport ::ns::* "));
    assert!(!body_cache_eligible(" rename set myset "));
    assert!(!body_cache_eligible(" oo::class create C "));
    // A nested `proc` disqualifies the enclosing body.
    assert!(!body_cache_eligible(" proc inner {} {} "));
    // The gate is per-body: a context-carrying sibling does not disable the
    // clean body — the clean body stays eligible on its own.
    assert!(body_cache_eligible(" set y 2 "));
}

#[test]
fn compiler_check_o122_tailrec_memo_matches_uncached() {
    // An impure (side-effecting) tail-recursive proc fires O122
    // (recursion→loop). The per-proc optimise memo must reconstruct
    // `proc.body_source` as the *body text* — not the whole-command slice used
    // for span alignment — so the loop-conversion replacement wraps only the
    // body, not the entire `proc …` declaration.
    let dialect = "tcl8.6";
    let src = "proc countdown {n} {\n    puts $n\n    if {$n <= 0} { return }\n    countdown [expr {$n - 1}]\n}\n";
    let db = TclDatabase::default();
    let registry = db.registry(dialect);
    let file = SourceFile::new(&db, src.to_owned(), dialect.to_owned(), None);
    let got = compiler_check_diagnostics(&db, file, cfg(&db));
    let want = compiler_check_diagnostics_uncached(src, registry, dialect, None, None);
    assert!(
        got.optimisations.iter().any(|o| o.code == DiagCode::O122),
        "expected O122 to fire on the tail-recursive proc"
    );
    assert_eq!(
        got.optimisations, want.optimisations,
        "per-proc optimise memo diverged from the whole-module build"
    );
}

#[test]
fn original_document_policy_key_admits_actual_input_and_root_bom_geometry() {
    // naming.source.original-configured-script-completeness
    // docs/design/analysis/name-resolution-proofs/source-original-configured-script-completeness.md
    // The actual memoised source build supplies no Native entry or execution.
    let source = "\u{feff}probe root\n\u{feff}probe later\n";
    for name in [
        "tcl8.4",
        "tcl8.5",
        "tcl8.6",
        "tcl9.0",
        "tcl9.1",
        "jim",
        "f5-irules",
        "f5-iapps",
        "f5-tmsh",
    ] {
        let db = TclDatabase::default();
        let file = SourceFile::new(&db, source.to_owned(), name.to_owned(), None);
        let policy = cfg(&db);
        let input =
            document_analysis_input(&db, file, policy).expect("actual document source input");
        let key = lexer_cfg_key(&db, name);
        assert_eq!(key.to_config(&db), input.lexer_config(), "{name}");
        let unit = compilation_unit_for_config(&db, file, key, policy)
            .expect("complete document policy must join its actual input");
        assert_eq!(
            unit.ir_module.source,
            tcl_lexer::SourceImage::document(source),
            "{name}"
        );
        assert_eq!(
            unit.ir_module.source_metadata_input.as_ref(),
            Some(input.as_ref()),
            "{name}"
        );
        assert_eq!(unit.ir_module.lexer_config, input.lexer_config(), "{name}");
        assert_eq!(unit.ir_module.top_level.statements.len(), 2, "{name}");
        let first = unit.ir_module.top_level.statements[0]
            .tokens()
            .expect("original root command");
        let expected = if input.unit_profile().grammar.script_skips_leading_bom {
            "probe"
        } else {
            "\u{feff}probe"
        };
        assert_eq!(first.argv_texts[0], expected, "{name}");
        assert_eq!(&source[first.argv[0].as_range()], expected, "{name}");
        let later = unit.ir_module.top_level.statements[1]
            .tokens()
            .expect("original later command");
        assert_eq!(later.argv_texts[0], "\u{feff}probe", "{name}");
        assert_eq!(&source[later.argv[0].as_range()], "\u{feff}probe", "{name}");
    }
}

#[test]
fn original_document_policy_key_refuses_foreign_or_changed_full_input() {
    // naming.source.original-configured-script-completeness
    // docs/design/analysis/name-resolution-proofs/source-original-configured-script-completeness.md
    let db = TclDatabase::default();
    let file = SourceFile::new(
        &db,
        "\u{feff}probe root".to_owned(),
        "tcl9.1".to_owned(),
        None,
    );
    let policy = cfg(&db);
    let input = document_analysis_input(&db, file, policy).unwrap();
    let key = lexer_cfg_key(&db, "tcl9.1");
    let registry = input.borrowed_context_registry().commands();
    let declared = declared_command_surface(&db, file, registry);
    assert!(
        supplied_unit_build_options(&db, file, key, registry, None, &declared, &input).is_some()
    );
    let foreign_key = lexer_cfg_key(&db, "tcl8.6");
    assert!(compilation_unit_for_config(&db, file, foreign_key, policy).is_none());
    for config in [
        tcl_lexer::LexerConfig {
            leading_bom: tcl_lexer::LeadingBom::Content,
            ..input.lexer_config()
        },
        tcl_lexer::LexerConfig {
            strict_quoting: !input.lexer_config().strict_quoting,
            ..input.lexer_config()
        },
    ] {
        let changed = tcl_compiler::analyser::ResolvedAnalysisInput::new(
            input.analyser_profile(),
            input.unit_profile(),
            input.context_registry(),
            config,
        );
        assert!(
            supplied_unit_build_options(&db, file, key, registry, None, &declared, &changed)
                .is_none()
        );
    }
    let mut foreign_registry = registry.project_for_profile(input.unit_profile());
    let mut changed_spec = foreign_registry.get("puts").unwrap().clone();
    if changed_spec
        .traits
        .contains(tcl_registry::Traits::EVALUATES_CODE)
    {
        changed_spec
            .traits
            .remove(tcl_registry::Traits::EVALUATES_CODE);
    } else {
        changed_spec
            .traits
            .insert(tcl_registry::Traits::EVALUATES_CODE);
    }
    foreign_registry.insert(changed_spec);
    assert!(
        supplied_unit_build_options(&db, file, key, &foreign_registry, None, &declared, &input)
            .is_none()
    );
}

#[test]
fn original_standalone_compiler_document_keeps_root_and_later_bom_roles() {
    // naming.source.original-configured-script-completeness
    // docs/design/analysis/name-resolution-proofs/source-original-configured-script-completeness.md
    // The scalar diagnostics route shares this genuine fresh source builder.
    let source = "\u{feff}probe root\n\u{feff}probe later\n";
    for name in [
        "tcl8.4",
        "tcl8.5",
        "tcl8.6",
        "tcl9.0",
        "tcl9.1",
        "jim",
        "f5-irules",
        "f5-iapps",
        "f5-tmsh",
    ] {
        let environment = tcl_registry::model::ingress::resolve_known_environment(name)
            .expect("actual known source policy");
        let context = environment.default_context_registry();
        let unit = fresh_compiler_document_unit(source, context.commands(), name, None);
        assert_eq!(
            unit.ir_module.lexer_config,
            tcl_lexer::LexerConfig::for_file_grammar(environment.grammar()),
            "{name}"
        );
        assert_eq!(
            unit.ir_module.source,
            tcl_lexer::SourceImage::document(source),
            "{name}"
        );
        assert_eq!(unit.ir_module.top_level.statements.len(), 2, "{name}");
        let first = unit.ir_module.top_level.statements[0].tokens().unwrap();
        let expected = if environment.grammar().script_skips_leading_bom {
            "probe"
        } else {
            "\u{feff}probe"
        };
        assert_eq!(first.argv_texts[0], expected, "{name}");
        assert_eq!(&source[first.argv[0].as_range()], expected, "{name}");
        let later = unit.ir_module.top_level.statements[1].tokens().unwrap();
        assert_eq!(later.argv_texts[0], "\u{feff}probe", "{name}");
        assert_eq!(&source[later.argv[0].as_range()], "\u{feff}probe", "{name}");
    }
}

#[test]
fn file_analysis_matches_direct_analyse() {
    let db = TclDatabase::default();
    let file = SourceFile::new(&db, SRC.to_owned(), "tcl".to_owned(), None);
    let got = file_analysis(&db, file, cfg(&db));

    let mut direct = Analyser::new();
    let expected = direct.analyse(SRC, "tcl");
    assert_eq!(*got, expected);
    assert!(got.all_procs.contains_key("::greet"));
}

/// A memo *hit* must hand its per-procedure lattices over by
/// refcount, not deep-copy them.
///
/// The span-carrying halves of a `FunctionUnit` (`cfg` / `ssa` /
/// `sccp.constant_branches`) genuinely have to be copied — the memo stores
/// the unit at offset 0 and every consumer is rebased to the procedure's
/// real position — but `def_use` / `types` / `taints` / `rendered_props` are
/// span-free, so a rebuild that hits the memo must share them.
///
/// Two *distinct* `SourceFile`s carrying the same text give two distinct
/// `compilation_unit` keys, so both really build — while their per-procedure
/// lattice demands land on the same `function_lattice` memos.  Driving it
/// through the tracked query rather than calling `memoised_compilation_unit`
/// directly is deliberate: the per-body keys are only garbage-collectable
/// when interned inside a tracked query (see the crate docs' "The interned
/// garbage collector is load-bearing").
#[test]
fn memoised_lattices_are_shared_not_deep_copied_across_builds() {
    const SRC: &str = "proc alpha {a b} {\n    set s [expr {$a + $b}]\n    return $s\n}\n\
                           proc beta {x} {\n    return [alpha $x 1]\n}\n";
    let db = TclDatabase::default();
    let cfg_key = lexer_cfg_key(&db, "tcl8.6");
    let file_a = SourceFile::new(&db, SRC.to_owned(), "tcl8.6".to_owned(), None);
    let file_b = SourceFile::new(&db, SRC.to_owned(), "tcl8.6".to_owned(), None);
    let first = compilation_unit(&db, file_a, cfg_key, 0).expect("no overlay always builds");
    let second = compilation_unit(&db, file_b, cfg_key, 0).expect("no overlay always builds");
    for (qname, offset) in [("::alpha", 0), ("::beta", SRC.find("proc beta").unwrap())] {
        let first_id = original_test_body_id(&first, offset);
        let second_id = original_test_body_id(&second, offset);
        let a = first
            .function_for_body(Some(first_id))
            .expect("procedure in first build");
        let b = second
            .function_for_body(Some(second_id))
            .expect("procedure in second build");
        assert!(
            Arc::ptr_eq(&a.def_use, &b.def_use),
            "{qname}: def-use chains must be shared across a memo hit",
        );
        assert!(
            Arc::ptr_eq(&a.types, &b.types),
            "{qname}: the type lattice must be shared across a memo hit",
        );
        assert!(
            Arc::ptr_eq(&a.rendered_props, &b.rendered_props),
            "{qname}: rendered properties must be shared across a memo hit",
        );
        assert!(
            Arc::ptr_eq(&a.taints, &b.taints),
            "{qname}: the taint cascade result must be shared across a memo hit",
        );
    }
}

/// A per-procedure lattice is rebuilt through the public single-function
/// CFG entry point, so that entry point must receive the document's exact
/// registry surface. Tcl 8.4 has no builtin `throw`: a user procedure with
/// that name returns normally and the following statement stays reachable.
#[test]
fn tcl84_memoised_cfg_keeps_user_throw_call_fallthrough_reachable() {
    const SRC: &str = "proc throw {} {return ok}\n\
                           proc subject {} {throw; set after_throw 1}\n";
    let db = TclDatabase::default();
    let cfg_key = lexer_cfg_key(&db, "tcl8.4");
    let file = SourceFile::new(&db, SRC.to_owned(), "tcl8.4".to_owned(), None);
    let unit = compilation_unit(&db, file, cfg_key, 0).expect("no overlay always builds");
    let subject_id = original_test_body_id(&unit, SRC.find("proc subject").unwrap());
    let subject = unit
        .procedures
        .get(&subject_id)
        .expect("subject procedure lattice");

    assert!(
        subject.cfg.blocks.values().any(|block| {
            block.statements.iter().any(|stmt| {
                matches!(
                    stmt,
                    tcl_compiler::ir::Statement::AssignConst { name, .. }
                        if name == "after_throw"
                )
            })
        }),
        "Tcl 8.4 user proc `throw` must fall through to `after_throw`"
    );
}

#[test]
// Implementation contract: naming.variable.registry-event-frame-identity
// docs/design/analysis/name-resolution-proofs/variable-registry-event-frame-identity.md
// Implementation contract: naming.compiler.original-function-lattice-mutation-identity
// docs/design/analysis/name-resolution-proofs/compiler-original-function-lattice-mutation-identity.md
fn original_event_lattice_cache_retains_conditional_entry_and_reuses_same_source() {
    let source = "when HTTP_REQUEST {set local 1}";
    let db = TclDatabase::default();
    let config = lexer_cfg_key(&db, "f5-irules");
    let first_file = SourceFile::new(&db, source.to_owned(), "f5-irules".to_owned(), None);
    let second_file = SourceFile::new(&db, source.to_owned(), "f5-irules".to_owned(), None);
    let first = compilation_unit(&db, first_file, config, 0)
        .expect("zero overlay selects the retained stock F5 Registry");
    let second = compilation_unit(&db, second_file, config, 0)
        .expect("zero overlay selects the retained stock F5 Registry");
    let first_id = original_test_body_id(&first, 0);
    let second_id = original_test_body_id(&second, 0);
    let a = first.procedures.get(&first_id).unwrap();
    let b = second.procedures.get(&second_id).unwrap();
    assert!(
        Arc::ptr_eq(&a.def_use, &b.def_use),
        "the sealed event participates in the lattice memo"
    );
    for unit in [a, b] {
        let event = unit
            .irules_event_body
            .as_ref()
            .expect("actual event producer");
        assert_eq!(event.event(), "HTTP_REQUEST");
        assert!(event.matches_source(
            &tcl_lexer::SourceImage::document(source),
            first.ir_module.lexer_config
        ));
        let (&block, _) = unit
            .cfg
            .blocks
            .iter()
            .find(|(_, block)| !block.statements.is_empty())
            .unwrap();
        let point = unit
            .ssa
            .point_contexts
            .as_ref()
            .unwrap()
            .context_before(block, 0)
            .unwrap();
        assert_eq!(
            point.frame_kind,
            tcl_compiler::var_resolve::VariableFrameKind::Local
        );
        assert_eq!(
            point.hosted_execution_context,
            Some(tcl_registry::f5::BigIpExecutionContext::TmmIRule)
        );
        assert_eq!(
            point.execution, None,
            "memoisation cannot manufacture an executing worker"
        );
    }
}

fn original_event_test_context_snapshot(db: &dyn TclDb) -> (CfgContext<'_>, CompilerMemoSnapshot) {
    let context = CfgContext::new(
        db,
        Vec::new(),
        Vec::new(),
        Vec::new(),
        BodyLatticeCommandBindings::from(ModuleCommandBindings::default()),
        OriginalProcedureSummaryOwners::default(),
    );
    let snapshot = CompilerMemoSnapshot {
        profile: Some(tcl_dialect::DialectProfile::irules().cache_key()),
        registry: db.registry("f5-irules").snapshot(),
    };
    (context, snapshot)
}

fn original_event_changed_mutations<'db>(
    db: &'db dyn TclDb,
    selected: FnLatticeKey<'db>,
) -> FnLatticeKey<'db> {
    FnLatticeKey::new(
        db,
        selected.body(db).clone(),
        selected.qname(db).clone(),
        selected.params(db).clone(),
        selected.context(db),
        selected.lexer_config(db),
        selected.dialect(db).clone(),
        selected.snapshot(db).clone(),
        selected.param_constants(db).clone(),
        selected.module_facts(db).clone(),
        FnLatticeEntry {
            original_declaration_body: None,
            plain_command_dispatch: selected.entry(db).plain_command_dispatch,
            metadata_context: selected.entry(db).metadata_context.clone(),
            source_metadata_input: selected.entry(db).source_metadata_input.clone(),
            irules_event_body: selected.entry(db).irules_event_body.clone(),
            command_trust: tcl_compiler::command_binding::ModuleCommandMutations::distrust_all()
                .snapshot(),
        },
        selected.analysis_context(db),
    )
}

type OriginalEventKeyBase<'db, 'a> = (
    &'db dyn TclDb,
    &'a CompilationUnit,
    CfgContext<'db>,
    &'a str,
);
type OriginalEventKeyRequest = (
    Script,
    tcl_lexer::LexerConfig,
    CompilerMemoSnapshot,
    Option<Arc<tcl_compiler::ir::SourceIrulesEventBody>>,
);

fn original_event_make_key<'db>(
    (db, unit, context, qname): OriginalEventKeyBase<'db, '_>,
    (body, config, snapshot, event): OriginalEventKeyRequest,
) -> FnLatticeKey<'db> {
    FnLatticeKey::new(
        db,
        BodyLatticeInputs::from(body),
        qname.to_owned(),
        Vec::new(),
        context,
        config,
        "f5-irules".to_owned(),
        snapshot,
        Vec::new(),
        FnLatticeModuleFacts {
            known_classes: Vec::new(),
            traced_variables: Vec::new(),
            has_dynamic_variable_trace: false,
        },
        FnLatticeEntry {
            original_declaration_body: None,
            plain_command_dispatch: false,
            metadata_context: unit.ir_module.source_entry.metadata_context.clone(),
            source_metadata_input: unit.ir_module.source_metadata_input.clone(),
            irules_event_body: event,
            command_trust: tcl_compiler::command_binding::ModuleCommandMutations::default()
                .snapshot(),
        },
        ValueTransferContext::of(
            db,
            tcl_compiler::value_transfer::AnalysisContextKey::for_module(
                &unit.command_mutations,
                unit.ir_module.resolved_registry(),
            ),
            0,
        ),
    )
}

#[test]
// Implementation contract: naming.variable.registry-event-frame-identity
// docs/design/analysis/name-resolution-proofs/variable-registry-event-frame-identity.md
fn original_event_lattice_key_rejects_foreign_source_body_config_and_registry() {
    let source = "when HTTP_REQUEST {set local 1}";
    let db = TclDatabase::default();
    let config_key = lexer_cfg_key(&db, "f5-irules");
    let file = SourceFile::new(&db, source.to_owned(), "f5-irules".to_owned(), None);
    let unit = compilation_unit(&db, file, config_key, 0)
        .expect("zero overlay selects the retained stock F5 Registry");
    let qname = "::when::HTTP_REQUEST";
    let body_id = original_test_body_id(&unit, 0);
    let event = unit.ir_module.irules_event_bodies.get(&body_id).unwrap();
    let procedure = unit.ir_module.procedures.get(&body_id).unwrap();
    assert_eq!(
        procedure.span.start(),
        0,
        "this actual declaration already owns the normalization origin"
    );
    let config = unit.ir_module.lexer_config.nested().normalized();
    let (context, snapshot) = original_event_test_context_snapshot(&db);
    let make_key = |body: Script, config, snapshot, event| {
        original_event_make_key(
            (&db, &unit, context, qname),
            (body, config, snapshot, event),
        )
    };
    let selected = make_key(
        procedure.body.clone(),
        config,
        snapshot.clone(),
        Some(Arc::clone(event)),
    );
    let plain = make_key(procedure.body.clone(), config, snapshot.clone(), None);
    assert!(
        selected != plain,
        "the same name and body cannot donate event entry"
    );
    // Implementation contract: naming.compiler.original-function-lattice-mutation-identity
    // docs/design/analysis/name-resolution-proofs/compiler-original-function-lattice-mutation-identity.md
    let changed_mutations = original_event_changed_mutations(&db, selected);
    assert!(
        selected != changed_mutations,
        "unchanged event/source/Registry/body cannot erase mutation obligations"
    );
    assert!(!function_lattice(&db, selected).complexity_guarded);
    assert!(function_lattice(&db, selected).irules_event_body.is_some());
    assert!(function_lattice(&db, plain).irules_event_body.is_none());
    let changed_file = SourceFile::new(
        &db,
        format!("{source}\n# changed source"),
        "f5-irules".to_owned(),
        None,
    );
    let changed_unit = compilation_unit(&db, changed_file, config_key, 0)
        .expect("zero overlay selects the retained stock F5 Registry");
    let changed_body_id = original_test_body_id(&changed_unit, 0);
    let foreign = make_key(
        procedure.body.clone(),
        config,
        snapshot.clone(),
        changed_unit
            .ir_module
            .irules_event_bodies
            .get(&changed_body_id)
            .cloned(),
    );
    assert!(
        selected != foreign,
        "complete original source participates in key equality"
    );
    assert!(function_lattice(&db, foreign).complexity_guarded);
    let wrong_body = make_key(
        Script::default(),
        config,
        snapshot.clone(),
        Some(Arc::clone(event)),
    );
    assert!(function_lattice(&db, wrong_body).complexity_guarded);
    let mut wrong_config = config;
    wrong_config.strict_quoting = !wrong_config.strict_quoting;
    let wrong_config_key = make_key(
        procedure.body.clone(),
        wrong_config,
        snapshot.clone(),
        Some(Arc::clone(event)),
    );
    assert!(function_lattice(&db, wrong_config_key).complexity_guarded);
    let wrong_registry = make_key(
        procedure.body.clone(),
        config,
        CompilerMemoSnapshot {
            profile: Some(tcl_lsp_core::profile_for_dialect("tcl8.6").cache_key()),
            registry: db.registry("tcl8.6").snapshot(),
        },
        Some(Arc::clone(event)),
    );
    assert!(function_lattice(&db, wrong_registry).complexity_guarded);
}

fn supplied_cfg_lattice_key<'db>(
    db: &'db dyn TclDb,
    module: &tcl_compiler::ir::Module,
    input: Option<tcl_compiler::analyser::ResolvedAnalysisInput>,
) -> FnLatticeKey<'db> {
    let registry = module.resolved_registry();
    let (upvars, params, globals, bindings, owners) =
        tcl_compiler::cfg_builder::prepare_cfg_context_with_registry(module, registry);
    assert!(upvars.is_empty() && params.is_empty() && globals.is_empty());
    let context = CfgContext::new(
        db,
        Vec::new(),
        Vec::new(),
        Vec::new(),
        BodyLatticeCommandBindings::from(bindings),
        owners,
    );
    FnLatticeKey::new(
        db,
        BodyLatticeInputs::from(module.top_level.clone()),
        "::subject".to_owned(),
        Vec::new(),
        context,
        module.lexer_config.normalized(),
        module.dialect.clone().unwrap(),
        CompilerMemoSnapshot {
            profile: module
                .dialect_profile
                .map(tcl_dialect::DialectProfile::cache_key),
            registry: registry.snapshot(),
        },
        Vec::new(),
        FnLatticeModuleFacts {
            known_classes: Vec::new(),
            traced_variables: Vec::new(),
            has_dynamic_variable_trace: false,
        },
        FnLatticeEntry {
            original_declaration_body: None,
            plain_command_dispatch: module.plain_command_dispatch,
            metadata_context:
                tcl_compiler::registry_invocation::OwnedInvocationMetadataContext::for_source_input(
                    input.as_ref(),
                ),
            source_metadata_input: input,
            irules_event_body: None,
            command_trust: tcl_compiler::command_binding::ModuleCommandMutations::default()
                .snapshot(),
        },
        ValueTransferContext::of(
            db,
            tcl_compiler::value_transfer::AnalysisContextKey::for_module(
                &tcl_compiler::command_binding::ModuleCommandMutations::default(),
                registry,
            ),
            0,
        ),
    )
}

fn assert_supplied_function_lattice_currency<'db>(
    db: &'db dyn TclDb,
    module: &tcl_compiler::ir::Module,
    generation: &Arc<tcl_registry::model::assembly::ContextRegistry>,
    entry: &tcl_compiler::command_binding::SourceAnalysisEntry,
    unknown: FnLatticeKey<'db>,
) -> (Arc<FunctionUnit>, Option<tcl_compiler::cfg::Terminator>) {
    let input = module.source_metadata_input.as_ref().unwrap();
    assert!(Arc::ptr_eq(&input.context_registry(), generation));
    let selected = supplied_cfg_lattice_key(db, module, Some(input.clone()));
    let older = Arc::new(
        tcl_registry::model::ingress::static_context_for("tcl8.4")
            .with_command_store(Arc::clone(generation.commands())),
    );
    assert!(Arc::ptr_eq(older.commands(), generation.commands()));
    let unavailable = supplied_cfg_lattice_key(
        db,
        module,
        Some(tcl_compiler::analyser::ResolvedAnalysisInput::new(
            input.analyser_profile(),
            input.unit_profile(),
            older,
            input.lexer_config(),
        )),
    );
    let foreign = supplied_cfg_lattice_key(
        db,
        module,
        Some(tcl_compiler::analyser::ResolvedAnalysisInput::new(
            input.analyser_profile(),
            input.unit_profile(),
            tcl_registry::model::ingress::context_for_profile(
                tcl_registry::model::ingress::resolve_environment("tcl9.1").analyser_profile(),
            ),
            input.lexer_config(),
        )),
    );
    let missing = supplied_cfg_lattice_key(db, module, None);
    let expected = Some(tcl_compiler::cfg::Terminator::Complete {
        route: tcl_registry::completion_route::InvocationCompletionRoute::Tcl(
            tcl_registry::CompletionCode::Error,
        ),
        span: Some(module.top_level.statements[0].span()),
    });
    let current = function_lattice(db, selected);
    assert_eq!(current.cfg.blocks[&current.cfg.entry].terminator, expected);
    assert_eq!(current.cfg.blocks[&current.cfg.entry].statements.len(), 1);
    for negative in [unavailable, foreign, missing, unknown] {
        assert!(
            selected != negative,
            "complete source input owns memo identity"
        );
        let conservative = function_lattice(db, negative);
        let entry = &conservative.cfg.blocks[&conservative.cfg.entry];
        assert_ne!(entry.terminator, expected);
        assert_eq!(
            entry.statements.len(),
            2,
            "withheld completion keeps the following call"
        );
    }
    assert!(Arc::ptr_eq(&current, &function_lattice(db, selected)));
    assert!(
        entry
            .native_entry
            .as_ref()
            .unwrap()
            .compiler_pass_environment
            .as_ref()
            .unwrap()
            .is_current_for(entry.native_entry.as_ref().unwrap().interpreter)
    );
    (current, expected)
}

fn supplied_lattice_test_input(
    generation: &Arc<tcl_registry::model::assembly::ContextRegistry>,
    config: tcl_lexer::LexerConfig,
) -> tcl_compiler::analyser::ResolvedAnalysisInput {
    tcl_compiler::analyser::ResolvedAnalysisInput::new(
        tcl_dialect::DialectProfile::find("tcl8.6").unwrap(),
        tcl_dialect::DialectProfile::find("tcl8.6").unwrap(),
        Arc::clone(generation),
        config,
    )
}

#[test]
fn function_lattice_retains_supplied_availability_and_refuses_missing_or_foreign_input() {
    // naming.compiler.original-analysis-metadata-context
    // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
    // Conditional source CFG completion only; no Native execution or handler grant.
    struct Capture(
        std::rc::Rc<std::cell::RefCell<Option<tcl_runtime_api::NativeCompilationEntry>>>,
    );
    impl tcl_vm::CompileService for Capture {
        type Module = <tcl_codegen::compile_service::BytecodeCompileService as tcl_vm::CompileService>::Module;

        fn compile(&self, _: &str) -> Result<Self::Module, tcl_vm::CompileError> {
            Err(tcl_vm::CompileError::Unsupported(
                "entry capture only".into(),
            ))
        }

        fn compile_script_with_entry(
            &self,
            _: tcl_runtime_api::ScriptCompileTarget<'_>,
            _: &'static tcl_dialect::DialectProfile,
            entry: &tcl_runtime_api::NativeCompilationEntry,
        ) -> Result<Self::Module, tcl_vm::CompileError> {
            *self.0.borrow_mut() = Some(entry.clone());
            self.compile("")
        }
    }
    let db = TclDatabase::default();
    let generation = tcl_registry::model::ingress::context_for_profile(
        tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile(),
    );
    let config = tcl_lexer::LexerConfig::for_dialect("tcl8.6");
    let supplied = supplied_lattice_test_input(&generation, config);
    let mut lowerer = tcl_compiler::lowering::Lowerer::with_config(generation.commands(), config)
        .with_resolved_analysis_input(supplied.clone());
    // Without an entry the document runs in a fresh interpreter; unknown
    // history must be supplied explicitly.
    lowerer.set_source_analysis_options(
        tcl_compiler::command_binding::SourceAnalysisEntry {
            metadata_context:
                tcl_compiler::registry_invocation::OwnedInvocationMetadataContext::for_source_input(
                    Some(&supplied),
                ),
            unknown_entry: true,
            ..tcl_compiler::command_binding::SourceAnalysisEntry::default()
        }
        .options(),
    );
    let source = "throw ERROR payload; set reached 1";
    let unknown_module = lowerer.lower(source).clone();
    assert!(unknown_module.source_entry.unknown_entry);
    assert!(unknown_module.source_entry.native_entry.is_none());
    let input = unknown_module.source_metadata_input.as_ref().unwrap();
    let unknown = supplied_cfg_lattice_key(&db, &unknown_module, Some(input.clone()));

    let captured = std::rc::Rc::new(std::cell::RefCell::new(None));
    let profile = tcl_dialect::DialectProfile::find("tcl8.6").unwrap();
    let mut owner = tcl_vm::Vm::with_native_core(
        Box::new(std::io::sink()),
        std::rc::Rc::new(tcl_vm::host_native::NativeHost::new()),
        profile,
        tcl_registry::special_vars::NativeBootstrapInputs::default(),
    )
    .unwrap();
    owner.set_compiler(Box::new(Capture(std::rc::Rc::clone(&captured))));
    assert!(owner.try_compile_function("set entry_probe 1").is_err());
    let native = captured
        .borrow_mut()
        .take()
        .expect("actual VM compiler entry");
    assert_eq!(
        native.execution_point,
        tcl_registry::model::ingress::resolve_environment("tcl8.6").point()
    );
    assert!(
        native
            .compiler_pass_environment
            .as_ref()
            .unwrap()
            .is_current_for(native.interpreter)
    );
    let entry = tcl_compiler::command_binding::SourceAnalysisEntry {
        metadata_context:
            tcl_compiler::registry_invocation::OwnedInvocationMetadataContext::for_source_input(
                Some(input),
            ),
        invocation_dialect: Some(tcl_registry::InvocationDialect::of_profile(profile)),
        native_compilation: tcl_registry::native_compilation::NativeCompilationContext {
            mode: tcl_registry::native_compilation::NativeCompilationMode::Direct,
            frame: tcl_registry::native_compilation::NativeCompilationFrame::ScriptCode,
            loop_depth: 0,
            catch_depth: Some(0),
        },
        native_entry: Some(Arc::new(native)),
        ..tcl_compiler::command_binding::SourceAnalysisEntry::default()
    };
    let mut lowerer = tcl_compiler::lowering::Lowerer::with_config(generation.commands(), config)
        .with_resolved_analysis_input(input.clone());
    lowerer.set_source_analysis_options(entry.options());
    let module = lowerer.lower(source).clone();
    assert!(!module.source_entry.unknown_entry);
    assert!(module.source_entry.native_entry.is_some());
    assert_eq!(module.source_metadata_input.as_ref(), Some(input));
    let _retained_currency =
        assert_supplied_function_lattice_currency(&db, &module, &generation, &entry, unknown);
    drop(owner);
}

fn semantic_bundle_has_source_metadata(
    unit: &tcl_compiler::compilation_unit::FunctionUnit,
) -> bool {
    unit.semantic_facts
        .conditional_source_metadata()
        .invocations()
        .next()
        .is_some()
}

#[test]
fn function_lattice_semantic_bundle_uses_retained_availability() {
    // naming.compiler.original-analysis-metadata-context
    // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
    let db = TclDatabase::default();
    let baseline = tcl_registry::model::ingress::static_context_for("tcl8.6");
    let mut registry = baseline
        .commands()
        .project_for_profile(tcl_dialect::DialectProfile::find("tcl8.6").unwrap());
    let mut puts = registry.get("puts").unwrap().clone();
    puts.surface = Some(tcl_dialect::model::SpecSurface::TCL86_PLUS);
    registry.insert(puts);
    let registry = registry
        .project_for_profile(tcl_registry::model::resolve_environment("tcl8.6").unit_profile());
    let current = Arc::new(baseline.with_command_store(Arc::new(registry)));
    assert!(
        current
            .context()
            .resolve_spec(current.commands(), "puts")
            .is_some()
    );
    let config = tcl_lexer::LexerConfig::for_dialect("tcl8.6");
    let mut lowerer = tcl_compiler::lowering::Lowerer::with_config(current.commands(), config)
        .with_resolved_analysis_input(tcl_compiler::analyser::ResolvedAnalysisInput::new(
            tcl_dialect::DialectProfile::find("tcl8.6").unwrap(),
            tcl_dialect::DialectProfile::find("tcl8.6").unwrap(),
            Arc::clone(&current),
            config,
        ));
    let module = lowerer.lower("puts VALUE").clone();
    let input = module.source_metadata_input.as_ref().unwrap();
    let selected = supplied_cfg_lattice_key(&db, &module, Some(input.clone()));
    assert!(semantic_bundle_has_source_metadata(&function_lattice(
        &db, selected
    )));
    let older = Arc::new(
        tcl_registry::model::ingress::static_context_for("tcl8.4")
            .with_command_store(Arc::clone(current.commands())),
    );
    assert!(Arc::ptr_eq(older.commands(), current.commands()));
    assert!(
        older
            .context()
            .resolve_spec(older.commands(), "puts")
            .is_none()
    );
    let unavailable = supplied_cfg_lattice_key(
        &db,
        &module,
        Some(tcl_compiler::analyser::ResolvedAnalysisInput::new(
            input.analyser_profile(),
            input.unit_profile(),
            older,
            input.lexer_config(),
        )),
    );
    assert!(selected != unavailable);
    assert!(!semantic_bundle_has_source_metadata(&function_lattice(
        &db,
        unavailable
    )));
    let foreign = supplied_cfg_lattice_key(
        &db,
        &module,
        Some(tcl_compiler::analyser::ResolvedAnalysisInput::new(
            input.analyser_profile(),
            input.unit_profile(),
            tcl_registry::model::ingress::resolve_environment("tcl9.1").default_context_registry(),
            input.lexer_config(),
        )),
    );
    let missing = supplied_cfg_lattice_key(&db, &module, None);
    for negative in [foreign, missing] {
        assert!(selected != negative);
        assert!(matches!(
            function_lattice(&db, negative).semantic_facts.executable(),
            tcl_compiler::semantic_analysis::ExecutableAnalysisAvailability::ContextUnavailable
        ));
    }
}

#[test]
fn function_lattice_key_keeps_the_exact_normalized_lexer_config() {
    let db = TclDatabase::default();
    let context = CfgContext::new(
        &db,
        Vec::new(),
        Vec::new(),
        Vec::new(),
        BodyLatticeCommandBindings::from(ModuleCommandBindings::default()),
        OriginalProcedureSummaryOwners::default(),
    );
    let make_key = |config: tcl_lexer::LexerConfig| {
        FnLatticeKey::new(
            &db,
            BodyLatticeInputs::from(Script::default()),
            "::subject".to_owned(),
            Vec::new(),
            context,
            config.normalized(),
            "tcl8.6".to_owned(),
            CompilerMemoSnapshot {
                profile: Some(tcl_lsp_core::profile_for_dialect("tcl8.6").cache_key()),
                registry: db.registry("tcl8.6").snapshot(),
            },
            Vec::new(),
            FnLatticeModuleFacts {
                known_classes: Vec::new(),
                traced_variables: Vec::new(),
                has_dynamic_variable_trace: false,
            },
            FnLatticeEntry {
                original_declaration_body: None,
                plain_command_dispatch: false,
                metadata_context:
                    tcl_compiler::registry_invocation::OwnedInvocationMetadataContext::Unavailable,
                source_metadata_input: None,
                irules_event_body: None,
                command_trust: tcl_compiler::command_binding::ModuleCommandMutations::default()
                    .snapshot(),
            },
            ValueTransferContext::of(
                &db,
                tcl_compiler::value_transfer::AnalysisContextKey::detached(),
                0,
            ),
        )
    };

    let tcl_key = make_key(tcl_lexer::LexerConfig::for_dialect("tcl8.6"));
    let irules_key = make_key(tcl_lexer::LexerConfig::for_dialect("f5-irules"));
    let jim_config = tcl_lexer::LexerConfig::for_dialect("jim");
    let jim_key = make_key(jim_config);
    assert!(
        tcl_key != irules_key,
        "the iRules ghost-separator axis is part of memo identity"
    );
    assert!(
        tcl_key != jim_key,
        "Jim's vertical-tab word-separator axis is part of memo identity"
    );

    let shifted_jim_key = make_key(tcl_lexer::LexerConfig {
        base_offset: 91,
        base_line: 7,
        base_col: 13,
        ..jim_config
    });
    assert!(
        jim_key == shifted_jim_key,
        "position-only relocation must normalize out of memo identity"
    );

    let _cold_other_config = function_lattice(&db, tcl_key);
    let jim_cold = function_lattice(&db, jim_key);
    let jim_warm = function_lattice(&db, jim_key);
    assert!(
        Arc::ptr_eq(&jim_cold, &jim_warm),
        "the warm exact-config demand must reuse the cold lattice"
    );
}

/// An unchanged proc body must re-intern to the *same*
/// `ItemBodyKey` (so the memo hits) and must not cost a fresh copy of the
/// body text to get there — the key shares the analyser's `Arc<str>`.
#[test]
fn unchanged_body_reinterns_to_the_same_key_without_copying_its_text() {
    let db = TclDatabase::default();
    let body: Arc<str> = Arc::from("set s 1\nreturn $s\n");
    let make = || {
        ItemBodyKey::new(
            &db,
            Arc::clone(&body),
            "::".to_owned(),
            "alpha".to_owned(),
            Vec::new(),
            false,
            false,
            Vec::new(),
            None,
            ((Vec::new(), Vec::new()), None, None, None, None),
            "tcl8.6".to_owned(),
            Vec::new(),
            NonAsciiMode::Default,
        )
    };
    assert!(make() == make(), "unchanged text must re-intern to one key");
    assert!(
        Arc::ptr_eq(make().body_text(&db), &body),
        "the interned key must share the caller's body text, not copy it",
    );
    // A different body must not collide with it.
    let other: Arc<str> = Arc::from("set s 2\nreturn $s\n");
    let other_key = ItemBodyKey::new(
        &db,
        other,
        "::".to_owned(),
        "alpha".to_owned(),
        Vec::new(),
        false,
        false,
        Vec::new(),
        None,
        ((Vec::new(), Vec::new()), None, None, None, None),
        "tcl8.6".to_owned(),
        Vec::new(),
        NonAsciiMode::Default,
    );
    assert!(
        make() != other_key,
        "distinct bodies must intern distinctly"
    );
}

#[test]
fn document_symbols_match_direct() {
    let db = TclDatabase::default();
    let file = SourceFile::new(&db, SRC.to_owned(), "tcl".to_owned(), None);
    let got = document_symbols(&db, file, cfg(&db));
    let expected = tcl_lsp_core::document_symbols::document_symbols(
        SRC,
        tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
    );
    assert_eq!(got, expected);
}

#[test]
fn semantic_tokens_match_direct() {
    let db = TclDatabase::default();
    let file = SourceFile::new(&db, SRC.to_owned(), "tcl".to_owned(), None);
    let got = semantic_tokens(&db, file, cfg(&db));
    let reg = db.registry("tcl");
    let expected = tcl_lsp_core::semantic_tokens::full(
        SRC,
        tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
        reg,
    );
    assert_eq!(got, expected);
    assert_ne!(got.data.len(), 0);
}

/// TP: the enriched `semantic_tokens` query (backed by a
/// [`CompilationUnit`] and SSA/SCCP-derived constant-string facts) retags
/// the `.*abc` literal at its originating `set` as regex source, because
/// `regexp $my_re` provably reads that constant — see
/// [`tcl_compiler::regex_source`]. The cheap coarse tier
/// (`tcl_lsp_core::semantic_tokens::full`, no `CompilationUnit`) has no
/// SSA facts to do this with, so the two streams differ — proving the
/// enrichment `semantic_tokens` performs over the coarse walk is real,
/// not a no-op: the fast-path fallback in
/// `Backend::semantic_tokens_core_data` trades this enrichment away,
/// so it must exist for the trade to mean anything.
#[test]
fn semantic_tokens_retags_constant_regex_source_true_positive() {
    let src = "set my_re \".*abc\"\nregexp $my_re $s\n";
    let db = TclDatabase::default();
    let file = SourceFile::new(&db, src.to_owned(), "tcl9.0".to_owned(), None);
    let enriched = semantic_tokens(&db, file, cfg(&db));
    let reg = db.registry("tcl9.0");
    let coarse = tcl_lsp_core::semantic_tokens::full(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl9.0").analyser_profile(),
        reg,
    );
    assert_ne!(
        enriched, coarse,
        "the CompilationUnit-informed regex-source retag must change the \
             token stream relative to the coarse (no-CU) tier"
    );
}

/// TN: a `regexp` pattern read from a variable that is *not* provably
/// constant (reassigned from a proc parameter) gets no retag from either
/// tier — the coarse and enriched streams agree, because there is no
/// constant fact for the enriched tier to add. Guards against the retag
/// firing indiscriminately on every `regexp $var` call.
#[test]
fn semantic_tokens_skips_retag_for_non_constant_pattern_true_negative() {
    let src = "proc match {re s} {\n    regexp $re $s\n}\n";
    let db = TclDatabase::default();
    let file = SourceFile::new(&db, src.to_owned(), "tcl9.0".to_owned(), None);
    let enriched = semantic_tokens(&db, file, cfg(&db));
    let reg = db.registry("tcl9.0");
    let coarse = tcl_lsp_core::semantic_tokens::full(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl9.0").analyser_profile(),
        reg,
    );
    assert_eq!(
        enriched, coarse,
        "a non-constant pattern source must not be retagged by either tier"
    );
}

/// `semantic_tokens` must depend on
/// the incremental, per-item-memoised `file_analysis_incremental` — not
/// the coarse `file_analysis` — so a token request that lands after the
/// diagnostics worker has already analysed this revision is a cache hit
/// (no second whole-file walk), and a token request that lands *first*
/// still primes the exact query diagnostics will reuse. Also asserts the
/// coarse, uncancellable `file_analysis` is never invoked by either order.
#[test]
fn semantic_tokens_shares_incremental_analysis_with_diagnostics() {
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
    let mut db = TclDatabase {
        storage: salsa::Storage::new(Some(Box::new(sink))),
    };
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
    let file = SourceFile::new(&db, SRC.to_owned(), "tcl8.6".to_owned(), None);

    // Diagnostics-first order: the worker analyses, then a token request
    // arrives for the same revision.
    let _ = file_analysis_incremental(&db, file, cfg);
    let after_diagnostics = std::mem::take(&mut *log.lock().unwrap());
    assert_eq!(
        after_diagnostics
            .iter()
            .filter(|s| s.contains("item_body_analysis"))
            .count(),
        1,
        "diagnostics analyses the one proc body: {after_diagnostics:?}"
    );

    let _ = semantic_tokens(&db, file, cfg);
    let after_tokens = std::mem::take(&mut *log.lock().unwrap());
    assert!(
        after_tokens
            .iter()
            .all(|s| !s.contains("item_body_analysis")),
        "a token request for the same revision must be a cache hit against \
             the diagnostics worker's analysis, not a second walk: {after_tokens:?}"
    );
    assert!(
        after_tokens.iter().all(|s| !s.contains("file_analysis(")),
        "semantic_tokens must never invoke the coarse, uncancellable \
             file_analysis query: {after_tokens:?}"
    );

    // Token-first order (a cold open before the debounced diagnostics
    // worker has run): editing then re-requesting tokens still only
    // recomputes the one changed body, proving the per-item firewall
    // covers the token path too, not just the diagnostics path.
    file.set_text(&mut db)
        .to("proc greet {name} {\n    puts \"hi $name!!\"\n}\n# c\nset x 1\n".to_owned());
    let _ = std::mem::take(&mut *log.lock().unwrap());
    let _ = semantic_tokens(&db, file, cfg);
    let after_edit = std::mem::take(&mut *log.lock().unwrap());
    assert_eq!(
        after_edit
            .iter()
            .filter(|s| s.contains("item_body_analysis"))
            .count(),
        1,
        "a single-body edit must recompute exactly one item via the token \
             path: {after_edit:?}"
    );
}

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
        let lib = SourceFile::new(&db, LIB.to_owned(), "tcl8.6".to_owned(), None);
        let main = SourceFile::new(
            &db,
            "proc other {x} { return $x }\nother 1\nhelper dev\n".to_owned(),
            "tcl8.6".to_owned(),
            None,
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

/// Build a database whose executed-query keys land in the returned log.
fn logging_db() -> (TclDatabase, Arc<Mutex<Vec<String>>>) {
    let log: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(Vec::new()));
    let sink = {
        let l = Arc::clone(&log);
        move |ev: salsa::Event| {
            if let salsa::EventKind::WillExecute { database_key } = ev.kind {
                l.lock().unwrap().push(format!("{database_key:?}"));
            }
        }
    };
    let db = TclDatabase {
        storage: salsa::Storage::new(Some(Box::new(sink))),
    };
    (db, log)
}

/// [`project_class_index`] / [`project_proc_var_index`] read
/// *every* file in the project, so the tier they read decides what an
/// interactive `semanticTokens` request costs on a large workspace. They
/// must stay on the light [`file_token_facts`] tier and never reach the deep
/// one — reading `file_analysis_incremental` there would mean a
/// whole-workspace deep analysis (CFG/SSA units, per-body lattices,
/// diagnostic emitters) behind a token request for one file, which on an
/// 883-file checkout would never complete before the next `set_text`
/// cancelled it.
#[test]
fn project_indexes_never_touch_the_deep_tier() {
    let (db, log) = logging_db();
    let lib = SourceFile::new(
        &db,
        "oo::configurable create ::Pin { property node }\n".to_owned(),
        "tcl8.6".to_owned(),
        None,
    );
    let main = SourceFile::new(
        &db,
        "set p [::Pin new]\n$p configure -node n1\n".to_owned(),
        "tcl8.6".to_owned(),
        None,
    );
    let project = Project::new(&db, vec![lib, main]);

    let _ = project_class_index(&db, project);
    let _ = project_proc_var_index(&db, project);
    let log_snapshot = log.lock().unwrap().clone();
    assert!(
        log_snapshot.iter().any(|s| s.contains("file_token_facts")),
        "project indexes must read the light per-file tier: {log_snapshot:?}"
    );
    assert!(
        log_snapshot.iter().all(|s| !s.contains("file_analysis")),
        "project indexes must never analyse a project file at the deep tier \
             (#1163): {log_snapshot:?}"
    );
}

/// The per-file [`file_token_facts`] firewall: a body edit that changes no
/// class and no parameter role backdates, so neither project index
/// re-executes — the property that keeps the cross-file token aggregates off
/// the per-keystroke path once they are warm.
#[test]
fn project_indexes_backdate_on_a_role_neutral_body_edit() {
    use salsa::Setter as _;
    let (mut db, log) = logging_db();
    let lib = SourceFile::new(
        &db,
        "oo::configurable create ::Pin { property node }\n".to_owned(),
        "tcl8.6".to_owned(),
        None,
    );
    let main = SourceFile::new(
        &db,
        "proc ::helper {a b} { return [expr {$a + $b}] }\n".to_owned(),
        "tcl8.6".to_owned(),
        None,
    );
    let project = Project::new(&db, vec![lib, main]);
    let _ = project_class_index(&db, project);
    let _ = project_proc_var_index(&db, project);

    log.lock().unwrap().clear();
    main.set_text(&mut db)
        .to("proc ::helper {a b} { return [expr {$a * $b}] }\n".to_owned());
    let _ = project_class_index(&db, project);
    let _ = project_proc_var_index(&db, project);
    let after = log.lock().unwrap().clone();
    assert!(
        after.iter().any(|s| s.contains("file_token_facts")),
        "the edited file's own facts must be recomputed: {after:?}"
    );
    assert!(
        after
            .iter()
            .all(|s| !s.contains("project_class_index") && !s.contains("project_proc_var_index")),
        "a role-neutral body edit must backdate at the per-file firewall, \
             leaving the project aggregates memoised: {after:?}"
    );
}

/// The merged project role index must equal one built from every file's
/// procs at once — including the abstentions each file made on its own, so
/// a name one file already dropped as ambiguous is not re-adopted from
/// another file's unambiguous entry.
#[test]
fn merged_role_index_equals_the_all_at_once_build() {
    let sources = [
        // `::a::grow` and `::b::grow` disagree, so this file abstains on the
        // bare `grow` key all by itself.
        "namespace eval a { proc grow {v} { upvar 1 $v x; set x 1 } }\n\
             namespace eval b { proc grow {n v} { upvar 1 $v x; set x 1 } }\n",
        // A third `grow`, unambiguous within its own file.
        "namespace eval c { proc grow {v} { upvar 1 $v x; set x 2 } }\n",
    ];
    let db = TclDatabase::default();
    let files: Vec<SourceFile> = sources
        .iter()
        .map(|s| SourceFile::new(&db, (*s).to_owned(), "tcl8.6".to_owned(), None))
        .collect();
    let project = Project::new(&db, files.clone());

    let all_procs: Vec<Arc<AnalysisResult>> = sources
        .iter()
        .map(|s| Arc::new(Analyser::new().structure_only().analyse(s, "tcl8.6")))
        .collect();
    let expected = VarNameArgRoles::from_procs(all_procs.iter().flat_map(|a| a.all_procs.values()));
    assert_eq!(*project_proc_var_index(&db, project), expected);
    // Order-independent: reversing the project's files changes nothing.
    let reversed = Project::new(&db, files.into_iter().rev().collect());
    assert_eq!(*project_proc_var_index(&db, reversed), expected);
}

#[test]
fn cross_file_object_dispatch_resolves_via_project_index() {
    // A class defined in one file, dispatched on via a direct constructor in
    // another: `semantic_tokens_project` resolves the method through the
    // workspace-merged `project_class_index`, so its output differs from the
    // local-only `semantic_tokens` (which leaves the method a plain string).
    let db = TclDatabase::default();
    let lib = SourceFile::new(
        &db,
        "oo::configurable create ::Pin { property node }\n".to_owned(),
        "tcl9.0".to_owned(),
        None,
    );
    let user = SourceFile::new(
        &db,
        "[::Pin new] configure -node 5\n".to_owned(),
        "tcl9.0".to_owned(),
        None,
    );
    let project = Project::new(&db, vec![lib, user]);
    let cross = semantic_tokens_project(&db, user, project, cfg(&db));
    let local = semantic_tokens(&db, user, cfg(&db));
    assert_ne!(
        cross.data, local.data,
        "cross-file index must resolve the method the local pass leaves unresolved"
    );
    // The merged index sees the class from the other file.
    assert!(
        project_class_index(&db, project)
            .classes
            .contains_key("::Pin"),
        "project index should contain ::Pin from the library file"
    );
}

#[test]
fn project_index_abstains_on_duplicate_class_name() {
    // The same qualified class name defined in two files is genuinely
    // ambiguous — which `::Pin` does a cross-file dispatch mean?  The merged
    // index drops it (sound-by-abstention), deterministically regardless of
    // file order, rather than letting an order-dependent "winner" leak into
    // resolution.
    let db = TclDatabase::default();
    let a = SourceFile::new(
        &db,
        "oo::class create ::Pin { method a {} {} }\n".to_owned(),
        "tcl9.0".to_owned(),
        None,
    );
    let b = SourceFile::new(
        &db,
        "oo::class create ::Pin { method b {} {} }\n".to_owned(),
        "tcl9.0".to_owned(),
        None,
    );
    // Both orderings must agree (and both must drop the ambiguous class).
    for files in [vec![a, b], vec![b, a]] {
        let project = Project::new(&db, files);
        assert!(
            !project_class_index(&db, project)
                .classes
                .contains_key("::Pin"),
            "an ambiguous cross-file class name must be dropped, not resolved"
        );
    }
}

#[test]
fn folding_matches_direct() {
    let db = TclDatabase::default();
    let file = SourceFile::new(&db, SRC.to_owned(), "tcl".to_owned(), None);
    let got = folding_ranges(&db, file, cfg(&db));
    let reg = db.registry("tcl");
    let expected = tcl_lsp_core::folding::folding_ranges(
        SRC,
        tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
        reg,
    );
    assert_eq!(got, expected);
}

/// Folding served through salsa is memoised: a repeat request for the
/// same revision must not re-execute the tracked query — the property
/// `Backend::db_folding_ranges` relies on to avoid a fresh
/// segmenter/registry walk on every `textDocument/foldingRange`.
#[test]
fn folding_ranges_memoised_across_repeat_queries() {
    let log: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(Vec::new()));
    let db = {
        let sink = Arc::clone(&log);
        TclDatabase::with_event_logger(move |key| sink.lock().unwrap().push(key))
    };
    let file = SourceFile::new(&db, SRC.to_owned(), "tcl".to_owned(), None);
    // `AnalyserConfig` is an input: one handle, so the repeat request
    // names the same query key.
    let config = cfg(&db);
    let drain = || std::mem::take(&mut *log.lock().unwrap());
    let executions = |events: &[String]| {
        events
            .iter()
            .filter(|key| key.contains("folding_ranges"))
            .count()
    };

    let _ = folding_ranges(&db, file, config);
    assert_eq!(
        executions(&drain()),
        1,
        "cold: one execution for the first request"
    );
    let _ = folding_ranges(&db, file, config);
    assert_eq!(
        executions(&drain()),
        0,
        "warm: memoised, no re-execution for a repeat request at the same revision"
    );
}

#[test]
fn editing_text_recomputes() {
    use salsa::Setter as _;
    let mut db = TclDatabase::default();
    let config = cfg(&db);
    let file = SourceFile::new(&db, "proc a {} {}\n".to_owned(), "tcl".to_owned(), None);
    assert!(
        file_analysis(&db, file, config)
            .all_procs
            .contains_key("::a")
    );

    file.set_text(&mut db).to("proc b {} {}\n".to_owned());
    let after = file_analysis(&db, file, config);
    assert!(after.all_procs.contains_key("::b"));
    assert!(!after.all_procs.contains_key("::a"));
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

#[test]
fn file_analysis_incremental_matches_full() {
    let db = TclDatabase::default();
    let cfg = cfg(&db);
    for src in [
        SRC,
        "proc a {x} { return $x }\nproc b {} { a 1 }\n",
        "namespace eval n { proc f {y} { set z $y } }\nset g 1\nputs $g\n",
        "oo::class create K {\n  method m {a} { set n $a }\n}\nproc p {} { set q 1 }\n",
    ] {
        let file = SourceFile::new(&db, src.to_owned(), "tcl8.6".to_owned(), None);
        let inc = file_analysis_incremental(&db, file, cfg);
        let full = file_analysis(&db, file, cfg);
        assert_eq!(*inc, *full, "incremental != full for:\n{src}");
    }
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

#[test]
fn sidecar_stubs_reach_incremental_proc_bodies_and_invalidate() {
    use salsa::Setter as _;

    let root = std::env::temp_dir().join(format!(
        "tcl-lsp-sidecar-incremental-{}-{}",
        std::process::id(),
        std::thread::current().name().unwrap_or("test")
    ));
    let src_dir = root.join("src");
    std::fs::create_dir_all(&src_dir).unwrap();
    let sidecar = root.join("tcl8.6.tcl.stubs");
    std::fs::write(&sidecar, "stub sidecar_cmd {value}\n").unwrap();
    let path = src_dir.join("main.tcl");
    let mut db = TclDatabase::default();
    let config = cfg(&db);
    let file = SourceFile::new(
        &db,
        "proc f {} { sidecar_cmd 1 }\n".to_owned(),
        "tcl8.6".to_owned(),
        Some(path.display().to_string()),
    );

    let initial = file_analysis_incremental(&db, file, config);
    assert!(
        !initial.diagnostics.iter().any(|d| d.code == DiagCode::W123),
        "sidecar command must resolve inside a proc body: {:?}",
        initial.diagnostics
    );
    assert_eq!(*initial, *file_analysis(&db, file, config));

    std::fs::write(&sidecar, "stub replacement_cmd {value}\n").unwrap();
    file.set_sidecar_stubs_epoch(&mut db).to(1);
    let changed = file_analysis_incremental(&db, file, config);
    assert!(
        changed.diagnostics.iter().any(|d| d.code == DiagCode::W123),
        "the unchanged source must be reanalysed after its sidecar changes"
    );
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn file_analysis_incremental_carries_bigip_version_override() {
    let db = TclDatabase::default();
    let cfg = AnalyserConfig::new(
        &db,
        Vec::new(),
        NonAsciiMode::Default,
        Vec::new(),
        None,
        Some("21.1.0".to_owned()),
        0,
        Vec::new(),
        Vec::new(),
    );
    let file = SourceFile::new(
        &db,
        "SSL::c3d cert_lifespan 24\n".to_owned(),
        "f5-irules".to_owned(),
        None,
    );

    let incremental = file_analysis_incremental(&db, file, cfg);
    let full = file_analysis(&db, file, cfg);

    assert_eq!(*incremental, *full);
    assert_eq!(
        incremental.library_versions.bigip_version.as_deref(),
        Some("21.1.0")
    );
}

/// `tclLsp.targets` (§5.4 range targeting) flows through the config
/// into both analysis queries: a declared `tcl 8.5-9.0` range flags
/// the 8.6-introduced `lmap` at the declared 8.5 target, and the
/// undeclared default stays byte-identically silent.
#[test]
fn file_analysis_carries_declared_targets() {
    let db = TclDatabase::default();
    let ranged = AnalyserConfig::new(
        &db,
        Vec::new(),
        NonAsciiMode::Default,
        Vec::new(),
        None,
        None,
        0,
        vec![("tcl".to_owned(), "8.5-9.0".to_owned())],
        Vec::new(),
    );
    let bare = AnalyserConfig::new(
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
    let file = SourceFile::new(
        &db,
        "lmap x {1 2} {set x}\n".to_owned(),
        "tcl8.6".to_owned(),
        None,
    );

    let incremental = file_analysis_incremental(&db, file, ranged);
    let full = file_analysis(&db, file, ranged);
    assert_eq!(*incremental, *full);
    assert!(
        incremental
            .diagnostics
            .iter()
            .any(|d| d.code.as_str() == "W150" && d.message.contains("8.5")),
        "the declared range flags lmap at the 8.5 target: {:?}",
        incremental.diagnostics
    );
    let undeclared = file_analysis_incremental(&db, file, bare);
    assert!(
        !undeclared
            .diagnostics
            .iter()
            .any(|d| matches!(d.code.as_str(), "W150" | "W151")),
        "no declaration, no range diagnostics"
    );
}

/// A body-only edit (same length, so other bodies keep their offset)
/// recomputes exactly one `item_body_analysis`.
#[test]
fn body_edit_recomputes_one_item() {
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
    let mut db = TclDatabase {
        storage: salsa::Storage::new(Some(Box::new(sink))),
    };
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
    let file = SourceFile::new(
        &db,
        "proc a {} { set x 11111 }\nproc b {} { set y 22222 }\n".to_owned(),
        "tcl8.6".to_owned(),
        None,
    );
    let _ = file_analysis_incremental(&db, file, cfg);
    let init = std::mem::take(&mut *log.lock().unwrap());
    assert_eq!(
        init.iter()
            .filter(|s| s.contains("item_body_analysis"))
            .count(),
        2,
        "initial: both bodies analysed: {init:?}"
    );

    // Edit proc a's body, *changing its length* — this shifts proc b's
    // byte offset.  Offset-invariance means b's key (its body text) is
    // unchanged, so it stays a cache hit and only a recomputes.
    file.set_text(&mut db)
        .to("proc a {} { set x 9999999999 }\nproc b {} { set y 22222 }\n".to_owned());
    let _ = file_analysis_incremental(&db, file, cfg);
    let after = std::mem::take(&mut *log.lock().unwrap());
    assert_eq!(
        after
            .iter()
            .filter(|s| s.contains("item_body_analysis"))
            .count(),
        1,
        "length-changing body edit -> exactly ONE item recomputes (offset-invariant): {after:?}"
    );
}

/// The method firewall: a body edit to one OO method recomputes exactly one
/// `item_body_analysis` — methods are isolated + memoised like procs, so an
/// unedited sibling method (shifted by the edit) stays a cache hit.
#[test]
fn method_body_edit_recomputes_one_item() {
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
    let mut db = TclDatabase {
        storage: salsa::Storage::new(Some(Box::new(sink))),
    };
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
    let file = SourceFile::new(
        &db,
        "oo::class create K {\n  method a {} { set x 11111 }\n  method b {} { set y 22222 }\n}\n"
            .to_owned(),
        "tcl8.6".to_owned(),
        None,
    );
    let _ = file_analysis_incremental(&db, file, cfg);
    let init = std::mem::take(&mut *log.lock().unwrap());
    assert_eq!(
        init.iter()
            .filter(|s| s.contains("item_body_analysis"))
            .count(),
        2,
        "initial: both method bodies analysed: {init:?}"
    );

    // Edit method a's body length — shifts method b; b's offset-0 body is
    // unchanged, so its key is a cache hit and only a recomputes.
    file.set_text(&mut db).to(
            "oo::class create K {\n  method a {} { set x 9999999999 }\n  method b {} { set y 22222 }\n}\n"
                .to_owned(),
        );
    let _ = file_analysis_incremental(&db, file, cfg);
    let after = std::mem::take(&mut *log.lock().unwrap());
    assert_eq!(
        after
            .iter()
            .filter(|s| s.contains("item_body_analysis"))
            .count(),
        1,
        "method body edit -> exactly ONE item recomputes: {after:?}"
    );
}

/// The salsa-native optimiser path must be byte-identical to a direct
/// (non-memoised) compiler-checks + optimiser build, over several dialects.
#[test]
fn compiler_check_diagnostics_matches_uncached() {
    let db = TclDatabase::default();
    for (src, dialect) in [
        ("proc a {x} { if {1} { set y 1 }\n return $y }\n", "tcl8.6"),
        ("set g 0\nproc inc {} { global g; incr g }\ninc\n", "tcl8.6"),
        (
            "proc f {n} { set acc 0\n for {set i 0} {$i < $n} {incr i} { set acc [expr {$acc + $i}] }\n return $acc }\n",
            "tcl9.0",
        ),
        ("when HTTP_REQUEST { set u [HTTP::uri] }\n", "f5-irules"),
    ] {
        let file = SourceFile::new(&db, src.to_owned(), dialect.to_owned(), None);
        let got = compiler_check_diagnostics(
            &db,
            file,
            AnalyserConfig::new(
                &db,
                Vec::new(),
                NonAsciiMode::Default,
                Vec::new(),
                None,
                None,
                0,
                Vec::new(),
                Vec::new(),
            ),
        );
        let registry = db.registry(dialect);
        let want = compiler_check_diagnostics_uncached(src, registry, dialect, None, None);
        assert_eq!(
            got.checks, want.checks,
            "checks differ for ({dialect}):\n{src}"
        );
        assert_eq!(
            got.optimisations, want.optimisations,
            "optimisations differ for ({dialect}):\n{src}"
        );
    }
}

/// A `TclOO` method body / `namespace eval` body / `apply` lambda is not a
/// procedure, so it never gets an offset-0 `FnLatticeKey` and is invisible to
/// [`proc_taint_solve`]'s main `analysable_functions` loop — it is reached
/// only by that query's explicit top-up over
/// `analysable_methods_and_body_units`.  That top-up must run the whole
/// `function_nontaint_checks` family, not just its `shimmer_family_checks`
/// half: running only the shimmer half would silently drop every SCCP
/// constant-branch (`O100`) and GVN full / partial / loop-invariant
/// (`O105`/`O106`) finding inside a method or a body unit from the
/// memoised path while the direct [`compiler_check_diagnostics_uncached`]
/// build reports it — 20 missing hints on one large `TclOO` corpus file.
///
/// The existing corpus differential could not see this: it sweeps the
/// procedural `tmp/tcl*/library` + `tcllib` trees, which define almost no
/// `TclOO` methods.  This fixture pins the shape directly, and asserts the
/// counts are **non-zero** so the equality cannot pass vacuously.
#[test]
fn compiler_check_memo_covers_method_and_body_unit_checks() {
    let dialect = "tcl8.6";
    // Each body carries an `O100` (SCCP-folded existence branch) *and* an
    // `O106` (loop-invariant `lindex`), once inside a `TclOO` method, once
    // inside a `destructor`, once inside an `apply` lambda, and once inside a
    // `namespace eval` body — the four function kinds outside
    // `analysable_functions`.
    // Absolute command heads keep the TclOO bodies eligible for lexical
    // analysis: their execution namespace is receiver-selected, so a
    // relative command may be shadowed at runtime and is deliberately
    // complexity-guarded by the compiler.
    let body = "::if {[::info exists Missing]} { ::puts no }\n\
                    ::for {::set i 0} {$i < [::llength $vals]} {::incr i 2} {\n\
                        ::set v [::lindex $vals [::expr {$i+1}]]\n\
                        ::puts $v\n\
                    }\n";
    let src = format!(
        "oo::class create K {{\n\
                 variable vals\n\
                 method m {{}} {{\n{body}}}\n\
                 destructor {{\n{body}}}\n\
             }}\n\
             apply {{{{vals}} {{\n{body}}}}} {{a b}}\n\
             namespace eval ns {{\n set vals {{a b}}\n{body}}}\n"
    );

    let db = TclDatabase::default();
    let file = SourceFile::new(&db, src.clone(), dialect.to_owned(), None);
    let got = compiler_check_diagnostics(&db, file, cfg(&db));
    let registry = db.registry(dialect);
    let want = compiler_check_diagnostics_uncached(&src, registry, dialect, None, None);

    let o1xx = |ds: &[CompilerCheck]| {
        ds.iter()
            .filter(|d| matches!(d.code, DiagCode::O100 | DiagCode::O105 | DiagCode::O106))
            .count()
    };
    assert!(
        o1xx(&want.checks) >= 4,
        "fixture must actually produce O1xx checks in the non-proc bodies, got {:?}",
        want.checks
    );
    assert_eq!(
        got.checks, want.checks,
        "memoised checks drop findings inside TclOO method / body-unit bodies (#1117)"
    );
    assert_eq!(
        o1xx(&got.checks),
        o1xx(&want.checks),
        "memoised O1xx hint count must equal the fresh build's"
    );
    assert_eq!(got.optimisations, want.optimisations);
}

/// `[info exists x]` where `x` is `TclOO` instance state
/// must not fold on **either** compiler-check path.
///
/// A class-level `variable x` binds `x` in every method frame with no
/// binding command in the body, so a naive existence fold would see a
/// never-defined local and produce an "always false" constant branch —
/// an `O100` hint (and its `I230` twin) on code that runs.  Oracle,
/// identical on tclsh 9.0.4 and 8.6.14: `oo::class create C { variable x;
/// constructor {} { set x 1 }; method m {} { puts [info exists x] } }`
/// then `[C new] m` → `1`.
///
/// The 17 corpus false positives were identical on the memoised and the
/// cold path (a fold-logic bug, not a memoisation one), so this pins both
/// halves: the instance-variable guard yields no `O100` anywhere, a
/// non-instance local in the same body still does, and the two paths
/// agree byte-for-byte.
#[test]
fn instance_variable_existence_guard_does_not_fold_on_either_check_path() {
    let dialect = "tcl8.6";
    let src = "oo::class create C {\n\
                       variable x\n\
                       constructor {} { ::set x 1 }\n\
                       method m {} {\n\
                           ::if {[::info exists x]} { ::puts got }\n\
                           ::if {[::info exists zzz]} { ::puts never }\n\
                       }\n\
                   }\n";

    let db = TclDatabase::default();
    let file = SourceFile::new(&db, src.to_owned(), dialect.to_owned(), None);
    let got = compiler_check_diagnostics(&db, file, cfg(&db));
    let registry = db.registry(dialect);
    let want = compiler_check_diagnostics_uncached(src, registry, dialect, None, None);

    // The O100 message names CFG blocks, not the source condition, so
    // identify the folded guard by slicing the span out of the source.
    let folded = |ds: &[CompilerCheck]| -> Vec<String> {
        ds.iter()
            .filter(|d| d.code == DiagCode::O100)
            .map(|d| src[d.span.start() as usize..d.span.end() as usize].to_owned())
            .collect()
    };
    assert_eq!(
        got.checks, want.checks,
        "memoised and cold compiler checks must agree on the instance-var fold"
    );
    let hints = folded(&got.checks);
    assert_eq!(
        hints.len(),
        1,
        "exactly the never-set `zzz` guard may fold, got {hints:?}"
    );
    assert!(
        hints[0].contains("zzz") && !hints[0].contains("exists x"),
        "the folded guard must be `zzz`, not the instance variable, got {hints:?}"
    );
    assert_eq!(folded(&want.checks), hints, "cold path must match");
}

/// The method/body-unit top-up must also **invalidate**: an edit inside a `TclOO`
/// method body has to move the memoised path's O1xx hints with it (and drop
/// them when the construct goes away), not serve a stale `proc_taint_solve`
/// result — the opposite failure direction from the missing-hints bug.
#[test]
fn method_body_checks_invalidate_on_edit() {
    use salsa::Setter as _;
    let dialect = "tcl8.6";
    let with_hint = |pad: &str| {
        format!(
            "oo::class create K {{\n{pad}\
                 method m {{}} {{\n\
                     ::if {{[::info exists Missing]}} {{ ::puts no }}\n\
                 }}\n\
                 }}\n"
        )
    };
    // Same class with the constant-branch guard removed — no O100 at all.
    let without_hint = "oo::class create K {\n\
             method m {} {\n\
                 ::puts no\n\
             }\n\
             }\n";

    let mut db = TclDatabase::default();
    let registry = db.registry(dialect);
    let file = SourceFile::new(&db, with_hint(""), dialect.to_owned(), None);

    let o100_spans = |ds: &[CompilerCheck]| -> Vec<(u32, u32)> {
        ds.iter()
            .filter(|d| d.code == DiagCode::O100)
            .map(|d| (d.span.start(), d.span.end()))
            .collect()
    };

    let mut seen: Vec<Vec<(u32, u32)>> = Vec::new();
    // 1. baseline, 2. body shifted by a prepended line (spans must move),
    // 3. the guard deleted (hint must disappear), 4. restored.
    for src in [
        with_hint(""),
        with_hint("\n\n"),
        without_hint.to_owned(),
        with_hint(""),
    ] {
        file.set_text(&mut db).to(src.clone());
        let got = compiler_check_diagnostics(&db, file, cfg(&db));
        let want = compiler_check_diagnostics_uncached(&src, registry, dialect, None, None);
        assert_eq!(got.checks, want.checks, "stale memo after edit to:\n{src}");
        seen.push(o100_spans(&got.checks));
    }
    assert_eq!(seen[0].len(), 1, "baseline must have the method-body O100");
    assert_ne!(
        seen[0], seen[1],
        "prepending lines must shift the method-body O100 span, not reuse the stale one"
    );
    assert!(
        seen[2].is_empty(),
        "deleting the guard must drop the method-body O100, got {:?}",
        seen[2]
    );
    assert_eq!(
        seen[3], seen[0],
        "restoring the guard must restore the span"
    );
}

#[test]
fn configured_taint_checks_consume_the_solve_original_module_owner() {
    let source = "proc pass {x} { return $x }\n\
                      proc danger {} { set u [gets stdin]; set p [pass $u]; exec $p }\n\
                      proc other {} { set z 1 }\n";
    let dialect = "tcl8.6";
    let db = TclDatabase::default();
    let file = SourceFile::new(&db, source.to_owned(), dialect.to_owned(), None);
    let policy = cfg(&db);
    let key = lexer_cfg_key(&db, dialect);
    let input = document_analysis_input(&db, file, policy).unwrap();
    let shared = compilation_unit_for_config(&db, file, key, policy).unwrap();
    let solved = proc_taint_solve_for_config(&db, file, key, policy).unwrap();
    let owner = solved.unit.as_ref();
    assert_eq!(
        owner.ir_module.source,
        tcl_lexer::SourceImage::document(source)
    );
    assert_eq!(
        owner.ir_module.source_metadata_input.as_ref(),
        Some(input.as_ref())
    );
    assert_eq!(owner.ir_module.lexer_config, input.lexer_config());
    assert!(!solved.taints.proc_taints.is_empty());
    for (id, taints) in &solved.taints.proc_taints {
        assert!(owner.ir_module.procedure_ids.owns(*id));
        assert!(!shared.ir_module.procedure_ids.owns(*id));
        let function = owner.function_for_body(Some(*id)).unwrap();
        assert!(std::ptr::eq(
            solved.taints.taints_for_function(owner, function),
            taints,
        ));
    }
    let registry = input.borrowed_context_registry().commands();
    let expected = compiler_check_diagnostics_uncached(source, registry, dialect, None, None);
    assert!(
        expected
            .checks
            .iter()
            .any(|check| check.code == DiagCode::T100)
    );
    let actual = compiler_check_diagnostics(&db, file, policy);
    assert_eq!(actual.checks, expected.checks);
    assert_eq!(actual.optimisations, expected.optimisations);
}

/// The interprocedural taint cascade (`taint_cascade`, backlog #1) must stay
/// byte-identical to the non-memoised `with_interprocedural` re-run **across
/// edits** — the cold corpus differential only proves the reconstruction is
/// complete, not that a stale cache can't survive an edit.  Drive a sequence
/// of edits (including one that flips a callee's passthrough/global-write
/// behaviour, which must invalidate its callers' cascades) and assert the
/// memoised diagnostics equal a fresh uncached build at every step.
#[test]
fn taint_cascade_matches_uncached_under_edits() {
    use salsa::Setter as _;
    let dialect = "tcl8.6";
    // A passthrough callee feeding a destructive sink in a caller, plus an
    // unrelated proc — exercises return-passthrough taint transfer and the
    // reachable-global seeding the cascade depends on.
    let versions = [
        "proc pass {x} { return $x }\n\
             proc danger {} { set u [gets stdin]; set p [pass $u]; exec $p }\n\
             proc other {} { set z 1 }\n",
        // Unrelated edit (other's body) — callers' cascades must be reused
        // yet still correct.
        "proc pass {x} { return $x }\n\
             proc danger {} { set u [gets stdin]; set p [pass $u]; exec $p }\n\
             proc other {} { set z 1; set z 2 }\n",
        // Flip the callee: no longer a passthrough (returns a constant) —
        // danger's cascade must recompute and drop the transferred taint.
        "proc pass {x} { return ok }\n\
             proc danger {} { set u [gets stdin]; set p [pass $u]; exec $p }\n\
             proc other {} { set z 1; set z 2 }\n",
        // Restore the passthrough — taint transfer must come back.
        "proc pass {x} { return $x }\n\
             proc danger {} { set u [gets stdin]; set p [pass $u]; exec $p }\n\
             proc other {} { set z 1; set z 2 }\n",
    ];
    // One warm db across the whole edit sequence — a fresh db per edit would
    // not exercise stale-cache reuse, which is the point.
    let mut db = TclDatabase::default();
    let registry = db.registry(dialect);
    let file = SourceFile::new(&db, versions[0].to_owned(), dialect.to_owned(), None);
    for src in versions {
        file.set_text(&mut db).to(src.to_owned());
        let got = compiler_check_diagnostics(
            &db,
            file,
            AnalyserConfig::new(
                &db,
                Vec::new(),
                NonAsciiMode::Default,
                Vec::new(),
                None,
                None,
                0,
                Vec::new(),
                Vec::new(),
            ),
        );
        let want = compiler_check_diagnostics_uncached(src, registry, dialect, None, None);
        assert_eq!(
            got.checks, want.checks,
            "cascade checks diverge after edit to:\n{src}"
        );
        assert_eq!(
            got.optimisations, want.optimisations,
            "cascade optimisations diverge after edit to:\n{src}"
        );
    }
}

/// Random-edit differential fuzzer for the **whole** memoised checks path.
/// The cold corpus differential and the
/// hand-written `taint_cascade_matches_uncached_under_edits` prove the memo is
/// complete and correct on a fixed edit script; this drives a **randomised**
/// sequence of incremental edits — body swaps, signature changes, and proc
/// add/remove across an interprocedural call graph — over **one warm db**,
/// asserting the memoised `compiler_check_diagnostics` (per-proc
/// `function_lattice` / `function_checks` / `proc_taint_solve` /
/// `proc_summary_cascade`) stays byte-identical (checks **and** optimisations)
/// to a from-scratch `compiler_check_diagnostics_uncached` after every edit.
/// Catches a stale per-proc cache or summary edge a fixed script would miss.
#[test]
#[allow(clippy::cast_possible_truncation)] // index modulo tiny arrays
fn compiler_check_incremental_matches_fresh_under_edits() {
    use salsa::Setter as _;
    let dialect = "tcl8.6";
    // Four interdependent slots, each with several variants (index 0 = the
    // proc is absent).  `caller` invokes `pass`/`leaf`, so flipping a callee's
    // passthrough / global-write / arity must cascade into the caller's solve;
    // the `calc` slot drives SCCP const-branch + loop optimisations (O1xx).
    let pass_variants = [
        "",
        "proc pass {x} { return $x }\n", // passthrough (taint transfer)
        "proc pass {x} { return ok }\n", // constant (drops taint)
        "proc pass {x y} { return $x$y }\n", // signature change (arity 2)
    ];
    let leaf_variants = [
        "",
        "proc leaf {} { global g; incr g }\n", // global write
        "proc leaf {} { set z 1 }\n",          // pure
    ];
    let caller_variants = [
        "",
        "proc caller {} { set u [gets stdin]; set p [pass $u]; exec $p }\n", // tainted sink
        "proc caller {} { leaf; set v [pass ok]; return $v }\n",             // benign
        "proc caller {} { if {1} { return 1 }\n return [pass 2] }\n",        // const branch
    ];
    let calc_variants = [
        "",
        "proc calc {n} { set acc 0\n for {set i 0} {$i < $n} {incr i} { set acc [expr {$acc + $i}] }\n return $acc }\n",
        "proc calc {n} { if {1} { set y 1 }\n return $y }\n",
    ];

    let assemble = |s: &[usize]| -> String {
        format!(
            "{}{}{}{}",
            pass_variants[s[0]], leaf_variants[s[1]], caller_variants[s[2]], calc_variants[s[3]],
        )
    };
    let lens = [
        pass_variants.len(),
        leaf_variants.len(),
        caller_variants.len(),
        calc_variants.len(),
    ];

    // xorshift64 — deterministic, reproducible run-to-run.
    let mut rng = 0xfeed_face_cafe_d00d_u64;
    let mut next = move || {
        rng ^= rng << 13;
        rng ^= rng >> 7;
        rng ^= rng << 17;
        rng
    };

    let mut state = [1usize, 1, 1, 1];
    let mut db = TclDatabase::default();
    let registry = db.registry(dialect);
    let file = SourceFile::new(&db, assemble(&state), dialect.to_owned(), None);

    for iter in 0..250 {
        let slot = (next() as usize) % state.len();
        state[slot] = (next() as usize) % lens[slot];
        let src = assemble(&state);
        file.set_text(&mut db).to(src.clone());

        let got = compiler_check_diagnostics(&db, file, cfg(&db));
        let want = compiler_check_diagnostics_uncached(&src, registry, dialect, None, None);
        assert_eq!(
            got.checks, want.checks,
            "iter {iter}: checks diverge from fresh build for state {state:?}:\n{src}"
        );
        assert_eq!(
            got.optimisations, want.optimisations,
            "iter {iter}: optimisations diverge from fresh build for state {state:?}:\n{src}"
        );
    }
}

/// The taint cascade memoises: a body edit to a procedure that no other
/// procedure's taint depends on must **not** re-execute the unrelated
/// procedures' `taint_cascade` (they reuse their cached taints), whereas the
/// edited procedure's own cascade does re-run.  Proves backlog #1 actually
/// skips the per-procedure `propagate_taints` re-run that
/// `with_interprocedural` did unconditionally.
#[test]
fn taint_cascade_reused_on_unrelated_edit() {
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
    let cascades = |log: &Arc<Mutex<Vec<String>>>| {
        std::mem::take(&mut *log.lock().unwrap())
            .into_iter()
            .filter(|s| s.contains("taint_cascade"))
            .count()
    };
    let mut db = TclDatabase {
        storage: salsa::Storage::new(Some(Box::new(sink))),
    };
    // Three procedures with no taint-relevant call edges between them.
    let file = SourceFile::new(
        &db,
        "proc a {} { set x 11111 }\n\
             proc b {} { set y 22222 }\n\
             proc c {} { set z 33333 }\n"
            .to_owned(),
        "tcl8.6".to_owned(),
        None,
    );
    let _ = compiler_check_diagnostics(
        &db,
        file,
        AnalyserConfig::new(
            &db,
            Vec::new(),
            NonAsciiMode::Default,
            Vec::new(),
            None,
            None,
            0,
            Vec::new(),
            Vec::new(),
        ),
    );
    assert_eq!(
        cascades(&log),
        3,
        "cold build: every procedure's taint cascade runs"
    );

    // Edit only `b`'s body — `a`/`c` are unaffected and their cascades are
    // cache hits; only `b`'s re-runs.
    file.set_text(&mut db).to("proc a {} { set x 11111 }\n\
             proc b {} { set y 99999999 }\n\
             proc c {} { set z 33333 }\n"
        .to_owned());
    let _ = compiler_check_diagnostics(
        &db,
        file,
        AnalyserConfig::new(
            &db,
            Vec::new(),
            NonAsciiMode::Default,
            Vec::new(),
            None,
            None,
            0,
            Vec::new(),
            Vec::new(),
        ),
    );
    assert_eq!(
        cascades(&log),
        1,
        "unrelated body edit -> exactly ONE taint cascade recomputes"
    );
}

/// The per-procedure optimiser memo (`function_optimisations`) must
/// skip an unrelated procedure across edits: a body edit that does not change a
/// proc's *opt-projection* (`OptDepsKey`) leaves every other proc's optimise a
/// cache hit, while the edited proc's own re-keys (its `FnLatticeKey` changed).
/// This is the per-edit incrementality win — the whole-module `optimise_unit`
/// re-optimised all procs on any edit.
#[test]
fn function_optimisations_reused_on_unrelated_edit() {
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
            .filter(|s| s.contains("function_optimisations"))
            .count()
    };
    let mut db = TclDatabase {
        storage: salsa::Storage::new(Some(Box::new(sink))),
    };
    // Three independent procs (no foldable cross-proc calls, no pure-non-const
    // proc → the memo path, not the whole-module fallback).
    let file = SourceFile::new(
        &db,
        "proc a {} { puts 11111 }\n\
             proc b {} { puts 22222 }\n\
             proc c {} { puts 33333 }\n"
            .to_owned(),
        "tcl8.6".to_owned(),
        None,
    );
    let _ = compiler_check_diagnostics(&db, file, cfg(&db));
    assert_eq!(
        runs(&log),
        3,
        "cold build: every procedure's optimise runs once"
    );

    // Edit only `b`'s body (a literal change that does not alter b's
    // opt-projection) — `a`/`c` are cache hits; only `b`'s optimise re-runs.
    file.set_text(&mut db).to("proc a {} { puts 11111 }\n\
             proc b {} { puts 99999999 }\n\
             proc c {} { puts 33333 }\n"
        .to_owned());
    let _ = compiler_check_diagnostics(&db, file, cfg(&db));
    assert_eq!(
        runs(&log),
        1,
        "unrelated body edit -> exactly ONE function_optimisations recomputes"
    );
}

/// The opt projection a memoised procedure reads its callees through keeps
/// what the dead-store passes ask of them: the completion, and each
/// parameter's default, which the call-by-name reads take for an omitted
/// `Name` argument.
#[test]
fn the_opt_projection_keeps_completion_and_defaults() {
    let mut summary = ProcSummary::unknown("::bumpd");
    summary.params = vec!["name".to_owned()];
    summary
        .param_defaults
        .insert("name".to_owned(), "n".to_owned());
    summary.completes = true;
    let projected = opt_callee_to_summary(&opt_callee_from_summary(&summary));
    assert_eq!(projected.param_defaults, summary.param_defaults);
    assert!(projected.completes);
}

/// A callee that completes whatever its arguments hold makes the unused
/// store of its call dead in the per-procedure optimiser memo as in the
/// whole-module build: the memo reads a callee's summary through its
/// opt projection, which carries the completion with the purity.
#[test]
fn a_completing_callee_reaches_the_optimiser_memo() {
    let log: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(Vec::new()));
    let sink = {
        let l = Arc::clone(&log);
        move |ev: salsa::Event| {
            if let salsa::EventKind::WillExecute { database_key } = ev.kind {
                l.lock().unwrap().push(format!("{database_key:?}"));
            }
        }
    };
    let db = TclDatabase {
        storage: salsa::Storage::new(Some(Box::new(sink))),
    };
    // `k` is pure with an argument-independent constant return and `f`
    // prints, so no procedure is an argument-sensitive fold target and the
    // module keeps the memo path rather than the whole-module fallback.
    let text = "proc k {} { return 1 }\nproc f {} {\n    set u [k]\n    puts done\n}\n";
    let file = SourceFile::new(&db, text.to_owned(), "tcl8.6".to_owned(), None);
    let memoised = compiler_check_diagnostics(&db, file, cfg(&db));
    assert_eq!(
        log.lock()
            .unwrap()
            .iter()
            .filter(|key| key.contains("function_optimisations"))
            .count(),
        2,
        "each procedure's optimisations come from the memo"
    );
    let registry = tcl_registry::model::ingress::static_context_for("tcl8.6").commands();
    let whole = compiler_check_diagnostics_uncached(text, registry, "tcl8.6", None, None);
    let codes = |optimisations: &[Optimisation]| -> Vec<(String, u32, u32)> {
        optimisations
            .iter()
            .map(|o| (o.code.as_str().to_owned(), o.span.start(), o.span.end()))
            .collect()
    };
    assert_eq!(codes(&memoised.optimisations), codes(&whole.optimisations));
    assert!(
        memoised
            .optimisations
            .iter()
            .any(|o| o.code.as_str() == "O126"),
        "the unused store of a completing call goes; got {:?}",
        memoised.optimisations
    );
}

/// The per-procedure body-lowering memo
/// (`lower_proc_body`) must skip an unchanged proc's body lowering across a
/// body-only edit. For a context-free file (no `namespace`/`oo::`/nested
/// `proc`), `build_unit_with_keys` lowers each top-level proc's static body
/// through `lower_proc_body`, keyed on the offset-0 body text — so editing one
/// proc's body re-lowers only that body; the others are cache hits. A shift
/// (prepended blank line) re-lowers nothing (offset-invariant key).
#[test]
fn lower_proc_body_reused_on_unrelated_edit() {
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
            .filter(|s| s.contains("lower_proc_body"))
            .count()
    };
    let mut db = TclDatabase {
        storage: salsa::Storage::new(Some(Box::new(sink))),
    };
    let file = SourceFile::new(
        &db,
        "proc a {} { puts 11111 }\n\
             proc b {} { puts 22222 }\n\
             proc c {} { puts 33333 }\n"
            .to_owned(),
        "tcl8.6".to_owned(),
        None,
    );
    let _ = compiler_check_diagnostics(&db, file, cfg(&db));
    assert_eq!(
        runs(&log),
        3,
        "cold build: every top-level proc body lowers once through the memo"
    );

    // Edit only `b`'s body — `a`/`c` bodies are cache hits.
    file.set_text(&mut db).to("proc a {} { puts 11111 }\n\
             proc b {} { puts 99999999 }\n\
             proc c {} { puts 33333 }\n"
        .to_owned());
    let _ = compiler_check_diagnostics(&db, file, cfg(&db));
    assert_eq!(
        runs(&log),
        1,
        "unrelated body edit -> exactly ONE lower_proc_body recomputes"
    );

    // Prepend a blank line: every body shifts but none changes — the offset-0
    // body key is identical, so no body re-lowers.
    file.set_text(&mut db).to("\nproc a {} { puts 11111 }\n\
             proc b {} { puts 99999999 }\n\
             proc c {} { puts 33333 }\n"
        .to_owned());
    let _ = compiler_check_diagnostics(&db, file, cfg(&db));
    assert_eq!(
        runs(&log),
        0,
        "pure offset shift -> no body re-lowers (offset-invariant key)"
    );
}

/// The interprocedural summary fixpoint memo (`proc_summary_cascade`)
/// must skip an unrelated procedure's `infer_proc_summary`
/// across edits: a body edit to one proc re-keys only that proc's summary
/// (its `FnLatticeKey` changed), while procedures it does not feed are cache
/// hits — this is what collapses the worklist's whole-unit pass-1 floor to the
/// edited proc's caller cascade.  Three procedures with no taint-relevant call
/// edges, so each is its own cascade root.
#[test]
fn proc_summary_cascade_reused_on_unrelated_edit() {
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
    let summaries = |log: &Arc<Mutex<Vec<String>>>| {
        std::mem::take(&mut *log.lock().unwrap())
            .into_iter()
            .filter(|s| s.contains("proc_summary_cascade"))
            .count()
    };
    let mut db = TclDatabase {
        storage: salsa::Storage::new(Some(Box::new(sink))),
    };
    let file = SourceFile::new(
        &db,
        "proc a {} { set x 11111 }\n\
             proc b {} { set y 22222 }\n\
             proc c {} { set z 33333 }\n"
            .to_owned(),
        "tcl8.6".to_owned(),
        None,
    );
    let _ = compiler_check_diagnostics(
        &db,
        file,
        AnalyserConfig::new(
            &db,
            Vec::new(),
            NonAsciiMode::Default,
            Vec::new(),
            None,
            None,
            0,
            Vec::new(),
            Vec::new(),
        ),
    );
    assert_eq!(
        summaries(&log),
        3,
        "cold build: every procedure's summary inference runs"
    );

    // Edit only `b`'s body — `a`/`c` summaries are cache hits; only `b`'s
    // `proc_summary_cascade` re-executes when its original summary inputs change.
    file.set_text(&mut db).to("proc a {} { set x 11111 }\n\
             proc b {} { set y 99999999 }\n\
             proc c {} { set z 33333 }\n"
        .to_owned());
    let _ = compiler_check_diagnostics(
        &db,
        file,
        AnalyserConfig::new(
            &db,
            Vec::new(),
            NonAsciiMode::Default,
            Vec::new(),
            None,
            None,
            0,
            Vec::new(),
            Vec::new(),
        ),
    );
    assert_eq!(
        summaries(&log),
        1,
        "unrelated body edit -> exactly ONE summary inference recomputes"
    );
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct SummaryDependencyProbe {
    caller_name: String,
    caller_body: String,
    provider: ReturnTaintSummary,
    dependencies: Vec<ReturnTaintSummary>,
    caller: ReturnTaintSummary,
}

#[salsa::tracked(returns(clone))]
fn summary_dependency_probe<'db>(
    db: &'db dyn TclDb,
    file: SourceFile,
    cfg: LexerCfgKey<'db>,
    analysis_config: AnalyserConfig,
    caller_offset: usize,
    caller: String,
) -> SummaryDependencyProbe {
    let dialect = file.dialect(db);
    let input =
        walked_document_input(db, file, analysis_config).expect("actual selected document input");
    let registry = input.borrowed_context_registry().commands();
    let declared = declared_command_surface(db, file, registry);
    let options = supplied_unit_build_options(db, file, cfg, registry, None, &declared, &input)
        .expect("the document's original grammar and Registry");
    let (cu, _) = build_unit_with_keys_and_input(db, file.text(db), options, Some(&input), None);
    let source_bodies: Vec<_> = cu
        .ir_module
        .procedures
        .iter()
        .chain(cu.ir_module.body_units.iter())
        .filter(|(id, _)| cu.original_procedure_for_body(**id).is_some())
        .collect();
    let known: HashSet<_> = source_bodies.iter().map(|(id, _)| **id).collect();
    let mut summaries: HashMap<_, _> = source_bodies
        .into_iter()
        .map(|(id, procedure)| {
            (
                *id,
                ReturnTaintSummary::untainted(&procedure.qualified_name, &procedure.params),
            )
        })
        .collect();
    let provider_id = original_test_body_id(&cu, 0);
    let caller_id = original_test_body_id(&cu, caller_offset);
    let provider = tcl_compiler::taint_interproc::infer_proc_summary(
        &cu,
        provider_id,
        registry,
        cu.interproc.as_ref(),
        tcl_lsp_core::stated_profile_for_dialect(dialect),
        &known,
        &summaries,
    )
    .expect("actual provider source unit");
    summaries.insert(provider_id, provider.clone());
    let body = cu
        .original_procedure_for_body(caller_id)
        .unwrap()
        .body_source
        .clone()
        .expect("actual caller source body");
    let capsule = tcl_compiler::body_analysis_capsule::BodyAnalysisCapsule::retain(
        Arc::new(cu),
        caller_id,
        registry,
    )
    .expect("actual current compiler owners");
    let dependencies = summary_deps_key(db, capsule, &summaries, &known, dialect);
    let summary = proc_summary_cascade(db, dependencies);
    SummaryDependencyProbe {
        caller_name: caller,
        caller_body: body,
        provider,
        dependencies: dependencies
            .inputs(db)
            .summaries()
            .iter()
            .map(|(_, summary)| summary.clone())
            .collect(),
        caller: (*summary).clone(),
    }
}

fn summary_memo_source(provider: &str) -> String {
    format!(
        "proc producer {{}} {{{provider}}}\n\
             proc quoted {{}} {{return [\"producer\"]}}\n\
             proc braced {{}} {{return [{{producer}}]}}\n"
    )
}

fn summary_memo_callers(text: &str) -> [(&'static str, usize); 2] {
    [
        ("::quoted", text.find("proc quoted ").unwrap()),
        ("::braced", text.find("proc braced ").unwrap()),
    ]
}

fn summary_memo_run_count(log: &Arc<Mutex<Vec<String>>>) -> usize {
    std::mem::take(&mut *log.lock().unwrap())
        .into_iter()
        .filter(|query| query.contains("proc_summary_cascade"))
        .count()
}

fn summary_memo_probes<'db>(
    db: &'db dyn TclDb,
    file: SourceFile,
    config: LexerCfgKey<'db>,
    analysis_config: AnalyserConfig,
) -> Vec<SummaryDependencyProbe> {
    summary_memo_callers(file.text(db))
        .into_iter()
        .map(|(caller, offset)| {
            summary_dependency_probe(db, file, config, analysis_config, offset, caller.to_owned())
        })
        .collect()
}

#[test]
fn summary_memo_observes_callee_edits_through_original_quoted_and_braced_heads() {
    // naming.compiler.original-summary-dependency-source-heads
    // docs/design/analysis/name-resolution-proofs/compiler-original-summary-dependency-source-heads.md
    // Actual document units feed the tracked summary query. This proves
    // dependency currency, not that a procedure or substitution executes.
    use salsa::Setter as _;
    let log = Arc::new(Mutex::new(Vec::new()));
    let sink = {
        let log = Arc::clone(&log);
        move |event: salsa::Event| {
            if let salsa::EventKind::WillExecute { database_key } = event.kind {
                log.lock().unwrap().push(format!("{database_key:?}"));
            }
        }
    };
    let mut db = TclDatabase {
        storage: salsa::Storage::new(Some(Box::new(sink))),
    };
    let initial = summary_memo_source("return CLEAN");
    let changed = summary_memo_source("return [gets stdin]");
    let file = SourceFile::new(&db, initial, "tcl".to_owned(), None);
    let config = lexer_cfg_key(&db, "tcl");
    let analysis_config = cfg(&db);
    let before = summary_memo_probes(&db, file, config, analysis_config);
    assert_eq!(
        summary_memo_run_count(&log),
        2,
        "two actual caller summary queries"
    );
    for (caller, offset) in summary_memo_callers(file.text(&db)) {
        let _ = summary_dependency_probe(
            &db,
            file,
            config,
            analysis_config,
            offset,
            caller.to_owned(),
        );
    }
    assert_eq!(
        summary_memo_run_count(&log),
        0,
        "unchanged document retains the summary memos"
    );
    file.set_text(&mut db).to(changed.clone());
    let config = lexer_cfg_key(&db, "tcl");
    let after = summary_memo_probes(&db, file, config, analysis_config);
    assert_eq!(
        summary_memo_run_count(&log),
        2,
        "callee summary edit invalidates both caller queries"
    );
    for (old, new) in before.iter().zip(&after) {
        assert_eq!(
            old.caller_body, new.caller_body,
            "caller source did not change"
        );
        assert_ne!(
            old.provider, new.provider,
            "the actual callee summary changed"
        );
        assert!(old.dependencies.contains(&old.provider));
        assert!(new.dependencies.contains(&new.provider));
        assert_ne!(old.dependencies, new.dependencies);
    }
    let fresh = TclDatabase::default();
    let fresh_file = SourceFile::new(&fresh, changed, "tcl".to_owned(), None);
    let fresh_config = lexer_cfg_key(&fresh, "tcl");
    let fresh_analysis_config = cfg(&fresh);
    for ((caller, offset), expected) in summary_memo_callers(fresh_file.text(&fresh))
        .into_iter()
        .zip(after)
    {
        assert_eq!(
            summary_dependency_probe(
                &fresh,
                fresh_file,
                fresh_config,
                fresh_analysis_config,
                offset,
                caller.to_owned()
            ),
            expected
        );
    }
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

/// Original header arities ignore body edits and recompute once when the
/// selected signature changes.
#[test]
fn project_command_arities_firewall() {
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
            .filter(|s| s.contains("project_original_command_arities"))
            .count()
    };
    let mut db = TclDatabase {
        storage: salsa::Storage::new(Some(Box::new(sink))),
    };
    let a = SourceFile::new(
        &db,
        "proc helper {x y} { set z 1 }\n".to_owned(),
        "tcl8.6".to_owned(),
        None,
    );
    let b = SourceFile::new(
        &db,
        "proc other {} {}\n".to_owned(),
        "tcl8.6".to_owned(),
        None,
    );
    let project = Project::new(&db, vec![a, b]);

    let headers = item_sigs(&db, a);
    let header = headers[0].original_declaration.as_ref().unwrap().name();

    // Cold: `helper` is a 2-param proc → arity (2, 2).
    assert_eq!(
        original_command_arity(
            &db,
            project,
            OriginalCommandSlot::new(&db, header.policy(), header.slot().clone()),
        )
        .map(|arities| (*arities).clone()),
        Some(vec![(2, 2)]),
        "cold: helper has arity (2, 2)"
    );
    let _ = runs(&log);

    // BODY edit to `helper` — `item_sigs(a)` is byte-identical (signatures
    // unchanged), so the arity table backdates and does NOT recompute.
    a.set_text(&mut db)
        .to("proc helper {x y} { set z 99999 }\n".to_owned());
    let _ = original_command_arity(
        &db,
        project,
        OriginalCommandSlot::new(&db, header.policy(), header.slot().clone()),
    );
    assert_eq!(
        runs(&log),
        0,
        "body edit must not recompute project_original_command_arities"
    );

    // SIGNATURE edit — drop a parameter — `item_sigs(a)` changes, so the table
    // recomputes exactly once and the new arity (1, 1) flows through.
    a.set_text(&mut db)
        .to("proc helper {x} { set z 99999 }\n".to_owned());
    let arities = original_command_arity(
        &db,
        project,
        OriginalCommandSlot::new(&db, header.policy(), header.slot().clone()),
    );
    assert_eq!(
        runs(&log),
        1,
        "signature edit must recompute project_original_command_arities exactly once"
    );
    assert_eq!(
        arities.map(|arities| (*arities).clone()),
        Some(vec![(1, 1)]),
        "the new parameter list flows through to arity (1, 1)"
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

/// Per-symbol cross-file precision: editing an *unrelated* proc's signature in
/// the defining file must **not** re-run a calling file's `project_diagnostics`.
///
/// `b` calls only `foo` (defined in `a`); `a` also defines `bar`, which `b`
/// never calls. Each demanded original source slot has its own arity query.
/// A signature edit to `bar` changes the project header table while the
/// projected `foo` header is unchanged, so salsa backdates that result →
/// `project_diagnostics(b)` does **not** re-execute (the per-symbol early-cutoff
/// the whole-table dependency could not give).  Editing `foo` itself, by
/// contrast, *must* re-run `b` and flow the new arity through.
#[test]
fn project_diagnostics_per_symbol_cutoff() {
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
    // Count `project_diagnostics` re-executions only (not the cheap accessor /
    // aggregate, which legitimately re-run on a signature edit).
    let runs = |log: &Arc<Mutex<Vec<String>>>| {
        std::mem::take(&mut *log.lock().unwrap())
            .into_iter()
            .filter(|s| s.contains("project_diagnostics"))
            .count()
    };
    let mut db = TclDatabase {
        storage: salsa::Storage::new(Some(Box::new(sink))),
    };
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
    // `a` defines `foo` (1 param) and an unrelated `bar`.
    let a = SourceFile::new(
        &db,
        "proc foo {x} {}\nproc bar {y} {}\n".to_owned(),
        "tcl8.6".to_owned(),
        None,
    );
    // `b` calls `foo` with one arg — resolves cross-file, fits arity (1, 1),
    // so its diagnostics are empty.  It never references `bar`.
    let b = SourceFile::new(&db, "foo 1\n".to_owned(), "tcl8.6".to_owned(), None);
    let project = Project::new(&db, vec![a, b]);

    assert!(
        project_diagnostics(&db, b, cfg, project).is_empty(),
        "cold: foo resolves cross-file (1 arg fits arity 1) → no diagnostics"
    );
    let _ = runs(&log);

    // Signature edit to the UNRELATED `bar` (add a param).  The arity table and
    // The project table changes, but the `foo` slot backdates → `b`'s
    // cross-file diagnostics must NOT re-run.
    a.set_text(&mut db)
        .to("proc foo {x} {}\nproc bar {y z} {}\n".to_owned());
    let after_unrelated = project_diagnostics(&db, b, cfg, project);
    assert_eq!(
        runs(&log),
        0,
        "unrelated proc's signature edit must not re-run the caller's project_diagnostics"
    );
    assert!(
        after_unrelated.is_empty(),
        "b's diagnostics unchanged by an edit to a proc it never calls"
    );

    // Control: a signature edit to `foo` itself (now needs 2 args) MUST re-run
    // `b` and surface the cross-file arity error.
    a.set_text(&mut db)
        .to("proc foo {x w} {}\nproc bar {y z} {}\n".to_owned());
    let after_foo = project_diagnostics(&db, b, cfg, project);
    assert_eq!(
        runs(&log),
        1,
        "the called proc's signature edit must re-run the caller's project_diagnostics"
    );
    assert!(
        after_foo.iter().any(|d| d.code == DiagCode::E002),
        "foo now needs 2 args but b passes 1 → cross-file E002 (too few)"
    );
}

/// Project diagnostics for a diagnostic-vec comparison: `(code, start, end,
/// message)` per diagnostic, in order (the analyser emits deterministically).
#[cfg(test)]
fn diag_keys(
    diags: &[tcl_compiler::analyser::types::Diagnostic],
) -> Vec<(String, u32, u32, String)> {
    diags
        .iter()
        .map(|d| {
            (
                d.code.to_string(),
                d.span.start(),
                d.span.end(),
                d.message.clone(),
            )
        })
        .collect()
}

/// Cross-file W123: a command unresolved locally but
/// defined as a `proc` in another project file must have its W123 suppressed
/// when (and only when) that file is in the `Project`.
#[test]
fn project_diagnostics_suppresses_cross_file_w123() {
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
    let a = SourceFile::new(
        &db,
        "helper foo bar\n".to_owned(),
        "tcl8.6".to_owned(),
        None,
    );
    let b = SourceFile::new(
        &db,
        "proc helper {x y} { return $x }\n".to_owned(),
        "tcl8.6".to_owned(),
        None,
    );
    let has_helper_w123 = |diags: &[tcl_compiler::analyser::types::Diagnostic]| {
        diags
            .iter()
            .any(|d| d.code == DiagCode::W123 && d.message.contains("helper"))
    };
    // A alone: `helper` is unresolved → W123 present.
    let proj_a = Project::new(&db, vec![a]);
    assert!(
        has_helper_w123(&project_diagnostics(&db, a, cfg, proj_a)),
        "helper must be unresolved (W123) when B is not in the project"
    );
    // A + B (B defines `proc helper`): cross-file resolved → W123 suppressed.
    let proj_ab = Project::new(&db, vec![a, b]);
    assert!(
        !has_helper_w123(&project_diagnostics(&db, a, cfg, proj_ab)),
        "helper must resolve cross-file (no W123) when B defines proc helper"
    );
}

/// The multi-file `incremental == fresh`
/// differential.  Drive a 2-file project through edits to the *defining* file
/// (`b`) — adding/removing the procs the calling file (`a`) invokes — and
/// assert the calling file's cross-file diagnostics always match a fresh
/// whole-project rebuild.  Catches any untracked read / non-deterministic
/// fold in `project_diagnostics` or its cross-file dependency edges.
/// Reduce a `u64` PRNG draw to an index into a small slice. The modulo
/// is done in `u64` and the result (always `< len`) converts back
/// losslessly, so there is no truncating cast.
fn pick_index(r: u64, len: usize) -> usize {
    usize::try_from(r % len as u64).unwrap()
}

#[test]
fn project_diagnostics_incremental_matches_fresh_under_edits() {
    use salsa::Setter as _;
    // `a` calls four commands; `set` is a builtin, the rest resolve only if
    // `b` defines them.
    let a_text = "alpha 1\nbeta 2\ngamma 3\nset x 4\ndelta 5\n";
    let b_variants = [
        "",
        "proc alpha {x} {}\n",
        "proc alpha {x} {}\nproc beta {y} {}\n",
        "proc beta {y} {}\nproc gamma {z} {}\n",
        "namespace eval ns { proc delta {q} {} }\n",
        "proc alpha {x} {}\nproc beta {y} {}\nproc gamma {z} {}\nproc delta {q} {}\n",
    ];
    let mk = |b_text: &str| -> Vec<(String, u32, u32, String)> {
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
        let a = SourceFile::new(&db, a_text.to_owned(), "tcl8.6".to_owned(), None);
        let b = SourceFile::new(&db, b_text.to_owned(), "tcl8.6".to_owned(), None);
        let project = Project::new(&db, vec![a, b]);
        diag_keys(&project_diagnostics(&db, a, cfg, project))
    };

    let mut db = TclDatabase::default();
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
    let a = SourceFile::new(&db, a_text.to_owned(), "tcl8.6".to_owned(), None);
    let b = SourceFile::new(&db, b_variants[0].to_owned(), "tcl8.6".to_owned(), None);
    let project = Project::new(&db, vec![a, b]);

    let mut rng = 0x1234_5678_9abc_def0_u64;
    let mut next = move || {
        rng ^= rng << 13;
        rng ^= rng >> 7;
        rng ^= rng << 17;
        rng
    };
    for _ in 0..60 {
        let b_text = b_variants[pick_index(next(), b_variants.len())];
        b.set_text(&mut db).to(b_text.to_owned());
        let inc = diag_keys(&project_diagnostics(&db, a, cfg, project));
        let fresh = mk(b_text);
        assert_eq!(inc, fresh, "incremental != fresh after b = {b_text:?}");
    }
}

/// Cross-file differential, both files edited: the
/// stronger sibling of the B-only fuzzer above — drive a 2-file project through
/// independent edits to **both** the calling file (`a`) and the defining file
/// (`b`), asserting `a`'s cross-file diagnostics always match a fresh
/// whole-project rebuild.  Editing the caller changes the call sites (and their
/// arg counts) while editing the callee changes the resolution/arity domain;
/// catches any stale cross-file edge that a single-file fuzzer would miss.
#[test]
fn project_diagnostics_incremental_matches_fresh_both_files_edited() {
    use salsa::Setter as _;
    // Caller variants: vary which commands are called and with how many args
    // (so the cross-file arity error path is exercised, not just W123).
    let a_variants = [
        "alpha 1\nbeta 2\n",
        "alpha 1 2 3\nbeta\n", // wrong arg counts → arity error once resolved
        "alpha\nset x 1\ngamma 9\n", // gamma may be unresolved → W123
        "beta 1 2\nalpha 7\n",
        "",
    ];
    // Defining variants: vary which procs exist and their arities.
    let b_variants = [
        "",
        "proc alpha {x} {}\n",
        "proc alpha {x} {}\nproc beta {y z} {}\n",
        "proc alpha {a b c} {}\nproc beta {} {}\nproc gamma {q} {}\n",
        "proc alpha {args} {}\nproc beta {x {y 1}} {}\n",
    ];
    let mk = |a_text: &str, b_text: &str| -> Vec<(String, u32, u32, String)> {
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
        let a = SourceFile::new(&db, a_text.to_owned(), "tcl8.6".to_owned(), None);
        let b = SourceFile::new(&db, b_text.to_owned(), "tcl8.6".to_owned(), None);
        let project = Project::new(&db, vec![a, b]);
        diag_keys(&project_diagnostics(&db, a, cfg, project))
    };

    let mut db = TclDatabase::default();
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
    let a = SourceFile::new(&db, a_variants[0].to_owned(), "tcl8.6".to_owned(), None);
    let b = SourceFile::new(&db, b_variants[0].to_owned(), "tcl8.6".to_owned(), None);
    let project = Project::new(&db, vec![a, b]);

    let mut rng = 0x0f0f_1234_dead_beef_u64;
    let mut next = move || {
        rng ^= rng << 13;
        rng ^= rng >> 7;
        rng ^= rng << 17;
        rng
    };
    for _ in 0..80 {
        // Edit one file (sometimes both) per round.
        let pick = next() % 3;
        let a_text = a_variants[pick_index(next(), a_variants.len())];
        let b_text = b_variants[pick_index(next(), b_variants.len())];
        if pick != 1 {
            a.set_text(&mut db).to(a_text.to_owned());
        }
        if pick != 0 {
            b.set_text(&mut db).to(b_text.to_owned());
        }
        // Read back the *current* committed texts for the fresh comparison.
        let cur_a = a.text(&db).clone();
        let cur_b = b.text(&db).clone();
        let inc = diag_keys(&project_diagnostics(&db, a, cfg, project));
        let fresh = mk(&cur_a, &cur_b);
        assert_eq!(inc, fresh, "incremental != fresh: a={cur_a:?} b={cur_b:?}");
    }
}

/// Cross-file arity: a call to a workspace-defined
/// proc with the wrong argument count emits the analyser's arity error
/// (`E003` too many here) — *not* the unrelated `W124` IP-literal warning — and
/// the W123 it would have drawn is suppressed; a correct count emits neither.
#[test]
fn project_diagnostics_emits_cross_file_arity() {
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
    // B defines `proc helper {x y}` — arity exactly 2.
    let b = SourceFile::new(
        &db,
        "proc helper {x y} { return $x }\n".to_owned(),
        "tcl8.6".to_owned(),
        None,
    );
    let has = |diags: &[tcl_compiler::analyser::types::Diagnostic], code: &str| {
        diags
            .iter()
            .any(|d| d.code.as_str() == code && d.message.contains("helper"))
    };

    // 3 args to a 2-param proc → E003 (too many), and the W123 is suppressed.
    let a3 = SourceFile::new(&db, "helper a b c\n".to_owned(), "tcl8.6".to_owned(), None);
    let p3 = Project::new(&db, vec![a3, b]);
    let d3 = project_diagnostics(&db, a3, cfg, p3);
    assert!(
        has(&d3, "E003"),
        "3 args to a 2-param cross-file proc must emit E003"
    );
    assert!(
        !has(&d3, "W124"),
        "must not reuse W124 (the IP-literal warning)"
    );
    assert!(
        !has(&d3, "W123"),
        "W123 must be suppressed once resolved cross-file"
    );

    // Correct arity (2 args) → no arity error and no W123.
    let a2 = SourceFile::new(&db, "helper a b\n".to_owned(), "tcl8.6".to_owned(), None);
    let p2 = Project::new(&db, vec![a2, b]);
    let d2 = project_diagnostics(&db, a2, cfg, p2);
    assert!(
        !has(&d2, "E002") && !has(&d2, "E003"),
        "correct arity → no arity error"
    );
    assert!(!has(&d2, "W123"), "resolved cross-file → no W123");
}

/// Mixed proc / non-proc tail: when a class (or alias
/// / ensemble) and a proc share a tail name, a call may dispatch to the
/// arity-less class command, so no arity error may fire even when the arg count
/// fits no proc arity — while the call still resolves (no W123).
#[test]
fn cross_file_arity_suppressed_for_mixed_proc_nonproc_tail() {
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
    // B: a class `Widget` AND a proc whose tail is also `Widget` (arity 1).
    let b = SourceFile::new(
        &db,
        "oo::class create Widget { method draw {} {} }\n\
             namespace eval ns { proc Widget {x} {} }\n"
            .to_owned(),
        "tcl8.6".to_owned(),
        None,
    );
    // The actual root class slot is occupied without a procedure arity;
    // a same-tailed declaration in another namespace cannot donate one.
    let proj_names = Project::new(&db, vec![b]);
    let headers = item_sigs(&db, b);
    let class = headers
        .iter()
        .find(|header| header.id.kind == tcl_compiler::analyser::ItemKind::Class)
        .unwrap();
    let name = class.original_declaration.as_ref().unwrap().name();
    let slot = OriginalCommandSlot::new(&db, name.policy(), name.slot().clone());
    assert_eq!(
        original_command_arity(&db, proj_names, slot).map(|arities| (*arities).clone()),
        Some(Vec::new()),
        "an occupied non-procedure slot must carry an empty arity list"
    );
    // A calls `Widget new extra` (3 args) — fits no proc arity, but resolves to
    // the class → neither an arity error nor W123.
    let a = SourceFile::new(
        &db,
        "Widget new extra\n".to_owned(),
        "tcl8.6".to_owned(),
        None,
    );
    let proj = Project::new(&db, vec![a, b]);
    let d = project_diagnostics(&db, a, cfg, proj);
    assert!(
        !d.iter()
            .any(|x| x.code == DiagCode::E002 || x.code == DiagCode::E003),
        "a mixed proc/non-proc tail must never draw an arity error"
    );
    assert!(
        !d.iter()
            .any(|x| x.code == DiagCode::W123 && x.message.contains("Widget")),
        "the call still resolves cross-file (no W123)"
    );
}

/// Object-instance method callbacks resolve cross-file — including when the
/// dispatch is **inside a proc body**, which the incremental per-item
/// firewall (`file_analysis_incremental`, used by `project_diagnostics`)
/// otherwise missed because it defers instance creation to the graft.
/// Registry object-factories now bind eagerly in an isolated body, so the
/// in-body `$g walk … -command cb` records the callback and its arity is
/// resolved against the other file.
#[test]
fn cross_file_in_proc_instance_method_callback_arity() {
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
    // B defines onNode with 2 params; `graph walk -command` appends 3.
    let b = SourceFile::new(
        &db,
        "proc onNode {a b} { }\n".to_owned(),
        "tcl9.0".to_owned(),
        None,
    );
    let has_e003 = |src: &str| {
        let a = SourceFile::new(&db, src.to_owned(), "tcl9.0".to_owned(), None);
        let p = Project::new(&db, vec![a, b]);
        project_diagnostics(&db, a, cfg, p)
            .iter()
            .any(|d| d.code.as_str() == "E003" && d.message.contains("onNode"))
    };
    // `struct::graph` is a tcllib command: C knows it only after the
    // `package require` (tclsh 8.4 to 9.1: invalid command name
    // "struct::graph" without it).
    // Named factory + dispatch both inside a proc body.
    assert!(
        has_e003(
            "package require struct::graph\n\
                 proc build {} {\n struct::graph g\n g walk root -command onNode\n}\n"
        ),
        "in-proc `struct::graph g; g walk -command onNode` must resolve cross-file arity (E003)"
    );
    // Handle form (`set g [struct::graph]`) inside a proc body.
    assert!(
        has_e003(
            "package require struct::graph\n\
                 proc build {} {\n set g [struct::graph]\n $g walk root -command onNode\n}\n"
        ),
        "in-proc `set g [struct::graph]; $g walk -command onNode` must resolve cross-file arity"
    );
    // Correct arity (3 params) is silent.
    let b3 = SourceFile::new(
        &db,
        "proc onNode {a b c} { }\n".to_owned(),
        "tcl9.0".to_owned(),
        None,
    );
    let a_ok = SourceFile::new(
        &db,
        "package require struct::graph\n\
             proc build {} {\n struct::graph g\n g walk root -command onNode\n}\n"
            .to_owned(),
        "tcl9.0".to_owned(),
        None,
    );
    let p_ok = Project::new(&db, vec![a_ok, b3]);
    assert!(
        !project_diagnostics(&db, a_ok, cfg, p_ok)
            .iter()
            .any(|d| (d.code.as_str() == "E003" || d.code.as_str() == "E002")
                && d.message.contains("onNode")),
        "a correct 3-param in-proc instance callback must be silent cross-file"
    );
}

/// Cross-file arity honours `disabled_diagnostics`:
/// the synthesised arity error is produced *after* the analyser's own code
/// filter (and the LSP lift doesn't re-filter), so it must replicate it —
/// disabling `E003` (while keeping W123) must drop the cross-file arity error,
/// yet the call still resolves (no W123).
#[test]
fn cross_file_arity_honors_disabled_code() {
    let db = TclDatabase::default();
    // B defines a 2-param proc; A calls it with 3 args (too many → E003).
    let b = SourceFile::new(
        &db,
        "proc helper {x y} { return $x }\n".to_owned(),
        "tcl8.6".to_owned(),
        None,
    );
    let a = SourceFile::new(&db, "helper a b c\n".to_owned(), "tcl8.6".to_owned(), None);
    let proj = Project::new(&db, vec![a, b]);
    let has = |diags: &[tcl_compiler::analyser::types::Diagnostic], code: &str| {
        diags
            .iter()
            .any(|d| d.code.as_str() == code && d.message.contains("helper"))
    };

    // E003 enabled (default): wrong arity surfaces as E003, W123 suppressed.
    let cfg_on = AnalyserConfig::new(
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
    let d_on = project_diagnostics(&db, a, cfg_on, proj);
    assert!(has(&d_on, "E003"), "baseline: E003 present when enabled");
    assert!(!has(&d_on, "W123"), "baseline: W123 suppressed (resolved)");

    // E003 disabled (W123 left enabled): no E003, W123 still suppressed — the
    // call genuinely resolves cross-file regardless of the arity-code toggle.
    let cfg_off = AnalyserConfig::new(
        &db,
        vec!["E003".to_owned()],
        NonAsciiMode::Default,
        Vec::new(),
        None,
        None,
        0,
        Vec::new(),
        Vec::new(),
    );
    let d_off = project_diagnostics(&db, a, cfg_off, proj);
    assert!(
        !has(&d_off, "E003"),
        "disabling E003 must suppress the cross-file arity error"
    );
    assert!(
        !has(&d_off, "W123"),
        "W123 stays suppressed even with E003 disabled"
    );
}

/// Cross-file arity independent of the W123 toggle:
/// disabling W123 (unknown-command) must NOT also silence cross-file arity —
/// the analyser drops the W123 markers the arity pass keys off, so
/// `project_diagnostics` drives off a W123-forced analysis.  A wrong-arg
/// cross-file call must still report `E003` (matching local arity), with no
/// W123 leaking through.
#[test]
fn cross_file_arity_survives_w123_disabled() {
    let db = TclDatabase::default();
    let b = SourceFile::new(
        &db,
        "proc helper {x y} { return $x }\n".to_owned(),
        "tcl8.6".to_owned(),
        None,
    );
    let has = |diags: &[tcl_compiler::analyser::types::Diagnostic], code: &str| {
        diags.iter().any(|d| d.code.as_str() == code)
    };
    // W123 disabled, E003 left enabled.
    let cfg = AnalyserConfig::new(
        &db,
        vec!["W123".to_owned()],
        NonAsciiMode::Default,
        Vec::new(),
        None,
        None,
        0,
        Vec::new(),
        Vec::new(),
    );

    // Wrong arity (3 args to a 2-param proc) → E003 still fires; no W123.
    let bad = SourceFile::new(&db, "helper a b c\n".to_owned(), "tcl8.6".to_owned(), None);
    let pb = Project::new(&db, vec![bad, b]);
    let d_bad = project_diagnostics(&db, bad, cfg, pb);
    assert!(
        has(&d_bad, "E003"),
        "cross-file arity must survive W123 being disabled, got: {:?}",
        d_bad.iter().map(|d| &d.code).collect::<Vec<_>>()
    );
    assert!(
        !has(&d_bad, "W123"),
        "W123 stays suppressed (it is disabled)"
    );

    // Correct arity → no arity error and no W123 (resolved, W123 disabled).
    let ok = SourceFile::new(&db, "helper a b\n".to_owned(), "tcl8.6".to_owned(), None);
    let pok = Project::new(&db, vec![ok, b]);
    let d_ok = project_diagnostics(&db, ok, cfg, pok);
    assert!(
        !has(&d_ok, "E002") && !has(&d_ok, "E003"),
        "correct arity → no arity error"
    );
    assert!(
        !has(&d_ok, "W123"),
        "no W123 (disabled, and resolved anyway)"
    );
}

/// `tclLsp.extraCommands` threaded through `AnalyserConfig` makes a named
/// command known, so calling it never draws a W123.
#[test]
fn extra_commands_config_suppresses_w123() {
    let db = TclDatabase::default();
    let src = "mylibsend foo\n";
    let has_w123 = |cfg: AnalyserConfig| {
        let file = SourceFile::new(&db, src.to_owned(), "tcl8.6".to_owned(), None);
        file_analysis_incremental(&db, file, cfg)
            .diagnostics
            .iter()
            .any(|d| d.code.as_str() == "W123")
    };
    // Baseline: unknown command → W123.
    let base = AnalyserConfig::new(
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
    assert!(has_w123(base), "baseline W123 expected");
    // With the command declared extra → suppressed.
    let cfg = AnalyserConfig::new(
        &db,
        Vec::new(),
        NonAsciiMode::Default,
        vec!["mylibsend".to_owned()],
        None,
        None,
        0,
        Vec::new(),
        Vec::new(),
    );
    assert!(!has_w123(cfg), "extraCommands should suppress W123");
}

/// Cross-file classes: a command resolving to a
/// class (the class command) defined in another project file is resolved —
/// W123 suppressed — and, being a non-proc, never draws an arity error.
#[test]
fn project_diagnostics_resolves_cross_file_class() {
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
    // B defines a TclOO class `Widget`.
    let b = SourceFile::new(
        &db,
        "oo::class create Widget { method draw {} {} }\n".to_owned(),
        "tcl8.6".to_owned(),
        None,
    );
    // A invokes the `Widget` class command (defined cross-file).
    let a = SourceFile::new(&db, "Widget new\n".to_owned(), "tcl8.6".to_owned(), None);
    let proj = Project::new(&db, vec![a, b]);
    let d = project_diagnostics(&db, a, cfg, proj);
    assert!(
        !d.iter()
            .any(|x| x.code == DiagCode::W123 && x.message.contains("Widget")),
        "cross-file class command must resolve (no W123)"
    );
    assert!(
        !d.iter()
            .any(|x| x.code == DiagCode::E002 || x.code == DiagCode::E003),
        "a class command has no proc arity → no arity error"
    );
}

/// Cross-file arity edge cases: exercise the
/// `proc_arity` `(min, max)` computation across required-only, optional
/// defaults, a trailing `args` (unbounded), and the no-parameter proc — the
/// subtle part where an off-by-one would mis-fire the arity error.  Checks the
/// *specific* code too: too-few → `E002`, too-many → `E003`.
#[test]
fn cross_file_arity_edge_cases() {
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
    let b = SourceFile::new(
        &db,
        "proc two {a b} {}\nproc opt {a {b 1}} {}\nproc variadic {a args} {}\nproc none {} {}\n"
            .to_owned(),
        "tcl8.6".to_owned(),
        None,
    );
    // The cross-file arity code drawn by calling `src` (file A over {A, B}):
    // Some("E002") too few, Some("E003") too many, None if it fits.
    let arity_code = |src: &str| -> Option<String> {
        let a = SourceFile::new(&db, src.to_owned(), "tcl8.6".to_owned(), None);
        let proj = Project::new(&db, vec![a, b]);
        project_diagnostics(&db, a, cfg, proj)
            .iter()
            .find(|d| d.code == DiagCode::E002 || d.code == DiagCode::E003)
            .map(|d| d.code.to_string())
    };
    let e002 = || Some("E002".to_owned());
    let e003 = || Some("E003".to_owned());
    // two {a b} → arity (2, 2).
    assert_eq!(
        arity_code("two 1\n"),
        e002(),
        "1 arg to a 2-param proc → too few"
    );
    assert_eq!(arity_code("two 1 2\n"), None, "2 args → ok");
    assert_eq!(arity_code("two 1 2 3\n"), e003(), "3 args → too many");
    // opt {a {b 1}} → arity (1, 2).
    assert_eq!(arity_code("opt\n"), e002(), "0 args to (1,2) → too few");
    assert_eq!(arity_code("opt 1\n"), None, "1 arg to (1,2) → ok");
    assert_eq!(arity_code("opt 1 2\n"), None, "2 args to (1,2) → ok");
    assert_eq!(
        arity_code("opt 1 2 3\n"),
        e003(),
        "3 args to (1,2) → too many"
    );
    // variadic {a args} → arity (1, unbounded).
    assert_eq!(
        arity_code("variadic\n"),
        e002(),
        "0 args to (1,∞) → too few"
    );
    assert_eq!(arity_code("variadic 1\n"), None, "1 arg → ok");
    assert_eq!(
        arity_code("variadic 1 2 3 4 5\n"),
        None,
        "many → ok (trailing args)"
    );
    // none {} → arity (0, 0).
    assert_eq!(arity_code("none\n"), None, "0 args to a no-param proc → ok");
    assert_eq!(
        arity_code("none 1\n"),
        e003(),
        "1 arg to a 0-param proc → too many"
    );
}

/// A proc whose parameter-list word is **computed** has
/// unknown formals, so the *cross-file* arity check must abstain.
/// `ItemSig` must carry an "unknown, not none" flag distinct from a
/// genuinely empty parameter list — without it the cross-file table
/// would read the empty list as "takes no arguments" and every call
/// would draw a false `E003` — on code both tclsh 9.0.4 and 8.6.16 run
/// (`proc makeargs {} {return {a b}}`; `proc p [makeargs] {…}`;
/// `info args p` → `a b`; `p 1 2` → runs).
#[test]
fn cross_file_arity_abstains_for_computed_parameter_lists() {
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
    let b = SourceFile::new(
        &db,
        "proc makeargs {} { return {a b} }\n\
             proc computed [makeargs] { return 1 }\n\
             set params {x y}\n\
             proc dollar $params { return 1 }\n\
             proc literal {a b} { return 1 }\n"
            .to_owned(),
        "tcl8.6".to_owned(),
        None,
    );
    let arity_code = |src: &str| -> Option<String> {
        let a = SourceFile::new(&db, src.to_owned(), "tcl8.6".to_owned(), None);
        let proj = Project::new(&db, vec![a, b]);
        project_diagnostics(&db, a, cfg, proj)
            .iter()
            .find(|d| d.code == DiagCode::E002 || d.code == DiagCode::E003)
            .map(|d| d.code.to_string())
    };
    // FP guards — every call count is acceptable for an unknown signature.
    for call in [
        "computed\n",
        "computed 1\n",
        "computed 1 2\n",
        "dollar 1 2\n",
    ] {
        assert_eq!(
            arity_code(call),
            None,
            "computed parameter list must abstain, not draw an arity error for {call:?}"
        );
    }
    // TP control — a literal list in the same file still checks arity.
    assert_eq!(arity_code("literal 1 2\n"), None, "2 args → ok");
    assert_eq!(
        arity_code("literal 1 2 3\n"),
        Some("E003".to_owned()),
        "a literal list is still checked"
    );
}

/// A required parameter
/// positioned *after* a defaulted one does not lower the minimum by
/// the defaulted parameters ahead of it — Tcl's argument binding is
/// strictly positional, so supplying a value for the later required
/// parameter requires also supplying one for every position before
/// it, including the "optional" one. Confirmed against real `tclsh`
/// 9.0.4: `proc opt {a {b 5} c} {}` accepts exactly 3 arguments,
/// never 2 — computing `min` as the count of non-default params (2)
/// would silently accept a 2-argument call that real Tcl rejects.
#[test]
fn cross_file_arity_required_after_default_forces_exact_count() {
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
    let b = SourceFile::new(
        &db,
        "proc opt {a {b 5} c} {}\n".to_owned(),
        "tcl8.6".to_owned(),
        None,
    );
    let arity_code = |src: &str| -> Option<String> {
        let a = SourceFile::new(&db, src.to_owned(), "tcl8.6".to_owned(), None);
        let proj = Project::new(&db, vec![a, b]);
        project_diagnostics(&db, a, cfg, proj)
            .iter()
            .find(|d| d.code == DiagCode::E002 || d.code == DiagCode::E003)
            .map(|d| d.code.to_string())
    };
    assert_eq!(
        arity_code("opt 1\n"),
        Some("E002".to_owned()),
        "1 arg → too few (min is 3, not 2)"
    );
    assert_eq!(
        arity_code("opt 1 2\n"),
        Some("E002".to_owned()),
        "2 args → still too few — real tclsh rejects this exact call"
    );
    assert_eq!(arity_code("opt 1 2 3\n"), None, "3 args → ok");
    assert_eq!(
        arity_code("opt 1 2 3 4\n"),
        Some("E003".to_owned()),
        "4 args → too many"
    );
}

/// Conservative arity: a `{*}`-expanded call has an
/// unknown runtime arg count (`argc == None`), so it must never draw an arity
/// error even though its literal word count looks wrong — while still resolving
/// (no W123).
#[test]
fn cross_file_arity_skips_expanded_call() {
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
    let b = SourceFile::new(
        &db,
        "proc two {a b} {}\n".to_owned(),
        "tcl8.6".to_owned(),
        None,
    );
    // `two {*}$lst` — one literal word, `{*}`-expanded → runtime arity unknown.
    let a = SourceFile::new(
        &db,
        "set lst {1 2 3}\ntwo {*}$lst\n".to_owned(),
        "tcl8.6".to_owned(),
        None,
    );
    let proj = Project::new(&db, vec![a, b]);
    let d = project_diagnostics(&db, a, cfg, proj);
    assert!(
        !d.iter()
            .any(|x| x.code == DiagCode::E002 || x.code == DiagCode::E003),
        "a {{*}}-expanded call must not draw an arity error (argc unknown)"
    );
    assert!(
        !d.iter()
            .any(|x| x.code == DiagCode::W123 && x.message.contains("two")),
        "the call still resolves cross-file (no W123)"
    );
}

/// Nested cross-file arity: a wrong-arg call to a
/// workspace proc *inside a command substitution* (`set x [helper a b c]`)
/// must still draw the cross-file arity error — the nested call's argument
/// count is statically known.
#[test]
fn cross_file_arity_in_command_substitution() {
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
    let b = SourceFile::new(
        &db,
        "proc helper {x y} { return $x }\n".to_owned(),
        "tcl8.6".to_owned(),
        None,
    );
    // `helper` called with 3 args inside a `[…]` substitution → too many.
    let a = SourceFile::new(
        &db,
        "set x [helper a b c]\n".to_owned(),
        "tcl8.6".to_owned(),
        None,
    );
    let proj = Project::new(&db, vec![a, b]);
    let d = project_diagnostics(&db, a, cfg, proj);
    assert!(
        d.iter()
            .any(|x| x.code == DiagCode::E003 && x.message.contains("helper")),
        "nested cross-file call must draw E003, got: {:?}",
        d.iter().map(|x| (&x.code, &x.message)).collect::<Vec<_>>()
    );
    assert!(
        !d.iter()
            .any(|x| x.code == DiagCode::W123 && x.message.contains("helper")),
        "the nested call still resolves cross-file (no W123)"
    );
}

/// Analyse a single-file project and return its diagnostic codes+messages.
fn callback_arity_diagnostics(src: &str) -> Vec<tcl_compiler::analyser::types::Diagnostic> {
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
    let f = SourceFile::new(&db, src.to_owned(), "tcl9.0".to_owned(), None);
    let proj = Project::new(&db, vec![f]);
    project_diagnostics(&db, f, cfg, proj).as_ref().clone()
}

fn callback_arity_codes(src: &str) -> Vec<(String, String)> {
    callback_arity_diagnostics(src)
        .iter()
        .map(|d| (d.code.as_str().to_owned(), d.message.clone()))
        .collect()
}

fn assert_callback_source_count(
    source: &str,
    input: &[u8],
    expected: &tcl_compiler::analyser::SourceCallbackArgumentCounts,
) {
    let diagnostics = callback_arity_diagnostics(source);
    let subject = diagnostics
        .iter()
        .find_map(|diagnostic| {
            let subject = diagnostic.callback_source_arity()?;
            (diagnostic.code == DiagCode::E003 && subject.prefix().name_input().bytes() == input)
                .then_some(subject)
        })
        .unwrap_or_else(|| panic!("{source}: no genuine E003 source subject in {diagnostics:?}"));
    assert_eq!(subject.argument_counts(), expected);
    assert!(subject.source_lookup().is_some());
}

#[test]
fn original_callback_diagnostics_keep_typed_subject_and_ignore_mutable_counts() {
    // naming.diagnostic.original-callback-signature-subject
    // docs/design/analysis/name-resolution-proofs/diagnostic-original-callback-signature-subject.md
    // Current source headers only; no callback registration or execution.
    let db = TclDatabase::default();
    let file = SourceFile::new(
        &db,
        "proc cb {a b} {return 0}\nlsort -command {cb fixed} {3 1 2}".to_owned(),
        "tcl9.0".to_owned(),
        None,
    );
    let project = Project::new(&db, vec![file]);
    let analysis = tcl_compiler::analyser::Analyser::new().analyse(file.text(&db), "tcl9.0");
    let mut invocations = analysis.command_invocations;
    for invocation in &mut invocations {
        if invocation.original_callback_prefix.is_some() {
            invocation.name = "wrong presentation".to_owned();
            invocation.range = tcl_lexer::Span::new(0, 1);
            invocation.callback_arity = Some(tcl_registry::AppendedArity::Unknown);
            invocation.callback_baked_args = usize::MAX;
            invocation.original_name_input = None;
            invocation.original_lookup = None;
        }
    }
    let mut output = Vec::new();
    apply_original_callback_arity(&db, project, &mut output, &invocations, |_| false);
    let diagnostic = output
        .first()
        .expect("authentic prefix, not mutable scalar counts");
    assert_eq!(output.len(), 1);
    assert_eq!(diagnostic.code, DiagCode::E003);
    let subject = diagnostic.callback_source_arity().unwrap();
    assert_eq!(subject.prefix().baked_argument_count(), 1);
    assert_eq!(
        subject.argument_counts(),
        &tcl_compiler::analyser::SourceCallbackArgumentCounts::Finite(vec![3])
    );
    assert_eq!(&file.text(&db)[diagnostic.span.as_range()], "cb");
    assert_eq!(
        invocations
            .iter()
            .find(|inv| inv.original_callback_prefix.is_some())
            .unwrap()
            .logical_callback_candidate_keys()
            .len(),
        0
    );
    assert_eq!(
        tcl_lsp_core::diagnostic_subject::diagnostic_subject_data(diagnostic).unwrap()["subject"]["purpose"],
        "signature"
    );
}

#[test]
fn original_callback_signature_query_updates_headers_without_tail_substitution() {
    // naming.diagnostic.original-callback-signature-subject
    // docs/design/analysis/name-resolution-proofs/diagnostic-original-callback-signature-subject.md
    use salsa::Setter as _;
    let mut db = TclDatabase::default();
    let definitions = SourceFile::new(&db,"namespace eval a {proc cb {one} {return FIRST}}\nnamespace eval b {proc cb {one two} {return 0}}".to_owned(),"tcl9.0".to_owned(),None);
    let call_source = "lsort -command ::a::cb {3 1 2}";
    let calls = SourceFile::new(&db, call_source.to_owned(), "tcl9.0".to_owned(), None);
    let project = Project::new(&db, vec![definitions, calls]);
    let analysis = tcl_compiler::analyser::Analyser::new().analyse(call_source, "tcl9.0");
    let prefix = analysis
        .command_invocations
        .iter()
        .find_map(|inv| inv.original_callback_prefix.as_ref())
        .unwrap();
    let lookup = prefix.lookup().unwrap();
    let first = original_lookup_command_signatures(&db, project, lookup).unwrap();
    assert_eq!(first.len(), 1);
    assert_eq!(
        first[0].formal_count_projection().arity(),
        tcl_registry::Arity::exact(1)
    );
    definitions.set_text(&mut db).to("namespace eval a {proc cb {one} {return LATER}}\nnamespace eval b {proc cb {one two} {return 0}}".to_owned());
    assert_eq!(
        original_lookup_command_signatures(&db, project, lookup).unwrap(),
        first,
        "body-free header equality"
    );
    definitions.set_text(&mut db).to("namespace eval a {proc cb {one two} {return LATER}}\nnamespace eval b {proc cb {one} {return 0}}".to_owned());
    let updated = original_lookup_command_signatures(&db, project, lookup).unwrap();
    assert_eq!(updated.len(), 1);
    assert_eq!(
        updated[0].formal_count_projection().arity(),
        tcl_registry::Arity::exact(2)
    );
    assert!(
        tcl_compiler::analyser::SourceCallbackAritySubject::from_source_signatures(
            Arc::clone(prefix),
            &updated
        )
        .is_none()
    );
}

#[test]
fn original_callback_signature_gap_retains_disjoint_source_headers() {
    // naming.diagnostic.original-callback-signature-subject
    // docs/design/analysis/name-resolution-proofs/diagnostic-original-callback-signature-subject.md
    // Two current source candidates, not two installed implementations.
    let db = TclDatabase::default();
    let one = SourceFile::new(&db, "proc cb {a} {}".to_owned(), "tcl9.0".to_owned(), None);
    let three = SourceFile::new(
        &db,
        "proc cb {a b c} {}".to_owned(),
        "tcl9.0".to_owned(),
        None,
    );
    let source = "lsort -command cb {1 2}";
    let call = SourceFile::new(&db, source.to_owned(), "tcl9.0".to_owned(), None);
    let project = Project::new(&db, vec![one, three, call]);
    let analysis = tcl_compiler::analyser::Analyser::new().analyse(source, "tcl9.0");
    let mut diagnostics = Vec::new();
    apply_original_callback_arity(
        &db,
        project,
        &mut diagnostics,
        &analysis.command_invocations,
        |_| false,
    );
    let diagnostic = diagnostics
        .first()
        .expect("finite count falls in a signature gap");
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostic.code, DiagCode::E005);
    let subject = diagnostic.callback_source_arity().unwrap();
    assert_eq!(subject.declarations().len(), 2);
    assert_eq!(
        subject.issue(),
        tcl_compiler::analyser::SourceCallbackArityIssue::NoCompatibleSignature { supplied: 2 }
    );
}

#[test]
fn qualified_callback_arity_keeps_same_tailed_namespaces_separate() {
    let diagnostics = callback_arity_codes(
        "namespace eval a {proc cb {one} {return 0}}\n\
             namespace eval b {proc cb {one two} {return 0}}\n\
             lsort -command ::a::cb {3 1 2}\n",
    );
    assert!(
        diagnostics.iter().any(|(code, _)| code == "E003"),
        "{diagnostics:?}"
    );
    let absent = callback_arity_codes(
        "namespace eval b {proc cb {one} {return 0}}\n\
             lsort -command ::absent::cb {3 1 2}\n",
    );
    assert!(
        absent
            .iter()
            .all(|(code, _)| !matches!(code.as_str(), "E002" | "E003" | "E005")),
        "{absent:?}"
    );
}

#[test]
fn callback_arity_mismatch_draws_e002() {
    // `lsort -command` appends 2 (Exactly(2)); `badCb` needs 3 → E002 too few.
    let d = callback_arity_codes("proc badCb {a b c} { return 0 }\nlsort -command badCb {3 1 2}\n");
    assert!(
        d.iter().any(|(c, m)| c == "E002" && m.contains("badCb")),
        "a callback proc needing 3 args used where 2 are appended must draw E002; got {d:?}"
    );
}

#[test]
fn callback_arity_correct_is_silent() {
    // A 2-arg callback matches lsort's Exactly(2) → no arity error (TN).
    let d = callback_arity_codes("proc goodCb {a b} { return 0 }\nlsort -command goodCb {3 1 2}\n");
    assert!(
        !d.iter().any(|(c, _)| c == "E002" || c == "E003"),
        "a correctly-sized callback must draw no arity error; got {d:?}"
    );
}

#[test]
fn callback_arity_too_many_draws_e003() {
    // A 1-param callback where lsort appends 2 → E003 too many.
    let d = callback_arity_codes("proc oneArg {a} { return 0 }\nlsort -command oneArg {3 1 2}\n");
    assert!(
        d.iter().any(|(c, m)| c == "E003" && m.contains("oneArg")),
        "a 1-param callback fed 2 args must draw E003; got {d:?}"
    );
}

#[test]
fn callback_arity_args_catchall_is_silent() {
    // FP guard: an `args` catch-all accepts any count → no arity error.
    let d = callback_arity_codes("proc anyN {args} { return 0 }\nlsort -command anyN {3 1 2}\n");
    assert!(
        !d.iter().any(|(c, _)| c == "E002" || c == "E003"),
        "an `args`-catchall callback must draw no arity error; got {d:?}"
    );
}

#[test]
fn callback_arity_braced_prefix_bakes_extra_args_too_few() {
    // `-command {cb 99}` bakes 1 extra arg ahead of `lsort`'s own
    // appended 2, for 3 total; `cb` needs 4 → E002.  A braced
    // multi-word prefix must be recorded as an invocation, or this
    // would draw nothing at all.
    let d =
        callback_arity_codes("proc cb {a b c d} { return 0 }\nlsort -command {cb 99} {3 1 2}\n");
    assert!(
        d.iter().any(|(c, m)| c == "E002" && m.contains("cb")),
        "a braced prefix's baked arg must count toward the total; got {d:?}"
    );
}

#[test]
fn callback_arity_braced_prefix_bakes_extra_args_exact_match_is_silent() {
    // Same shape, but `cb` needs exactly 3 (1 baked + 2 appended) — TN.
    let d = callback_arity_codes("proc cb {a b c} { return 0 }\nlsort -command {cb 99} {3 1 2}\n");
    assert!(
        !d.iter().any(|(c, _)| c == "E002" || c == "E003"),
        "1 baked + 2 appended matching cb's 3 params must be silent; got {d:?}"
    );
}

#[test]
fn callback_arity_braced_prefix_bakes_extra_args_too_many() {
    // `-command {cb 99 88}` bakes 2 + appended 2 = 4, but `cb` takes
    // only 3 → E003.
    let d =
        callback_arity_codes("proc cb {a b c} { return 0 }\nlsort -command {cb 99 88} {3 1 2}\n");
    assert!(
        d.iter().any(|(c, m)| c == "E003" && m.contains("cb")),
        "2 baked + 2 appended against a 3-param cb must draw E003; got {d:?}"
    );
}

#[test]
fn callback_arity_braced_prefix_dynamic_head_is_never_checked() {
    // FP guard: `{$cb 99}` — a dynamic head inside the braces — can't be
    // resolved to a proc; must never be flagged (and must not panic on
    // the list-parse).
    let d = callback_arity_codes(
        "proc cb {a b c d} { return 0 }\nset cb cb\nlsort -command {$cb 99} {3 1 2}\n",
    );
    assert!(
        !d.iter().any(|(c, _)| c == "E002" || c == "E003"),
        "a dynamic braced-prefix head must never be arity-checked; got {d:?}"
    );
}

#[test]
fn execution_trace_source_counts_keep_unavailable_future_frame() {
    // naming.callback.lookup-scope-owner
    // docs/design/analysis/name-resolution-proofs/callback-lookup-scope-owner.md
    // Registration syntax retains suffix metadata; it does not observe
    // the command table or namespace of a future triggering frame.
    for (operations, appended) in [
        ("enter", 2),
        ("enterstep", 2),
        ("leave", 4),
        ("leavestep", 4),
    ] {
        let source =
            format!("proc h {{a b c}} {{return 0}}\ntrace add execution somecmd {operations} h");
        let analysis = tcl_compiler::analyser::Analyser::new().analyse(&source, "tcl9.0");
        let prefix = analysis
            .command_invocations
            .iter()
            .find_map(|inv| inv.original_callback_prefix.as_ref())
            .expect(operations);
        assert_eq!(
            prefix.appended_arity(),
            Some(tcl_registry::AppendedArity::Exactly(appended))
        );
        assert_eq!(
            prefix.scope(),
            Some(tcl_registry::ScriptLookupScope::TriggerFrame)
        );
        assert!(prefix.lookup().is_none());
        assert!(
            callback_arity_codes(&source)
                .iter()
                .all(|(code, _)| !matches!(code.as_str(), "E002" | "E003" | "E005"))
        );
    }
}

#[test]
fn mixed_execution_trace_source_counts_do_not_issue_a_future_lookup() {
    // naming.callback.lookup-scope-owner
    // docs/design/analysis/name-resolution-proofs/callback-lookup-scope-owner.md
    for params in [
        "a b",
        "a b c",
        "a b c d",
        "a b {c default} {d default}",
        "a b args",
    ] {
        let source = format!(
            "proc h {{{params}}} {{return 0}}\ntrace add execution somecmd {{enter leave}} h"
        );
        let analysis = tcl_compiler::analyser::Analyser::new().analyse(&source, "tcl9.0");
        let prefix = analysis
            .command_invocations
            .iter()
            .find_map(|inv| inv.original_callback_prefix.as_ref())
            .expect(params);
        assert_eq!(
            prefix
                .appended_arity()
                .unwrap()
                .exact_counts()
                .unwrap()
                .collect::<Vec<_>>(),
            vec![2, 4]
        );
        assert!(prefix.lookup().is_none());
        assert!(
            callback_arity_codes(&source)
                .iter()
                .all(|(code, _)| !matches!(code.as_str(), "E002" | "E003" | "E005"))
        );
    }
}

#[test]
fn execution_trace_dynamic_and_malformed_operation_lists_abstain() {
    for source in [
        "proc h {a b c} { return 0 }\nset operations {enter leave}\ntrace add execution somecmd $operations h\n",
        "proc h {a b c} { return 0 }\ntrace add execution somecmd \"{enter\" h\n",
    ] {
        let diagnostics = callback_arity_codes(source);
        assert!(
            !diagnostics
                .iter()
                .any(|(code, _)| matches!(code.as_str(), "E002" | "E003" | "E005")),
            "an unproved operation list must not drive callback arity: {diagnostics:?}"
        );
    }
}

#[test]
fn callback_arity_namespace_unknown_zero_param_draws_e003() {
    // naming.source.original-callback-procedure-target
    // docs/design/analysis/name-resolution-proofs/source-original-callback-procedure-target.md
    // The absolute head selects a conditional source declaration without
    // borrowing the unentered trigger's namespace. AtLeast(1) cannot fit
    // the zero-parameter source header; registration itself calls nothing.
    assert_callback_source_count(
        "proc h {} { return 0 }\nnamespace unknown ::h\n",
        b"::h",
        &tcl_compiler::analyser::SourceCallbackArgumentCounts::AtLeast(1),
    );
}

#[test]
fn callback_arity_namespace_unknown_relative_trigger_does_not_borrow_installer_frame() {
    // naming.source.original-callback-procedure-target
    // docs/design/analysis/name-resolution-proofs/source-original-callback-procedure-target.md
    let diagnostics = callback_arity_codes("proc h {} {}; namespace unknown h");
    assert!(
        !diagnostics
            .iter()
            .any(|(code, _)| code == "E002" || code == "E003")
    );
}

fn assert_closed_tk_descriptor_contexts(
    available: &Arc<tcl_registry::model::assembly::ContextRegistry>,
    unavailable: &Arc<tcl_registry::model::assembly::ContextRegistry>,
    commands: &Arc<tcl_registry::CommandRegistry>,
) {
    assert!(Arc::ptr_eq(available.commands(), unavailable.commands()));
    assert!(
        available
            .context()
            .resolve_spec(commands, "scale")
            .is_some()
    );
    assert!(
        unavailable
            .context()
            .resolve_spec(commands, "scale")
            .is_none()
    );
    assert!(!unavailable.context().required_package_available(Some("Tk")));
    assert_eq!(unavailable.context().environment.id.as_str(), "bpf");
    assert_eq!(
        unavailable
            .context()
            .environment
            .policy_defaults
            .closed_world,
        tcl_dialect::model::WorldPolicy::Closed
    );
}

#[test]
fn callback_arity_closed_tk_descriptors_do_not_select_local_headers() {
    let available = tcl_registry::model::ingress::static_context_for("tcl9.0");
    let commands = Arc::clone(available.commands());
    let available = Arc::new(available.with_command_store(Arc::clone(&commands)));
    let unavailable = Arc::new(
        tcl_registry::model::ingress::static_context_for("bpf")
            .with_command_store(Arc::clone(&commands)),
    );
    assert_closed_tk_descriptor_contexts(&available, &unavailable, &commands);
    let profile = tcl_dialect::DialectProfile::find("tcl9.0").unwrap();
    let config = tcl_lexer::LexerConfig::for_dialect("tcl9.0");
    let input = |context| {
        tcl_compiler::analyser::ResolvedAnalysisInput::new(profile, profile, context, config)
    };
    let analyse = |source: &str, context| {
        tcl_compiler::analyser::Analyser::new()
            .with_resolved_input(input(context))
            .analyse(source, "tcl9.0")
    };
    let db = TclDatabase::default();
    let project = Project::new(&db, Vec::new());
    for source in [
        "proc cb {} {}; scale .s -command cb",
        "proc cb {} {}; scale .s -command ::cb",
        "package require Tk; proc cb {} {}; scale .s -command ::cb",
    ] {
        let analysis = analyse(source, Arc::clone(&unavailable));
        let supplied = input(Arc::clone(&unavailable));
        assert_eq!(analysis.resolved_input.as_ref(), Some(&supplied));
        assert!(
            analysis
                .retained_command_realm()
                .unwrap()
                .matches_resolved_analysis_input(&supplied)
        );
        assert!(
            analysis
                .matches_original_source_image(&tcl_lexer::SourceImage::document(source), config)
        );
        assert!(
            analysis
                .resolved_input
                .as_ref()
                .unwrap()
                .borrowed_context_registry()
                .context()
                .resolve_spec(&commands, "scale")
                .is_none()
        );
        let diagnostics =
            project_callback_diagnostics_for_analysis(&db, project, source, &analysis, |_| false);
        assert!(
            !diagnostics
                .iter()
                .any(|diagnostic| matches!(diagnostic.code, DiagCode::E002 | DiagCode::E003))
        );
    }
    let source = "proc cb {} {}; scale .s -command ::cb";
    let analysis = analyse(source, Arc::clone(&available));
    assert!(
        analysis
            .resolved_input
            .as_ref()
            .unwrap()
            .borrowed_context_registry()
            .context()
            .resolve_spec(&commands, "scale")
            .is_some()
    );
    let diagnostics =
        project_callback_diagnostics_for_analysis(&db, project, source, &analysis, |_| false);
    assert!(
        diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == DiagCode::E003)
    );
}

type ForeignCallbackContexts<'a> = (
    &'a Arc<tcl_registry::model::assembly::ContextRegistry>,
    &'a Arc<tcl_registry::model::assembly::ContextRegistry>,
    &'a Arc<tcl_registry::CommandRegistry>,
);

fn callback_input(
    context: Arc<tcl_registry::model::assembly::ContextRegistry>,
    profile: &'static tcl_dialect::DialectProfile,
    config: tcl_lexer::LexerConfig,
) -> tcl_compiler::analyser::ResolvedAnalysisInput {
    tcl_compiler::analyser::ResolvedAnalysisInput::new(profile, profile, context, config)
}

fn callback_analysis(
    source: &str,
    context: Arc<tcl_registry::model::assembly::ContextRegistry>,
    profile: &'static tcl_dialect::DialectProfile,
    config: tcl_lexer::LexerConfig,
) -> AnalysisResult {
    tcl_compiler::analyser::Analyser::new()
        .with_resolved_input(callback_input(context, profile, config))
        .analyse(source, "tcl9.0")
}

fn callback_codes_for_project(
    db: &TclDatabase,
    project: Project,
    source: &str,
    analysis: &AnalysisResult,
) -> Vec<(String, String)> {
    project_callback_diagnostics_for_analysis(db, project, source, analysis, |_| false)
        .into_iter()
        .map(|diagnostic| (diagnostic.code.as_str().to_owned(), diagnostic.message))
        .collect()
}

fn assert_foreign_callback_control_source(
    db: &TclDatabase,
    project: Project,
    source: &str,
    available: &Arc<tcl_registry::model::assembly::ContextRegistry>,
    profile: &'static tcl_dialect::DialectProfile,
    config: tcl_lexer::LexerConfig,
) -> (
    Vec<tcl_compiler::analyser::Diagnostic>,
    AnalysisResult,
    String,
) {
    let control_source = format!(
        "proc availability_control {{x}} {{}}; lsort -command ::availability_control {{2 1}}; {source}"
    );
    let control = callback_analysis(&control_source, Arc::clone(available), profile, config);
    assert!(
        control.matches_original_source_image(
            &tcl_lexer::SourceImage::document(&control_source),
            config
        )
    );
    let control_diagnostics =
        project_callback_diagnostics_for_analysis(db, project, &control_source, &control, |_| {
            false
        });
    assert!(
        control_diagnostics.iter().any(|diagnostic| {
            diagnostic.code == DiagCode::E003
                && diagnostic.callback_source_arity().is_some_and(|subject| {
                    subject.prefix().name_input().bytes() == b"::availability_control"
                        && subject.source_lookup().is_some()
                })
        }),
        "{control_source}"
    );
    (control_diagnostics, control, control_source)
}

fn assert_foreign_callback_sources_refuse_local_headers(
    db: &TclDatabase,
    project: Project,
    contexts: ForeignCallbackContexts<'_>,
    profile: &'static tcl_dialect::DialectProfile,
    config: tcl_lexer::LexerConfig,
) {
    let (available, unavailable, commands) = contexts;
    for source in [
        "proc cb {} {}; scale .s -command cb",
        "proc cb {x y} {}; math::calculus::integral 0 1 100 cb",
        "proc cb {x} {}; smtp::sendmessage $t -tlspolicy cb",
        "proc cb {x} {}; tcl::chan::halfpipe -write-command cb",
        "proc cb {} {}; mime::getbody $t -command cb",
        "proc cb {x y} {}; struct::graph g; g walk root -command cb",
        "proc cb {x y} {}; struct::tree t; t walkproc root cb",
        "package require smtp; proc cb {x} {}; smtp::sendmessage $t -tlspolicy ::cb",
        "package require mime; proc cb {} {}; mime::getbody $t -command ::cb",
        "package require struct::graph; proc cb {x y} {}; struct::graph g; g walk root -command ::cb",
        "package require struct::tree; proc cb {x y} {}; struct::tree t; t walkproc root ::cb",
    ] {
        // These library descriptors remain available in the closed model.
        // Its supplied input cannot borrow the other source producer's receipts.
        let mut analysis = callback_analysis(source, Arc::clone(available), profile, config);
        assert!(
            analysis
                .matches_original_source_image(&tcl_lexer::SourceImage::document(source), config,)
        );
        let supplied = callback_input(Arc::clone(unavailable), profile, config);
        analysis.resolved_input = Some(supplied.clone());
        assert!(
            analysis.resolved_input.as_ref() == Some(&supplied),
            "{source}: exact retained editing input"
        );
        let resolved = analysis.resolved_input.as_ref().unwrap();
        let current = resolved.context_registry();
        assert!(Arc::ptr_eq(current.commands(), commands));
        assert_eq!(resolved.lexer_config(), config);
        assert_eq!(resolved.vendor_source_policy(), None);
        assert!(!resolved.has_hosted_source_name_context());
        for command in [
            "smtp::sendmessage",
            "mime::getbody",
            "struct::graph",
            "struct::tree",
        ] {
            assert!(
                current.context().resolve_spec(commands, command).is_some(),
                "{source}: ordinary library metadata remains available"
            );
        }
        assert!(
            !analysis
                .retained_command_realm()
                .unwrap()
                .matches_resolved_analysis_input(&supplied)
        );
        assert!(
            !analysis
                .matches_original_source_image(&tcl_lexer::SourceImage::document(source), config)
        );
        assert!(
            analysis
                .original_procedure_declarations()
                .any(|declaration| declaration.metadata().qualified_name == "::cb")
        );
        let _retained_control =
            assert_foreign_callback_control_source(db, project, source, available, profile, config);
        let diagnostics = callback_codes_for_project(db, project, source, &analysis);
        assert!(
            !diagnostics
                .iter()
                .any(|(code, _)| code == "E002" || code == "E003"),
            "{source}: {diagnostics:?}"
        );
    }
}

fn assert_foreign_callback_source_currency_refusals(
    db: &TclDatabase,
    project: Project,
    contexts: ForeignCallbackContexts<'_>,
    profile: &'static tcl_dialect::DialectProfile,
    config: tcl_lexer::LexerConfig,
) {
    let (available, unavailable, commands) = contexts;
    for source in [
        "package require smtp; proc cb {x} {}; smtp::sendmessage $t -tlspolicy ::cb",
        "package require mime; proc cb {} {}; mime::getbody $t -command ::cb",
        "package require struct::graph; proc cb {x y} {}; struct::graph g; g walk root -command ::cb",
        "package require struct::tree; proc cb {x y} {}; struct::tree t; t walkproc root ::cb",
    ] {
        let analysis = callback_analysis(source, Arc::clone(available), profile, config);
        let diagnostics = callback_codes_for_project(db, project, source, &analysis);
        assert!(
            diagnostics.iter().any(|(code, _)| code == "E003"),
            "{source}: {diagnostics:?}"
        );
        let mut missing = analysis.clone();
        missing.resolved_input = None;
        assert!(
            !callback_codes_for_project(db, project, source, &missing)
                .iter()
                .any(|(code, _)| code == "E002" || code == "E003")
        );
        let mut foreign = analysis.clone();
        foreign.resolved_input = Some(callback_input(Arc::clone(unavailable), profile, config));
        assert!(
            !callback_codes_for_project(db, project, source, &foreign)
                .iter()
                .any(|(code, _)| code == "E002" || code == "E003")
        );
        let foreign_store = Arc::new(available.with_command_store(Arc::new(
            tcl_registry::CommandRegistry::build_default().project_for_profile(profile),
        )));
        assert_ne!(
            foreign_store.commands().snapshot().semantic_key(),
            commands.snapshot().semantic_key()
        );
        let mut foreign = analysis.clone();
        foreign.resolved_input = Some(callback_input(foreign_store, profile, config));
        assert!(
            !callback_codes_for_project(db, project, source, &foreign)
                .iter()
                .any(|(code, _)| code == "E002" || code == "E003")
        );
        assert!(
            !callback_codes_for_project(db, project, &format!("{source}\nputs OTHER"), &analysis)
                .iter()
                .any(|(code, _)| code == "E002" || code == "E003")
        );
    }
}

#[test]
fn callback_arity_foreign_library_source_contexts_do_not_select_local_headers() {
    // naming.database.original-project-callback-projection
    // docs/design/analysis/name-resolution-proofs/database-original-project-callback-projection.md
    let available = tcl_registry::model::ingress::static_context_for("tcl9.0");
    let commands = Arc::clone(available.commands());
    let available = Arc::new(available.with_command_store(Arc::clone(&commands)));
    let unavailable = Arc::new(
        tcl_registry::model::ingress::static_context_for("bpf")
            .with_command_store(Arc::clone(&commands)),
    );
    assert!(Arc::ptr_eq(available.commands(), unavailable.commands()));
    for command in [
        "smtp::sendmessage",
        "mime::getbody",
        "struct::graph",
        "struct::tree",
    ] {
        assert!(
            available
                .context()
                .resolve_spec(&commands, command)
                .is_some()
        );
        assert!(
            unavailable
                .context()
                .resolve_spec(&commands, command)
                .is_some()
        );
    }
    let profile = tcl_dialect::DialectProfile::find("tcl9.0").unwrap();
    let config = tcl_lexer::LexerConfig::for_dialect("tcl9.0");
    assert_eq!(unavailable.context().environment.id.as_str(), "bpf");
    assert_eq!(
        unavailable
            .context()
            .environment
            .policy_defaults
            .closed_world,
        tcl_dialect::model::WorldPolicy::Closed,
    );
    assert_eq!(unavailable.context().environment.expected_packages.len(), 0);
    let excluded_input = callback_input(Arc::clone(&unavailable), profile, config);
    assert!(!excluded_input.has_hosted_source_name_context());
    assert_eq!(excluded_input.vendor_source_policy(), None);
    let db = TclDatabase::default();
    let project = Project::new(&db, Vec::new());
    assert_foreign_callback_sources_refuse_local_headers(
        &db,
        project,
        (&available, &unavailable, &commands),
        profile,
        config,
    );
    assert_foreign_callback_source_currency_refusals(
        &db,
        project,
        (&available, &unavailable, &commands),
        profile,
        config,
    );
}

#[test]
fn callback_arity_package_unknown_variadic_handler_is_silent() {
    // FP guard: `package unknown` appends AtLeast(1); an `args` handler
    // absorbs any count → no arity error (the canonical handler shape).
    let d = callback_arity_codes("proc h {args} { return 0 }\npackage unknown h\n");
    assert!(
        !d.iter().any(|(c, _)| c == "E002" || c == "E003"),
        "a variadic `package unknown` handler must draw no arity error; got {d:?}"
    );
}

#[test]
fn callback_arity_tk_scale_command_arity_checked() {
    // Conditional source descriptor: `scale -command` appends Exactly(1).
    // A 0-param callback can't accept it → E003; a bareword 1-param callback
    // is silent (TN).
    let bad = callback_arity_codes(
        "# tcl-lsp: requires Tk\npackage require Tk\nproc onChange {} { }\nscale .s -command ::onChange\n",
    );
    assert!(
        bad.iter()
            .any(|(c, m)| c == "E003" && m.contains("onChange")),
        "a 0-param `scale -command` callback (1 appended) must draw E003; got {bad:?}"
    );
    let ok = callback_arity_codes(
        "# tcl-lsp: requires Tk\npackage require Tk\nproc onChange {v} { }\nscale .s -command ::onChange\n",
    );
    assert!(
        !ok.iter().any(|(c, _)| c == "E002" || c == "E003"),
        "a correct 1-param scale callback must be silent; got {ok:?}"
    );
    // A braced widget-path scroll callback is never arity-checked (not a
    // literal bareword head) — no false arity error.
    let widget = callback_arity_codes(
        "# tcl-lsp: requires Tk\npackage require Tk\nlistbox .lb -yscrollcommand {.sb set}\n",
    );
    assert!(
        !widget.iter().any(|(c, _)| c == "E002" || c == "E003"),
        "a braced widget-path scroll callback must not draw an arity error; got {widget:?}"
    );
}

#[test]
fn callback_arity_tcllib_calculus_func_arity_checked() {
    // `math::calculus::integral begin end nosteps func` calls `func x`
    // (Exactly(1), man-page-pinned).  A 2-param func is under-fed → E002.
    let d = callback_arity_codes(
        "# tcl-lsp: requires math::calculus\npackage require math::calculus\nproc f {x y} { expr {$x + $y} }\nmath::calculus::integral 0 1 100 ::f\n",
    );
    assert!(
        d.iter().any(|(c, m)| c == "E002" && m.contains("'::f'")),
        "a 2-param func where calculus::integral appends 1 must draw E002; got {d:?}"
    );
    // The correct 1-param shape is silent (TN).
    let ok = callback_arity_codes(
        "# tcl-lsp: requires math::calculus\npackage require math::calculus\nproc f {x} { expr {$x * 2} }\nmath::calculus::integral 0 1 100 ::f\n",
    );
    assert!(
        !ok.iter().any(|(c, _)| c == "E002" || c == "E003"),
        "a correct 1-param calculus func must be silent; got {ok:?}"
    );
}

#[test]
fn callback_arity_unknown_appended_never_fires() {
    // FP guard: `coroprobe` carries `Unknown` appended arity (depends on
    // the yield point), so the injected command is a reference only —
    // never arity-checked, whatever its param count. `coroinject` is not
    // an example of this guard: its own implementation always appends a
    // verified `Exactly(2)` (see `coroinject.rs`), so a 0-param `h` there
    // correctly draws E003 instead.
    let d = callback_arity_codes("proc h {} { return 0 }\ncoroprobe myCoro h\n");
    assert!(
        !d.iter().any(|(c, _)| c == "E002" || c == "E003"),
        "an Unknown-arity callback must never draw an arity error; got {d:?}"
    );
}

#[test]
fn callback_arity_option_value_exactly_checked() {
    // naming.database.original-project-callback-projection
    // docs/design/analysis/name-resolution-proofs/database-original-project-callback-projection.md
    // Required package metadata supplies source grammar, never loader or
    // future callback execution. The subject retains the actual prefix.
    // `smtp -tlspolicy` and `halfpipe -write-command` both append exactly 2;
    // a 1-param callback is over-fed → E003, a 2-param callback is silent.
    assert_callback_source_count(
        "# tcl-lsp: requires smtp\npackage require smtp\nproc pol {code} { }\nsmtp::sendmessage $t -tlspolicy ::pol\n",
        b"::pol",
        &tcl_compiler::analyser::SourceCallbackArgumentCounts::Finite(vec![2]),
    );
    let smtp_ok = callback_arity_codes(
        "# tcl-lsp: requires smtp\npackage require smtp\nproc pol {code diag} { }\nsmtp::sendmessage $t -tlspolicy ::pol\n",
    );
    assert!(
        !smtp_ok.iter().any(|(c, _)| c == "E002" || c == "E003"),
        "a correct 2-param -tlspolicy callback must be silent; got {smtp_ok:?}"
    );
    assert_callback_source_count(
        "# tcl-lsp: requires tcl::chan::halfpipe\npackage require tcl::chan::halfpipe\nproc w {chan} { }\ntcl::chan::halfpipe -write-command ::w\n",
        b"::w",
        &tcl_compiler::analyser::SourceCallbackArgumentCounts::Finite(vec![2]),
    );
}

#[test]
fn callback_arity_mime_getbody_command_atleast_one() {
    // naming.database.original-project-callback-projection
    // docs/design/analysis/name-resolution-proofs/database-original-project-callback-projection.md
    // Required package metadata supplies source grammar, never loader or
    // future callback execution. The subject retains the actual prefix.
    // `mime::getbody -command` appends AtLeast(1) (reason keyword + optional
    // payload).  A 0-param callback can't accept the reason word → E003; the
    // canonical `{reason args}` shape is silent (open-ended max ⇒ no
    // false "too many").
    assert_callback_source_count(
        "# tcl-lsp: requires mime\npackage require mime\nproc cb {} { }\nmime::getbody $t -command ::cb\n",
        b"::cb",
        &tcl_compiler::analyser::SourceCallbackArgumentCounts::AtLeast(1),
    );
    let ok = callback_arity_codes(
        "# tcl-lsp: requires mime\npackage require mime\nproc cb {reason args} { }\nmime::getbody $t -command ::cb\n",
    );
    assert!(
        !ok.iter().any(|(c, _)| c == "E002" || c == "E003"),
        "a `{{reason args}}` mime -command callback must be silent; got {ok:?}"
    );
    // FP guard: comm's 14-arg reply callback is virtually always `{args}` —
    // the catch-all absorbs all 14 → no arity error.
    let comm = callback_arity_codes(
        "# tcl-lsp: requires comm\npackage require comm\nproc reply {args} { }\ncomm::comm send -command ::reply $id {list x}\n",
    );
    assert!(
        !comm.iter().any(|(c, _)| c == "E002" || c == "E003"),
        "a variadic comm -command ::reply handler must be silent; got {comm:?}"
    );
}

#[test]
fn callback_arity_struct_graph_walk_command_checked() {
    // naming.database.original-project-callback-projection
    // docs/design/analysis/name-resolution-proofs/database-original-project-callback-projection.md
    // Required package metadata supplies source grammar, never loader or
    // future callback execution. The subject retains the actual prefix.
    // `$g walk … -command ::cb` (object instance method) appends 3 (action
    // graphName node).  A 2-param callback is over-fed → E003; a 3-param one
    // is silent.  Exercises both the named (`struct::graph name`) and handle
    // (`set g [struct::graph]`) instance forms.
    assert_callback_source_count(
        "# tcl-lsp: requires struct::graph\npackage require struct::graph\nproc twoP {a b} { }\nstruct::graph myG\nmyG walk root -command ::twoP\n",
        b"::twoP",
        &tcl_compiler::analyser::SourceCallbackArgumentCounts::Finite(vec![3]),
    );
    let ok = callback_arity_codes(
        "# tcl-lsp: requires struct::graph\npackage require struct::graph\nproc threeP {a b c} { }\nset g [struct::graph]\n$g walk root -command ::threeP\n",
    );
    assert!(
        !ok.iter().any(|(c, _)| c == "E002" || c == "E003"),
        "a correct 3-param graph walk callback must be silent; got {ok:?}"
    );
}

#[test]
fn callback_arity_struct_tree_walkproc_checked() {
    // naming.database.original-project-callback-projection
    // docs/design/analysis/name-resolution-proofs/database-original-project-callback-projection.md
    // Required package metadata supplies source grammar, never loader or
    // future callback execution. The subject retains the actual prefix.
    // `$t walkproc … cmdprefix` (trailing positional prefix) appends 3 (tree
    // node action).  A 2-param callback → E003.
    assert_callback_source_count(
        "# tcl-lsp: requires struct::tree\npackage require struct::tree\nproc twoP {a b} { }\nstruct::tree myT\nmyT walkproc root ::twoP\n",
        b"::twoP",
        &tcl_compiler::analyser::SourceCallbackArgumentCounts::Finite(vec![3]),
    );
}

/// Regression (whole-file-shift determinism): a pure prepend that shifts every
/// procedure must leave every `function_lattice` a cache hit — reliably, not by
/// HashMap-seed luck.  Before `prepare_cfg_context` was made deterministic, the
/// memo key flaked and this could re-execute *all* procedures.  Two procedures
/// share the short name `x` (the collision that exposed the nondeterminism).
#[test]
fn function_lattice_reused_on_whole_file_shift() {
    use salsa::Setter as _;
    let log: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(Vec::new()));
    let l = Arc::clone(&log);
    let sink = move |ev: salsa::Event| {
        if let salsa::EventKind::WillExecute { database_key } = ev.kind {
            l.lock().unwrap().push(format!("{database_key:?}"));
        }
    };
    let mut db = TclDatabase {
        storage: salsa::Storage::new(Some(Box::new(sink))),
    };
    let src = "namespace eval ::a { proc x {p} { set q $p; return $q } }\n\
                   namespace eval ::b { proc x {p} { set q $p; return $q } }\n\
                   proc top {} { set z 1 }\n";
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
    let file = SourceFile::new(&db, src.to_owned(), "tcl8.6".to_owned(), None);
    let _ = file_analysis_incremental(&db, file, cfg);
    log.lock().unwrap().clear();
    // Pure whole-file shift: prepend a blank line (every proc shifts, none
    // change).  All offset-0 lattice keys are unchanged -> all cache hits.
    file.set_text(&mut db).to(format!("\n{src}"));
    let _ = file_analysis_incremental(&db, file, cfg);
    let reexec = std::mem::take(&mut *log.lock().unwrap())
        .into_iter()
        .filter(|s| s.contains("function_lattice"))
        .count();
    assert_eq!(
        reexec, 0,
        "whole-file shift must reuse every proc lattice (deterministic key): {reexec} re-ran"
    );
}

/// A length-changing body edit to one procedure shifts the others but must
/// recompute exactly ONE `function_lattice` (the salsa-native per-procedure
/// lattice is offset-invariant: an unedited-but-shifted body interns to the
/// same key and is a cache hit, rebased to its new offset).
#[test]
fn function_lattice_reused_on_body_shift() {
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
    let mut db = TclDatabase {
        storage: salsa::Storage::new(Some(Box::new(sink))),
    };
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
    let file = SourceFile::new(
        &db,
        "proc a {} { set x 11111 }\nproc b {} { set y 22222 }\n".to_owned(),
        "tcl8.6".to_owned(),
        None,
    );
    let _ = file_analysis_incremental(&db, file, cfg);
    let init = std::mem::take(&mut *log.lock().unwrap());
    assert_eq!(
        init.iter()
            .filter(|s| s.contains("function_lattice"))
            .count(),
        2,
        "initial: both procedures' lattices built: {init:?}"
    );

    // Edit a's body length — shifts b's offset; b's offset-0 body is
    // unchanged, so its `function_lattice` key is unchanged (cache hit).
    file.set_text(&mut db)
        .to("proc a {} { set x 9999999999 }\nproc b {} { set y 22222 }\n".to_owned());
    let _ = file_analysis_incremental(&db, file, cfg);
    let after = std::mem::take(&mut *log.lock().unwrap());
    assert_eq!(
        after
            .iter()
            .filter(|s| s.contains("function_lattice"))
            .count(),
        1,
        "length-changing body edit -> exactly ONE lattice recomputes (offset-invariant): {after:?}"
    );
}

/// Direct measurement of per-edit *check* breadth:
/// a one-procedure body edit rebuilds exactly ONE `function_lattice` (the
/// per-proc memo works), but `compiler_check_diagnostics` re-executes wholesale
/// — `run_all_checks` is not a per-proc salsa query (it emits no `WillExecute`
/// event of its own), so it re-checks every procedure on every edit.  The
/// assertions pin the current breadth.
#[test]
fn check_diagnostics_rerun_whole_file_on_body_edit() {
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
    let mut db = TclDatabase {
        storage: salsa::Storage::new(Some(Box::new(sink))),
    };
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
    // Four independent procedures; we edit `b`'s body and leave a, c, d alone.
    let file = SourceFile::new(
        &db,
        "proc a {} { set x 11111 }\n\
             proc b {} { set y 22222 }\n\
             proc c {} { set z 33333 }\n\
             proc d {} { set w 44444 }\n"
            .to_owned(),
        "tcl8.6".to_owned(),
        None,
    );
    let _ = file_analysis_incremental(&db, file, cfg);
    let _ = compiler_check_diagnostics(&db, file, cfg);
    log.lock().unwrap().clear();

    // Length-changing body edit to ONE procedure (`b`); a/c/d shift but their
    // offset-0 lattice keys are unchanged (cache hits).
    file.set_text(&mut db).to("proc a {} { set x 11111 }\n\
             proc b {} { set y 222222222 }\n\
             proc c {} { set z 33333 }\n\
             proc d {} { set w 44444 }\n"
        .to_owned());
    let _ = file_analysis_incremental(&db, file, cfg);
    let _ = compiler_check_diagnostics(&db, file, cfg);
    let after = std::mem::take(&mut *log.lock().unwrap());
    let count = |q: &str| after.iter().filter(|s| s.contains(q)).count();

    // Per-proc memo: exactly one procedure's lattice rebuilds.
    assert_eq!(
        count("function_lattice"),
        1,
        "one-proc body edit -> ONE function_lattice rebuild: {after:?}"
    );
    // Checks re-run wholesale: compiler_check_diagnostics re-executes, and
    // run_all_checks (not a salsa query) re-checks every procedure inside it.
    assert_eq!(
        count("compiler_check_diagnostics"),
        1,
        "checks re-run whole-file every edit (no per-proc check memo): {after:?}"
    );
    assert_eq!(
        count("run_all_checks"),
        0,
        "run_all_checks is not a salsa query, so it has no WillExecute event: {after:?}"
    );
}

/// Constant parameter seeds participate in the lattice memo key.
/// An unrelated edit reuses the callee; changed caller input rebuilds it.
#[test]
fn function_lattice_memoises_param_constant_procs() {
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
    let lattice_execs = |log: &Arc<Mutex<Vec<String>>>| {
        std::mem::take(&mut *log.lock().unwrap())
            .into_iter()
            .filter(|s| s.contains("function_lattice"))
            .count()
    };
    let mut db = TclDatabase {
        storage: salsa::Storage::new(Some(Box::new(sink))),
    };
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
    // `other` first so editing it shifts the two below; `target` takes a
    // param `caller` always passes the literal `42` for -> param_constants.
    let file = SourceFile::new(
        &db,
        "proc other {} { set z 1 }\n\
             proc target {x} { set y $x }\n\
             proc caller {} { target 42 }\n"
            .to_owned(),
        "tcl8.6".to_owned(),
        None,
    );
    let _ = file_analysis_incremental(&db, file, cfg);
    assert_eq!(
        lattice_execs(&log),
        3,
        "cold build: every proc (incl. the param-constant callee) memoised"
    );

    // Length-changing edit to `other` shifts `target`/`caller`; their
    // offset-0 bodies (and `target`'s param_constants) are unchanged, so
    // they are cache hits — exactly one lattice recomputes.
    file.set_text(&mut db)
        .to("proc other {} { set z 123456789 }\n\
             proc target {x} { set y $x }\n\
             proc caller {} { target 42 }\n"
            .to_owned());
    let _ = file_analysis_incremental(&db, file, cfg);
    assert_eq!(
        lattice_execs(&log),
        1,
        "unrelated body edit -> param-constant callee reused (offset-invariant hit)"
    );

    // Change the caller's literal -> `target`'s param_constants change ->
    // its `FnLatticeKey` re-interns -> its lattice rebuilds.
    file.set_text(&mut db)
        .to("proc other {} { set z 123456789 }\n\
             proc target {x} { set y $x }\n\
             proc caller {} { target 99 }\n"
            .to_owned());
    let _ = file_analysis_incremental(&db, file, cfg);
    assert!(
        lattice_execs(&log) >= 1,
        "caller literal change rebuilds the param-constant callee's lattice"
    );
}

/// Both diagnostics consumers must **share one `compilation_unit` build per
/// edit** when their lexer configs coincide (every dialect but `tcl8.4` /
/// `f5-irules`): demanding `file_analysis_incremental` then
/// `compiler_check_diagnostics` in the same revision executes
/// `compilation_unit` exactly once.  For `tcl8.4` the configs differ
/// (`expand_syntax`), so each consumer builds its own — executed twice.
#[test]
fn compilation_unit_shared_across_consumers() {
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
    let mut db = TclDatabase {
        storage: salsa::Storage::new(Some(Box::new(sink))),
    };
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
    let src = "proc a {x} { return $x }\nproc b {} { a 1 }\n";
    let count_cu = |log: &Arc<Mutex<Vec<String>>>| {
        std::mem::take(&mut *log.lock().unwrap())
            .iter()
            .filter(|s| s.contains("compilation_unit"))
            .count()
    };

    // tcl8.6: both consumers intern the document's environment id, so
    // they share one build.
    let file86 = SourceFile::new(&db, src.to_owned(), "tcl8.6".to_owned(), None);
    let _ = file_analysis_incremental(&db, file86, cfg);
    let _ = compiler_check_diagnostics(&db, file86, cfg);
    assert_eq!(
        count_cu(&log),
        1,
        "tcl8.6: both consumers share exactly one compilation_unit build"
    );

    // `tcl8.4` is the interesting case: its `expand_syntax` differs from
    // `LexerConfig::default()`, so keying on the truncated three-field
    // config (rather than the environment id) would intern two separate
    // entries — one for the analyser tail's default config and one for
    // the checks pass's `for_dialect("tcl8.4")`. Keying on the
    // environment id instead means both consumers lex under the
    // document's own environment, so `tcl8.4` shares one build like
    // every other environment.
    let file84 = SourceFile::new(&db, src.to_owned(), "tcl8.4".to_owned(), None);
    let _ = file_analysis_incremental(&db, file84, cfg);
    let _ = compiler_check_diagnostics(&db, file84, cfg);
    assert_eq!(
        count_cu(&log),
        1,
        "tcl8.4: one environment id -> one shared build, as for every dialect"
    );

    // A fresh edit re-shares for tcl8.6 (one build for the new revision).
    file86
        .set_text(&mut db)
        .to("proc a {x} { return $x }\nproc b {} { a 2 }\n".to_owned());
    let _ = file_analysis_incremental(&db, file86, cfg);
    let _ = compiler_check_diagnostics(&db, file86, cfg);
    assert_eq!(
        count_cu(&log),
        1,
        "after an edit, tcl8.6 again shares one build across both consumers"
    );
}
#[test]
fn fresh_compiler_document_retains_actual_input_and_selected_registry() {
    let db = TclDatabase::default();
    let analysis_config = cfg(&db);
    for dialect in ["tcl", "tcl8.6"] {
        let registry = db.registry(dialect);
        let file = SourceFile::new(&db, "set value 1".to_owned(), dialect.to_owned(), None);
        let input = document_analysis_input(&db, file, analysis_config).unwrap();
        let fresh = fresh_compiler_document_unit(file.text(&db), registry, dialect, None);
        assert_eq!(
            fresh.ir_module.source_metadata_input.as_ref(),
            Some(input.as_ref())
        );
        assert!(fresh.ir_module.source_entry.native_entry.is_none());
        assert!(
            !fresh
                .ir_module
                .source_entry
                .metadata_context
                .is_standalone()
        );
        assert_eq!(
            fresh
                .ir_module
                .source_metadata_input
                .as_ref()
                .unwrap()
                .borrowed_context_registry()
                .commands()
                .snapshot()
                .semantic_key(),
            registry.snapshot().semantic_key()
        );
    }
    let registry = db.registry("tcl8.6");
    let profile = registry.profile().unwrap();
    let mut changed = registry.project_for_profile(profile);
    let mut spec = changed.get("puts").unwrap().clone();
    spec.traits.insert(tcl_registry::Traits::EVALUATES_CODE);
    changed.insert(spec);
    let fresh = fresh_compiler_document_unit("puts VALUE", &changed, "tcl8.6", None);
    assert_eq!(
        fresh
            .ir_module
            .source_metadata_input
            .as_ref()
            .unwrap()
            .borrowed_context_registry()
            .commands()
            .snapshot()
            .semantic_key(),
        changed.snapshot().semantic_key()
    );
    assert_ne!(
        changed.snapshot().semantic_key(),
        registry.snapshot().semantic_key()
    );
}

#[test]
fn compiler_taint_replay_status_is_retained_by_memo_and_direct_paths() {
    let db = TclDatabase::default();
    let source = "set value 1";
    let dialect = "tcl8.6";
    let file = SourceFile::new(&db, source.to_owned(), dialect.to_owned(), None);
    let memo = compiler_check_diagnostics(&db, file, cfg(&db));
    let registry = db.registry(dialect);
    let fresh = compiler_check_diagnostics_uncached(source, registry, dialect, None, None);
    assert_eq!(memo.checks, fresh.checks);
    assert_eq!(memo.taint_replays, fresh.taint_replays);
    let unavailable = abstain_diagnostics();
    assert_eq!(unavailable.checks.len(), 0);
    assert!(!unavailable.taint_replays.is_complete());
    assert!(matches!(
        unavailable.taint_replays,
        tcl_compiler::taint::TaintReplayAnalysis::Unavailable { .. }
    ));
}
