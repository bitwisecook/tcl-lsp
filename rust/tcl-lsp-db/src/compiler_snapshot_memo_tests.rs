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
use tcl_dialect::model::{DialectPoint, Release};

#[test]
fn same_name_body_memos_preserve_the_selected_execution_snapshot() {
    let db = TclDatabase::default();
    let mut keys = Vec::new();
    for release in [Release::JIM_0_80, Release::JIM_0_84] {
        let point = DialectPoint::canonical(release);
        let profile =
            tcl_dialect::DialectProfile::projected_from_point("jim", &[], "Jim", point).intern();
        let registry = CommandRegistry::build_default().project_for_profile(profile);
        let key = ProcBodyKey::new(
            &db,
            "return DONE".to_owned(),
            "::".to_owned(),
            "jim".to_owned(),
            CompilerMemoSnapshot {
                profile: Some(profile.cache_key()),
                registry: registry.snapshot(),
            },
            tcl_lexer::LexerConfig::from_grammar(profile.grammar),
        );
        assert_eq!(
            key.snapshot(&db).profile.unwrap().profile().core_point,
            Some(point)
        );
        assert_eq!(
            key.snapshot(&db)
                .registry
                .registry()
                .profile()
                .unwrap()
                .core_point,
            Some(point)
        );
        let body = lower_proc_body(&db, key);
        let dialect = body
            .statements
            .iter()
            .filter_map(|statement| statement.tokens())
            .filter_map(|tokens| tokens.source_binding.as_ref())
            .find_map(|binding| binding.variable_context.invocation_dialect)
            .unwrap();
        assert_eq!(dialect.core_point, Some(point));
        keys.push(key);
    }
    assert!(
        keys[0] != keys[1],
        "distinct selected profiles must intern different body keys"
    );
}

#[test]
fn body_memo_registry_identity_preserves_authored_overrides_and_pins() {
    let db = TclDatabase::default();
    let profile = tcl_dialect::DialectProfile::find("tcl8.6").unwrap();
    let mut registry = CommandRegistry::build_default().project_for_profile(profile);
    let make_key = |snapshot| {
        ProcBodyKey::new(
            &db,
            "return DONE".to_owned(),
            "::".to_owned(),
            "tcl8.6".to_owned(),
            CompilerMemoSnapshot {
                profile: Some(profile.cache_key()),
                registry: snapshot,
            },
            tcl_lexer::LexerConfig::from_grammar(profile.grammar),
        )
    };
    let original = make_key(registry.snapshot());
    registry.insert_ambient_package("MemoPackage", "1.0");
    let changed = make_key(registry.snapshot());
    assert!(
        original != changed,
        "ambient pins participate in memo equality"
    );
    assert!(
        !original
            .snapshot(&db)
            .registry
            .registry()
            .ambient_package_rows()
            .contains(&("MemoPackage", "1.0"))
    );
    assert!(
        changed
            .snapshot(&db)
            .registry
            .registry()
            .ambient_package_rows()
            .contains(&("MemoPackage", "1.0"))
    );
    let mut spec = registry.get("set").unwrap().clone();
    spec.traits = tcl_registry::Traits::CSE_CANDIDATE;
    registry.insert(spec);
    let authored = make_key(registry.snapshot());
    assert!(
        changed != authored,
        "authored overrides participate in memo equality"
    );
    assert_eq!(
        authored
            .snapshot(&db)
            .registry
            .registry()
            .get("set")
            .unwrap()
            .traits,
        tcl_registry::Traits::CSE_CANDIDATE
    );
}
#[test]
fn original_callback_alias_diagnostics_keep_target_and_separate_counts() {
    // naming.source.original-callback-procedure-target
    // docs/design/analysis/name-resolution-proofs/source-original-callback-procedure-target.md
    let db = TclDatabase::default();
    let source = "proc target {a b c} {}\ninterp alias {} cb {} target FIXED\nlsort -command {cb BAKED} {2 1}";
    let file = SourceFile::new(&db, source.to_owned(), "tcl9.0".to_owned(), None);
    let project = Project::new(&db, vec![file]);
    let analysis = tcl_compiler::analyser::Analyser::new().analyse(source, "tcl9.0");
    let mut invocations = analysis.command_invocations;
    for row in &mut invocations {
        if row
            .original_callback_signature_lookup
            .as_ref()
            .is_some_and(|selection| {
                selection.prefix().appended_arity() == Some(tcl_registry::AppendedArity::Exactly(2))
            })
        {
            row.original_callback_prefix = None;
            row.original_lookup = None;
            row.original_name_input = None;
            row.name = "unrelated presentation".to_owned();
            row.range = tcl_lexer::Span::new(0, 1);
            row.callback_arity = Some(tcl_registry::AppendedArity::Unknown);
            row.callback_baked_args = usize::MAX;
        }
    }
    let mut output = Vec::new();
    apply_original_callback_arity(&db, project, &mut output, &invocations, |_| false);
    let diagnostic = output
        .iter()
        .find(|diagnostic| {
            diagnostic.callback_source_arity().is_some_and(|subject| {
                subject.prefix().appended_arity() == Some(tcl_registry::AppendedArity::Exactly(2))
            })
        })
        .unwrap();
    assert_eq!(diagnostic.code, DiagCode::E003);
    let subject = diagnostic.callback_source_arity().unwrap();
    assert_eq!(subject.prefix().baked_argument_count(), 1);
    assert_eq!(
        subject
            .source_lookup()
            .unwrap()
            .captured_argument_count()
            .unwrap()
            .minimum,
        1
    );
    assert_eq!(
        subject.argument_counts(),
        &tcl_compiler::analyser::SourceCallbackArgumentCounts::Finite(vec![4])
    );
    assert_eq!(
        subject.declarations()[0].name().slot().simple.as_bytes(),
        b"target"
    );
    assert_eq!(&source[diagnostic.span.as_range()], "cb");
    assert!(
        invocations
            .iter()
            .all(|row| row.original_callback_signature_lookup.is_none()
                || row.logical_callback_candidate_keys().is_empty())
    );
}

#[test]
fn original_callback_alias_external_headers_use_held_target_slot() {
    // naming.source.original-callback-procedure-target
    // docs/design/analysis/name-resolution-proofs/source-original-callback-procedure-target.md
    let db = TclDatabase::default();
    let library = SourceFile::new(
        &db,
        "proc external {a b c d} {}\nproc cb {a b c} {}".to_owned(),
        "tcl9.0".to_owned(),
        None,
    );
    let source = "interp alias {} cb {} external FIXED\nlsort -command cb {2 1}";
    let caller = SourceFile::new(&db, source.to_owned(), "tcl9.0".to_owned(), None);
    let project = Project::new(&db, vec![library, caller]);
    let analysis = tcl_compiler::analyser::Analyser::new().analyse(source, "tcl9.0");
    let mut output = Vec::new();
    apply_original_callback_arity(
        &db,
        project,
        &mut output,
        &analysis.command_invocations,
        |_| false,
    );
    let subject = output
        .iter()
        .filter_map(|diagnostic| diagnostic.callback_source_arity())
        .find(|subject| {
            subject.prefix().appended_arity() == Some(tcl_registry::AppendedArity::Exactly(2))
        })
        .unwrap();
    assert_eq!(
        subject.issue(),
        tcl_compiler::analyser::SourceCallbackArityIssue::TooFew {
            supplied: 3,
            expected_minimum: 4
        }
    );
    assert_eq!(subject.declarations().len(), 1);
    assert_eq!(
        subject.declarations()[0].name().slot().simple.as_bytes(),
        b"external"
    );
    assert_eq!(subject.source_lookup().unwrap().original().target().unwrap().kind(), tcl_compiler::command_binding::OriginalSourceCallbackProcedureTargetKind::ExternalSourceName);
}

#[test]
fn original_callback_alias_database_refuses_deleted_and_unowned_horizons() {
    // naming.source.original-callback-procedure-target
    // docs/design/analysis/name-resolution-proofs/source-original-callback-procedure-target.md
    let db = TclDatabase::default();
    let source = "proc cb {a b c} {}\nrename cb {}\nlsort -command cb {2 1}";
    let file = SourceFile::new(&db, source.to_owned(), "tcl9.0".to_owned(), None);
    let project = Project::new(&db, vec![file]);
    let analysis = tcl_compiler::analyser::Analyser::new().analyse(source, "tcl9.0");
    let mut output = Vec::new();
    apply_original_callback_arity(
        &db,
        project,
        &mut output,
        &analysis.command_invocations,
        |_| false,
    );
    assert!(
        output
            .iter()
            .all(|diagnostic| diagnostic.callback_source_arity().is_none())
    );
    let source = "proc cb {a b c} {}\nlsort -command cb {2 1}";
    let mut analysis = tcl_compiler::analyser::Analyser::new().analyse(source, "tcl9.0");
    for row in &mut analysis.command_invocations {
        row.original_callback_signature_lookup = None;
    }
    output.clear();
    apply_original_callback_arity(
        &db,
        project,
        &mut output,
        &analysis.command_invocations,
        |_| false,
    );
    assert!(
        output.is_empty(),
        "lookup geometry without a source horizon cannot borrow a project header"
    );
}
