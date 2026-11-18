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

use std::collections::{HashMap, HashSet};

use super::*;
use crate::diagnostic_policy::{Directives, Policy, PolicyBuilder, PolicyLayer, apply};
use tcl_compiler::analyser::{Analyser, AnalysisResult, CodeFix, Diagnostic};
use tcl_lexer::Span;

#[test]
fn context_action_deduplication_keeps_distinct_operations() {
    let edit = crate::rename::TextEdit {
        range: LspRange {
            start_line: 1,
            start_character: 0,
            end_line: 1,
            end_character: 5,
        },
        new_text: " -nocommands".to_owned(),
    };
    let first = context_action("Harden subst".to_owned(), vec![edit.clone()]);
    let mut second = first.clone();
    second.edits[0].range.start_line = 2;
    second.edits[0].range.end_line = 2;
    let mut actions = vec![first.clone(), second.clone()];
    deduplicate_context_actions(&mut actions);
    assert_eq!(actions, [first.clone(), second.clone()]);

    let mut relabelled = first.clone();
    relabelled.title = "Updated presentation".to_owned();
    let mut actions = vec![first.clone(), relabelled];
    deduplicate_context_actions(&mut actions);
    assert_eq!(actions, std::slice::from_ref(&first));

    let mut different_kind = first.clone();
    different_kind.kind = ActionKind::RefactorRewrite;
    let mut different_command = first.clone();
    different_command.command = Some(ActionCommand {
        command: "tclLsp.renameSymbolAtPosition".to_owned(),
        args: vec![1, 0, 5],
        string_args: Vec::new(),
    });
    let mut different_data = first.clone();
    different_data.data_group_definition = Some("definition".to_owned());
    let mut actions = vec![
        first.clone(),
        different_kind.clone(),
        different_command.clone(),
        different_data.clone(),
    ];
    deduplicate_context_actions(&mut actions);
    assert_eq!(
        actions,
        [first, different_kind, different_command, different_data]
    );

    let mut unavailable = context_action("Unavailable".to_owned(), Vec::new());
    unavailable.disabled = Some("source authority unavailable".to_owned());
    let mut actions = vec![unavailable.clone(), unavailable.clone()];
    deduplicate_context_actions(&mut actions);
    assert_eq!(actions, [unavailable.clone(), unavailable]);
}

#[test]
fn original_source_action_metadata_uses_retained_availability_over_report_profile() {
    // naming.consumer.original-diagnostic-source-actions
    // docs/design/analysis/name-resolution-proofs/original-diagnostic-source-actions.md
    // Exact metadata admission, without a Native slot/handler/effect claim.
    let catalogue = tcl_registry::model::ingress::static_context_for("tcl8.6");
    let profile = tcl_dialect::DialectProfile::find("tcl8.6").unwrap();
    for (environment, available) in [("tcl8.6", true), ("tcl8.4", false)] {
        let context = std::sync::Arc::new(
            tcl_registry::model::ingress::static_context_for(environment)
                .with_command_store(std::sync::Arc::clone(catalogue.commands())),
        );
        assert!(std::sync::Arc::ptr_eq(
            context.commands(),
            catalogue.commands()
        ));
        let input = tcl_compiler::analyser::ResolvedAnalysisInput::new(
            profile,
            profile,
            context,
            tcl_lexer::LexerConfig::for_profile(Some(profile)),
        );
        let mut analysis = Analyser::new()
            .with_resolved_input(input)
            .analyse("", profile.name);
        analysis.dialect = "tcl9.1".to_owned();
        assert_eq!(
            source_action_spec(&analysis, catalogue.commands(), "dict").is_some(),
            available,
            "same store and profile, retained {environment} availability",
        );
        analysis.resolved_input = None;
        assert!(source_action_spec(&analysis, catalogue.commands(), "dict").is_none());
    }
}

fn whole_document_range(source: &str) -> LspRange {
    let line_count = source.lines().count().max(1);
    LspRange {
        start_line: 0,
        start_character: 0,
        end_line: u32::try_from(line_count - 1).unwrap_or(0),
        end_character: u32::MAX,
    }
}

#[test]
fn empty_actions_when_analysis_is_none() {
    assert_eq!(
        code_actions(
            "set x 1\n",
            whole_document_range("set x 1\n"),
            None,
            &Report::default()
        ),
        [] as [crate::code_actions::CodeAction; 0]
    );
}

#[test]
fn fix_attached_to_diagnostic_surfaces_as_action() {
    // A current source analysis carries the isolated fix-lifting fixture.
    // This checks transport; semantic fix permission remains its issuer's.
    let mut r = Analyser::new().analyse("set x 1\n", "tcl8.6");
    r.diagnostics.clear();
    r.diagnostics.push(Diagnostic {
        subject: None,
        code: DiagCode::W210,
        message: "Variable read before set".to_string(),
        severity: tcl_compiler::analyser::Severity::Warning,
        span: Span::new(0, 5),
        fixes: vec![CodeFix {
            span: Span::new(0, 5),
            new_text: "set var 0".to_string(),
            description: "Initialise `var`".to_string(),
            safety: tcl_compiler::analyser::FixSafety::RequiresReview,
        }],
    });
    let actions = code_actions(
        "set x 1\n",
        whole_document_range("set x 1\n"),
        Some(&r),
        &report_of(&r.diagnostics),
    );
    let qf: Vec<&CodeAction> = actions
        .iter()
        .filter(|a| a.kind == ActionKind::QuickFix)
        .collect();
    assert_eq!(qf.len(), 1, "{actions:?}");
    assert_eq!(qf[0].title, "Initialise `var`");
    assert_eq!(qf[0].edits.len(), 1);
    assert_eq!(qf[0].edits[0].new_text, "set var 0");
}

#[test]
fn no_action_when_range_outside_diagnostic() {
    let mut r = Analyser::new().analyse("set x 1\n", "tcl8.6");
    r.diagnostics.clear();
    r.diagnostics.push(Diagnostic {
        subject: None,
        code: DiagCode::W210,
        message: "msg".to_string(),
        severity: tcl_compiler::analyser::Severity::Warning,
        span: Span::new(0, 5),
        fixes: vec![CodeFix {
            span: Span::new(0, 5),
            new_text: "fix".to_string(),
            description: "Fix".to_string(),
            safety: tcl_compiler::analyser::FixSafety::RequiresReview,
        }],
    });
    // Request range on line 99 — far away from the
    // diagnostic's line 0.
    let far_range = LspRange {
        start_line: 99,
        start_character: 0,
        end_line: 99,
        end_character: 10,
    };
    assert_eq!(
        code_actions("set x 1\n", far_range, Some(&r), &report_of(&r.diagnostics)),
        [] as [crate::code_actions::CodeAction; 0]
    );
}

#[test]
fn empty_description_falls_back_to_diagnostic_message() {
    let mut r = Analyser::new().analyse("set x 1\n", "tcl8.6");
    r.diagnostics.clear();
    r.diagnostics.push(Diagnostic {
        subject: None,
        code: DiagCode::W210,
        message: "Variable read before set".to_string(),
        severity: tcl_compiler::analyser::Severity::Warning,
        span: Span::new(0, 5),
        fixes: vec![CodeFix {
            span: Span::new(0, 5),
            new_text: "x".to_string(),
            description: String::new(), // No description.
            safety: tcl_compiler::analyser::FixSafety::RequiresReview,
        }],
    });
    let actions = code_actions(
        "set x 1\n",
        whole_document_range("set x 1\n"),
        Some(&r),
        &report_of(&r.diagnostics),
    );
    let qf: Vec<&CodeAction> = actions
        .iter()
        .filter(|a| a.kind == ActionKind::QuickFix)
        .collect();
    assert_eq!(qf.len(), 1);
    assert!(qf[0].title.contains("Variable read before set"));
}

#[test]
fn no_actions_when_analyser_has_no_diagnostics_with_fixes() {
    // Run the actual analyser; with a clean source no
    // fixable diagnostics fire and the result is empty.
    let mut a = Analyser::new();
    let analysis = a.analyse("set x 1\nputs $x\n", "tcl8.6").clone();
    let actions = code_actions(
        "set x 1\nputs $x\n",
        whole_document_range("set x 1\nputs $x\n"),
        Some(&analysis),
        &report_of(&analysis.diagnostics),
    );
    // No diagnostic fixes → no quick-fix actions (range-based refactors
    // like extract-proc may still be offered for the selection).
    assert!(
        !actions.iter().any(|a| a.kind == ActionKind::QuickFix),
        "{actions:?}",
    );
}

#[test]
fn multiple_fixes_on_one_diagnostic_each_become_an_action() {
    let mut r = Analyser::new().analyse("set x 1\n", "tcl8.6");
    r.diagnostics.clear();
    r.diagnostics.push(Diagnostic {
        subject: None,
        code: DiagCode::W210,
        message: "msg".to_string(),
        severity: tcl_compiler::analyser::Severity::Warning,
        span: Span::new(0, 5),
        fixes: vec![
            CodeFix {
                span: Span::new(0, 5),
                new_text: "a".into(),
                description: "A".into(),
                safety: tcl_compiler::analyser::FixSafety::RequiresReview,
            },
            CodeFix {
                span: Span::new(0, 5),
                new_text: "b".into(),
                description: "B".into(),
                safety: tcl_compiler::analyser::FixSafety::RequiresReview,
            },
        ],
    });
    let actions = code_actions(
        "set x 1\n",
        whole_document_range("set x 1\n"),
        Some(&r),
        &report_of(&r.diagnostics),
    );
    let titles: Vec<&str> = actions
        .iter()
        .filter(|a| a.kind == ActionKind::QuickFix)
        .map(|a| a.title.as_str())
        .collect();
    assert_eq!(titles.len(), 2);
    assert!(titles.contains(&"A") && titles.contains(&"B"));
}

// refactor.inline — proc inlining (issue 179)

fn line_range(line: u32) -> LspRange {
    LspRange {
        start_line: line,
        start_character: 0,
        end_line: line,
        end_character: 0,
    }
}

/// The inline-proc action's replacement text, or its refusal reason.
fn inline_outcome(src: &str, call_line: u32) -> Result<String, String> {
    let mut analyser = Analyser::new();
    let analysis = analyser.analyse(src, "tcl8.6").clone();
    let actions = code_actions(
        src,
        line_range(call_line),
        Some(&analysis),
        &report_of(&analysis.diagnostics),
    );
    let action = actions
        .iter()
        .find(|a| a.title.starts_with("Inline proc "))
        .unwrap_or_else(|| panic!("expected an inline action in {actions:?}"));
    match &action.disabled {
        Some(reason) => Err(reason.clone()),
        None => Ok(action.edits[0].new_text.clone()),
    }
}

#[test]
fn fp_inline_refuses_a_braced_argument_that_is_not_a_plain_word() {
    // `f {a b}` passes the *value* `a b`, not the four characters
    // `{a b}`.  Splicing the written word makes the body print the
    // braces, so the transform declines and says why.
    let src = "proc f {p} { puts $p }\nf {a b}\n";
    let reason = inline_outcome(src, 1).unwrap_err();
    assert!(reason.contains("inert literal"), "{reason}");
}

#[test]
fn tp_inline_does_not_substitute_prefix_sharing_name() {
    // Issue 179: a naive `replace("$n", …)` corrupts `$nn`. tclsh 8.6.18 and
    // 9.0.4: a free `$nn` is the proc's unset local, so the call fails and
    // inlining refuses; as a second parameter each reference binds its own.
    let src = "proc g {n} { puts $nn$n }\ng 5\n";
    let reason = inline_outcome(src, 1).unwrap_err();
    assert!(!reason.is_empty());
    let src = "proc g {n nn} { list $nn $n }\ng 5 6\n";
    assert_eq!(inline_outcome(src, 1).unwrap(), "list 6 5");
}

#[test]
fn tp_inline_substitutes_braced_var_reference() {
    // tclsh 8.6.18 and 9.0.4: a free `${pq}` fails in the proc but would read
    // the caller's variable once inlined; bound, `${p}${pq}` gives 98.
    let src = "proc h {p} { puts ${p}${pq} }\nh 9\n";
    let reason = inline_outcome(src, 1).unwrap_err();
    assert!(!reason.is_empty());
    let src = "proc h {p pq} { puts ${p}${pq} }\nh 9 8\n";
    assert_eq!(inline_outcome(src, 1).unwrap(), "puts 98");
}

// W213 unset -nocomplain action

#[test]
fn w213_emits_unset_nocomplain_action() {
    // Confirm the analyser emits W213 on `unset xs` inside a
    // proc where `xs` is possibly undefined, then verify the
    // provider surfaces the `-nocomplain` quick-fix.
    let src = "proc foo {} { unset xs }\n";
    let mut a = Analyser::new();
    let analysis = a.analyse(src, "tcl8.6").clone();
    assert!(
        analysis
            .diagnostics
            .iter()
            .any(|d| d.code == DiagCode::W213),
        "expected W213 from {:?}",
        analysis.diagnostics,
    );
    let actions = code_actions(
        src,
        whole_document_range(src),
        Some(&analysis),
        &report_of(&analysis.diagnostics),
    );
    let nocomplain = actions
        .iter()
        .find(|a| a.title == "Add '-nocomplain' to unset");
    assert!(nocomplain.is_some(), "expected quick-fix in {actions:?}");
    let act = nocomplain.unwrap();
    assert_eq!(act.edits.len(), 1);
    assert_eq!(act.edits[0].new_text, " -nocomplain");
    // The edit is an insertion (zero-width range).
    assert_eq!(
        act.edits[0].range.start_character,
        act.edits[0].range.end_character,
    );
}

#[test]
fn w213_action_inserts_after_unset_keyword() {
    // Verify the insertion point is exactly after the 5
    // chars of `unset` — splicing produces a syntactically
    // correct `unset -nocomplain xs` command.
    let src = "proc foo {} { unset xs }\n";
    let mut a = Analyser::new();
    let analysis = a.analyse(src, "tcl8.6").clone();
    let actions = code_actions(
        src,
        whole_document_range(src),
        Some(&analysis),
        &report_of(&analysis.diagnostics),
    );
    let act = actions
        .iter()
        .find(|a| a.title == "Add '-nocomplain' to unset")
        .expect("expected unset action");
    // Apply the edit and check the result.
    let edit = &act.edits[0];
    let line0 = src.lines().nth(edit.range.start_line as usize).unwrap();
    let chars: Vec<char> = line0.chars().collect();
    let col = edit.range.start_character as usize;
    let before: String = chars[..col].iter().collect();
    let after: String = chars[col..].iter().collect();
    let spliced = format!("{before}{}{after}", edit.new_text);
    assert!(
        spliced.contains("unset -nocomplain xs"),
        "spliced line: {spliced}",
    );
}

// catch-result-variable actions

/// Apply a single-edit action's `TextEdit` to `src` and return the
/// rewritten document.
///
/// Every catch-fix test below asserts the *applied document* rather than
/// the inserted string plus a zero-width range: an insertion of exactly
/// the right text at exactly the wrong place satisfies both of those
/// weaker assertions.
fn apply_single_edit(src: &str, action: &CodeAction) -> String {
    assert_eq!(action.edits.len(), 1, "expected one edit: {action:?}");
    let edit = &action.edits[0];
    let line_index = LineIndex::new(src);
    let start = line_index.offset_at_utf16(
        edit.range.start_line,
        Utf16Col::new(edit.range.start_character),
        src,
    ) as usize;
    let end = line_index.offset_at_utf16(
        edit.range.end_line,
        Utf16Col::new(edit.range.end_character),
        src,
    ) as usize;
    let mut out = src.to_string();
    out.replace_range(start..end, &edit.new_text);
    out
}

/// The quick-fix actions the provider offers for the W302 in `src`.
fn catch_result_actions(src: &str) -> Vec<CodeAction> {
    let mut analyser = Analyser::new();
    let analysis = analyser.analyse(src, "tcl9.0").clone();
    assert!(
        analysis
            .diagnostics
            .iter()
            .any(|d| d.code == DiagCode::W302),
        "expected W302 from {:?}",
        analysis.diagnostics,
    );
    code_actions(
        src,
        whole_document_range(src),
        Some(&analysis),
        &report_of(&analysis.diagnostics),
    )
    .into_iter()
    .filter(|action| action.title.starts_with("Add catch"))
    .collect()
}

#[test]
fn w302_result_action_applies_after_the_body() {
    // The diagnostic anchors at the `catch` word, so a provider that
    // reconstructs the insertion point from the diagnostic's end
    // produces `catch result { puts hi }` — a catch of the script
    // `result`.
    let src = "catch { puts hi }\n";
    let actions = catch_result_actions(src);
    assert_eq!(actions.len(), 2, "{actions:?}");
    assert_eq!(
        apply_single_edit(src, &actions[0]),
        "catch { puts hi } result\n"
    );
}

#[test]
fn w302_options_action_applies_after_the_body() {
    let src = "catch { puts hi }\n";
    let actions = catch_result_actions(src);
    assert_eq!(
        apply_single_edit(src, &actions[1]),
        "catch { puts hi } result options\n"
    );
}

#[test]
fn w302_action_applies_after_a_multiline_body() {
    let src = "catch {\n    error oops\n}\n";
    let actions = catch_result_actions(src);
    assert_eq!(
        apply_single_edit(src, &actions[0]),
        "catch {\n    error oops\n} result\n"
    );
}

#[test]
fn w302_action_applies_before_a_trailing_comment() {
    let src = "catch {error oops} ;# best effort\n";
    let actions = catch_result_actions(src);
    assert_eq!(
        apply_single_edit(src, &actions[0]),
        "catch {error oops} result ;# best effort\n"
    );
}

#[test]
fn w302_action_does_not_overshoot_an_empty_body() {
    let src = "catch {}\n";
    let actions = catch_result_actions(src);
    assert_eq!(apply_single_edit(src, &actions[0]), "catch {} result\n");
}

#[test]
fn w302_action_applies_to_a_nested_catch_on_a_later_line() {
    // A catch that is neither on line 0 nor at column 0 — the anchor is
    // an absolute offset from the argument token, not a line-relative
    // guess.
    let src = "proc f {} {\n    catch {error oops}\n}\n";
    let actions = catch_result_actions(src);
    assert_eq!(
        apply_single_edit(src, &actions[0]),
        "proc f {} {\n    catch {error oops} result\n}\n"
    );
}

// W120 package-require fix

#[test]
fn w120_surfaces_add_package_require_action() {
    // The analyser emits W120 (with a CodeFix) for a
    // package-gated command used without `package require`;
    // the code-actions provider lifts that fix into an
    // `Add 'package require ...'` action.
    let src = "tcl::idna decode example.com\n";
    let mut a = Analyser::new();
    let analysis = a.analyse(src, "tcl9.0").clone();
    assert!(
        analysis
            .diagnostics
            .iter()
            .any(|d| d.code == DiagCode::W120),
        "expected W120 from {:?}",
        analysis.diagnostics,
    );
    let actions = code_actions(
        src,
        whole_document_range(src),
        Some(&analysis),
        &report_of(&analysis.diagnostics),
    );
    let add = actions
        .iter()
        .find(|a| a.title.starts_with("Add 'package require"));
    assert!(
        add.is_some(),
        "expected package-require action in {actions:?}"
    );
    let act = add.unwrap();
    assert_eq!(act.edits.len(), 1);
    assert_eq!(act.edits[0].new_text, "package require tcl::idna\n");
    // Insertion at the top of the file (line 0, col 0).
    assert_eq!(act.edits[0].range.start_line, 0);
    assert_eq!(act.edits[0].range.start_character, 0);
}

#[test]
fn package_require_action_inserts_between_original_terminated_commands() {
    let cases = [
        (
            "package require existing; http::get /resource\n",
            "package require existing;package require http\n http::get /resource\n",
        ),
        (
            "package require existing\nhttp::get /resource\n",
            "package require existing\npackage require http\nhttp::get /resource\n",
        ),
        (
            "package require existing; # retained comment\nhttp::get /resource\n",
            "package require existing;package require http\n # retained comment\nhttp::get /resource\n",
        ),
        (
            "package \\\n require existing\nhttp::get /resource\n",
            "package \\\n require existing\npackage require http\nhttp::get /resource\n",
        ),
    ];
    for (source, expected) in cases {
        let cursor = u32::try_from(source.find("http::get").unwrap()).unwrap();
        let position = LineIndex::new(source).position_at_utf16(cursor, source);
        let actions = package_actions(source, at(position.line, position.character.get()));
        let action = actions
            .iter()
            .find(|action| action.title == "Add 'package require http'")
            .expect("the original unresolved namespaced head supports the package action");
        assert_eq!(apply_single_edit(source, action), expected, "{source:?}");
    }
}

fn at(line: u32, character: u32) -> LspRange {
    LspRange {
        start_line: line,
        start_character: character,
        end_line: line,
        end_character: character,
    }
}

// Fuzzy `package require` suggestions.
//
// Applying one of these mutates package loading and runs the package's
// initialisation code, so the provider must have evidence a package is
// actually missing.  The two gates it applies are "the cursor is on a
// recorded command *head*" and "resolution says that head is unresolved".
// The cases below are the four coverage classes the issue asks for.

/// The package-require actions for `src` at `range`, driven by a real
/// analysis (the evidence both gates read) and no editor-supplied
/// diagnostics.
fn package_actions(src: &str, range: LspRange) -> Vec<CodeAction> {
    let registry = tcl_registry::CommandRegistry::build_default();
    let mut analyser = Analyser::new();
    let analysis = analyser.analyse(src, "tcl9.0").clone();
    package_require_actions(src, range, &registry, Some(&analysis), &[])
}

/// The titles of the package-require actions for `src` at `range`.
fn package_titles(src: &str, range: LspRange) -> Vec<String> {
    package_actions(src, range)
        .into_iter()
        .map(|action| action.title)
        .collect()
}

#[test]
fn tp_package_require_offered_on_an_unresolved_command_head() {
    // The one shape that is real evidence: an executable command head
    // the resolver could not satisfy, whose leading namespace exactly
    // names a package the registry knows.
    let src = "http::foo $x\n";
    let actions = package_actions(src, at(0, 2));
    assert_eq!(
        actions.iter().map(|a| a.title.as_str()).collect::<Vec<_>>(),
        vec!["Add 'package require http'"],
        "{actions:?}"
    );
    assert_eq!(actions[0].edits[0].new_text, "package require http\n");
    assert_eq!(actions[0].edits[0].range.start_line, 0);
}

#[test]
fn fn_package_require_offered_on_a_fully_qualified_head() {
    // A leading `::` is how library code writes a call unambiguously.
    // Splitting on `::` without stripping it first yields an empty
    // namespace component, which matches nothing at all.
    let titles = package_titles("::http::foo $x\n", at(0, 4));
    assert!(
        titles.iter().any(|t| t == "Add 'package require http'"),
        "{titles:?}"
    );
}

#[test]
fn tn_package_require_not_offered_for_a_bare_unknown_command() {
    // `frobnicate` is unresolved, but its name carries no evidence about
    // which package would define it. Guessing from a textual resemblance
    // is exactly the behaviour the namespace gate replaced.
    assert_eq!(
        package_titles("frobnicate 1\n", at(0, 2)),
        [] as [std::string::String; 0]
    );
}

#[test]
fn tn_package_require_not_offered_for_an_unregistered_namespace() {
    // `myapp` names no package in the registry catalogue, so there is
    // nothing evidence-backed to suggest — a project's own namespace
    // must never be read as a missing dependency.
    assert_eq!(
        package_titles("myapp::helper x\n", at(0, 2)),
        [] as [std::string::String; 0]
    );
}

#[test]
fn tn_package_require_not_offered_under_a_dynamic_provider() {
    // A computed `package require` may register the command at run time,
    // which is the same reason W123 stands down in such a file.
    let src = "set p http\npackage require $p\nhttp::foo\n";
    assert_eq!(
        package_titles(src, at(2, 2)),
        [] as [std::string::String; 0]
    );
}

#[test]
fn fp_package_require_not_offered_inside_a_comment() {
    let src = "# Documentation: http::geturl\n";
    assert_eq!(
        package_titles(src, at(0, 20)),
        [] as [std::string::String; 0]
    );
}

#[test]
fn fp_package_require_not_offered_inside_a_quoted_string() {
    let src = "set example \"http::geturl\"\n";
    assert_eq!(
        package_titles(src, at(0, 16)),
        [] as [std::string::String; 0]
    );
}

#[test]
fn fp_package_require_not_offered_inside_a_braced_word() {
    let src = "set example {http::geturl}\n";
    assert_eq!(
        package_titles(src, at(0, 16)),
        [] as [std::string::String; 0]
    );
}

#[test]
fn fp_package_require_not_offered_on_an_argument_word() {
    // `http::geturl` here is data passed to `dict set`, not a call.
    let src = "dict set docs command http::geturl\n";
    assert_eq!(
        package_titles(src, at(0, 25)),
        [] as [std::string::String; 0]
    );
}

#[test]
fn fp_package_require_not_offered_on_a_definition_name() {
    // The name word of a `proc` is a definition, not an invocation — and
    // the file is defining the very command a package would provide.
    let src = "proc http::geturl {} {}\n";
    assert_eq!(
        package_titles(src, at(0, 8)),
        [] as [std::string::String; 0]
    );
}

#[test]
fn fp_package_require_not_offered_for_a_locally_defined_command() {
    // Defined above, called below: resolution settles the call, so no
    // unknown-command diagnostic covers the head and no package is
    // suggested for it.
    let src = "proc http::geturl {} {}\nhttp::geturl\n";
    assert_eq!(
        package_titles(src, at(1, 2)),
        [] as [std::string::String; 0]
    );
}

#[test]
fn fp_package_require_not_offered_when_the_package_is_already_required() {
    let src = "package require http\nhttp::foo\n";
    assert!(
        !package_titles(src, at(1, 2))
            .iter()
            .any(|t| t.contains("require http'")),
        "http is already required"
    );
}

#[test]
fn tn_package_require_not_offered_for_a_resolved_builtin() {
    // `puts` resolves in the registry, so nothing is unresolved here.
    assert_eq!(
        package_titles("puts hello\n", at(0, 1)),
        [] as [std::string::String; 0]
    );
}

#[test]
fn tn_package_require_not_offered_for_a_dynamic_head() {
    // `$cmd` is recorded as an invocation, but its written name is not
    // the command that will run, so matching it against a package
    // catalogue is meaningless.
    assert_eq!(
        package_titles("set cmd http::geturl\n$cmd\n", at(1, 1)),
        [] as [std::string::String; 0]
    );
}

#[test]
fn tn_package_require_not_offered_for_a_short_prefix() {
    assert_eq!(
        package_titles("x::y\n", at(0, 0)),
        [] as [std::string::String; 0]
    );
}

#[test]
fn tn_package_require_not_offered_without_an_analysis() {
    // No analysis, no evidence — the provider declines rather than
    // falling back to scanning the text.
    let registry = tcl_registry::CommandRegistry::build_default();
    assert_eq!(
        package_require_actions("http::foo\n", at(0, 2), &registry, None, &[]),
        [] as [crate::code_actions::CodeAction; 0]
    );
}

#[test]
fn package_require_offered_from_an_editor_supplied_diagnostic() {
    // A textual editor diagnostic carries no original issuer. The
    // genuine analysis subject, when present, independently supports
    // the suggestion; removing it must leave no native fallback.
    let src = "http::foo $x\n";
    let registry = tcl_registry::CommandRegistry::build_default();
    let mut analyser = Analyser::new();
    let analysis = analyser.analyse(src, "tcl9.0").clone();
    let context = vec![ContextDiagnostic {
        data: None,
        code: "W123".to_string(),
        message: "Unknown command 'http::foo'".to_string(),
        range: at(0, 0),
    }];
    let actions = package_require_actions(src, at(0, 2), &registry, Some(&analysis), &context);
    assert!(
        actions
            .iter()
            .any(|a| a.title == "Add 'package require http'"),
        "{actions:?}"
    );
}

#[test]
fn package_named_by_namespace_matches_exactly() {
    let catalogue = vec!["http".to_string(), "json".to_string()];
    assert_eq!(
        package_named_by_namespace("http::get", &catalogue),
        Some("http".to_string())
    );
    assert_eq!(
        package_named_by_namespace("::http::get", &catalogue),
        Some("http".to_string()),
        "a leading `::` must not swallow the namespace component"
    );
    // A near-miss is not evidence: `httpd::start` names the `httpd`
    // namespace, which is not the `http` package.
    assert_eq!(package_named_by_namespace("httpd::start", &catalogue), None);
    // Nor is a bare name, which has no namespace at all.
    assert_eq!(package_named_by_namespace("httpget", &catalogue), None);
}

#[test]
fn logical_package_hint_keeps_shared_written_colon_geometry() {
    // naming.core.original-package-source-action-context
    // docs/design/analysis/name-resolution-proofs/core-original-package-source-action-context.md
    // Authored catalogue hints only; no package loader or callable identity.
    let catalogue = vec!["http".to_owned(), ":http".to_owned()];
    for written in [
        "http::get",
        "http:::get",
        "::http::get",
        ":::http::get",
        ":::::http:::get",
        "http::",
    ] {
        assert_eq!(
            package_named_by_namespace(written, &catalogue),
            Some("http".to_owned()),
            "{written}"
        );
    }
    assert_eq!(
        package_named_by_namespace(":http::get", &catalogue),
        Some(":http".to_owned())
    );
    for written in ["http", "::http", ":::http", ":::", "http:get", "httpd::get"] {
        assert_eq!(
            package_named_by_namespace(written, &catalogue),
            None,
            "{written}"
        );
    }
}

/// A document with no `# noqa` and no file-level directive.
fn no_suppression() -> HashMap<i32, HashSet<String>> {
    HashMap::new()
}

fn check_analysis(
    source: &str,
    registry: &tcl_registry::CommandRegistry,
    profile: &'static tcl_dialect::DialectProfile,
    config: tcl_lexer::LexerConfig,
) -> AnalysisResult {
    let context = crate::context_for_dialect_profile(profile)
        .with_command_store(registry.snapshot().shared_registry());
    Analyser::new()
        .with_resolved_input(tcl_compiler::analyser::ResolvedAnalysisInput::new(
            profile,
            profile,
            std::sync::Arc::new(context),
            config,
        ))
        .analyse(source, profile.name)
}

fn report_of(diagnostics: &[Diagnostic]) -> crate::diagnostic_policy::Report {
    apply(
        diagnostics
            .iter()
            .cloned()
            .map(crate::diagnostic_policy::Finding::from)
            .collect(),
        &Policy::unrestricted(),
    )
}
fn check_actions(
    current: &DiagnosticEditSource<'_>,
    range: LspRange,
    checks: &[tcl_compiler::compiler_checks::Diagnostic],
    disabled: &HashSet<String>,
    suppressed: &HashMap<i32, HashSet<String>>,
) -> Vec<CodeAction> {
    let layer: serde_json::Map<String, serde_json::Value> = disabled
        .iter()
        .map(|code| (code.clone(), serde_json::Value::Bool(false)))
        .collect();
    let policy = PolicyBuilder::new()
        .layer(
            PolicyLayer::Editor,
            &serde_json::json!({"diagnostics": layer}),
        )
        .directives(Directives::new(suppressed.clone(), current.source()))
        .build();
    let report = apply(
        checks
            .iter()
            .cloned()
            .map(crate::diagnostic_policy::Finding::from)
            .collect(),
        &policy,
    );
    code_actions(current.source(), range, Some(current.analysis()), &report)
        .into_iter()
        .filter(|action| action.kind == ActionKind::QuickFix)
        .collect()
}
fn style_report(source: &str, analysis: &AnalysisResult) -> crate::diagnostic_policy::Report {
    crate::diagnostic_report::document_report_with_analysis(
        &w115_test_doc(source),
        Vec::new(),
        &Policy::unrestricted(),
        Some(analysis),
    )
}

// compiler-check fixes: IRULE5002/5004 flow-warning fixes

#[test]
fn check_actions_surface_irule5002_flow_fix() {
    // An unguarded `drop` fires IRULE5002 through the compiler-checks
    // pass (not the analyser's `AnalysisResult.diagnostics`), carrying an
    // "insert event disable all + return" fix.  The provider must lift it.
    use tcl_compiler::compilation_unit::CompilationUnit;
    use tcl_compiler::compiler_checks::run_all_checks;
    use tcl_lexer::LexerConfig;

    let registry = tcl_registry::model::ingress::static_context_for("f5-irules").commands();
    let src = "when CLIENT_ACCEPTED { drop }\n";
    let profile = tcl_dialect::DialectProfile::irules();
    let cu = CompilationUnit::build_for_with_config(
        src,
        registry,
        false,
        LexerConfig::for_dialect("f5-irules"),
    )
    .with_interprocedural(registry, Some(profile));
    let checks = run_all_checks(&cu, registry, Some(profile));
    let analysis = check_analysis(
        src,
        registry,
        profile,
        LexerConfig::for_dialect("f5-irules"),
    );
    let current = DiagnosticEditSource::for_analysis(src, &analysis).unwrap();
    assert!(
        checks
            .iter()
            .any(|d| d.code == DiagCode::Irule5002 && !d.fixes.is_empty()),
        "expected an IRULE5002 check with a fix, got {checks:?}",
    );

    let none_disabled = std::collections::HashSet::new();
    let actions = check_actions(
        &current,
        whole_document_range(src),
        &checks,
        &none_disabled,
        &no_suppression(),
    );
    let fix = actions
        .iter()
        .find(|a| a.title == "Add 'event disable all' + 'return'");
    assert!(fix.is_some(), "expected IRULE5002 quick-fix in {actions:?}");
    let fix = fix.unwrap();
    assert_eq!(fix.kind, ActionKind::QuickFix);
    assert_eq!(fix.edits.len(), 1);
    assert_eq!(fix.edits[0].new_text, "\n    event disable all\n    return");
    // Insertion (zero-width range) right after the `drop` command.
    assert_eq!(fix.edits[0].range.start_line, fix.edits[0].range.end_line);
    assert_eq!(
        fix.edits[0].range.start_character,
        fix.edits[0].range.end_character,
    );
}

#[test]
fn check_actions_empty_without_fixes() {
    // Checks with no fixes (or an out-of-range diagnostic) yield nothing.
    let src = "set x 1\n";
    let analysis = Analyser::new().analyse(src, "tcl8.6");
    let current = DiagnosticEditSource::for_analysis(src, &analysis).unwrap();
    let none_disabled = std::collections::HashSet::new();
    assert_eq!(
        check_actions(
            &current,
            whole_document_range(src),
            &[],
            &none_disabled,
            &no_suppression(),
        ),
        [] as [crate::code_actions::CodeAction; 0]
    );
}

#[test]
fn check_actions_honour_disabled_codes() {
    // A check whose code is disabled (`tclLsp.diagnostics.IRULE5002 = false`)
    // must not offer its quick-fix — otherwise the lightbulb re-surfaces a
    // diagnostic the user turned off.
    use tcl_compiler::compilation_unit::CompilationUnit;
    use tcl_compiler::compiler_checks::run_all_checks;
    use tcl_lexer::LexerConfig;

    let mut registry = tcl_registry::CommandRegistry::build_default();
    registry.load_irules();
    let src = "when CLIENT_ACCEPTED { drop }\n";
    let profile = tcl_dialect::DialectProfile::irules();
    let cu = CompilationUnit::build_for_with_config(
        src,
        &registry,
        false,
        LexerConfig::for_dialect("f5-irules"),
    )
    .with_interprocedural(&registry, Some(profile));
    let checks = run_all_checks(&cu, &registry, Some(profile));
    let analysis = check_analysis(
        src,
        &registry,
        profile,
        LexerConfig::for_dialect("f5-irules"),
    );
    let current = DiagnosticEditSource::for_analysis(src, &analysis).unwrap();

    let mut disabled = std::collections::HashSet::new();
    disabled.insert("IRULE5002".to_string());
    let actions = check_actions(
        &current,
        whole_document_range(src),
        &checks,
        &disabled,
        &no_suppression(),
    );
    assert!(
        !actions
            .iter()
            .any(|a| a.title == "Add 'event disable all' + 'return'"),
        "disabled IRULE5002 must not offer a quick-fix, got {actions:?}",
    );
}

// compiler-check fixes: shimmer-family noqa-suppress action

/// A check the document already silences offers nothing: neither its
/// quick-fix nor a suppress action for a line that is already suppressed.
#[test]
fn check_actions_skip_a_diagnostic_a_noqa_already_silences() {
    use tcl_compiler::analyser::Analyser;
    use tcl_compiler::compilation_unit::CompilationUnit;
    use tcl_compiler::compiler_checks::run_all_checks;

    let registry = tcl_registry::model::ingress::static_context_for("tcl8.6").commands();
    let src = "set x [string repeat 1 2]\n# noqa: S100\nincr x\n";
    let cu = CompilationUnit::build_for(src, registry, false);
    let checks = run_all_checks(
        &cu,
        registry,
        Some(tcl_dialect::DialectProfile::find("tcl8.6").unwrap()),
    );
    let analysis = check_analysis(
        src,
        registry,
        tcl_dialect::DialectProfile::find("tcl8.6").unwrap(),
        tcl_lexer::LexerConfig::default(),
    );
    let current = DiagnosticEditSource::for_analysis(src, &analysis).unwrap();
    assert!(
        checks.iter().any(|d| d.code == DiagCode::S100),
        "the check itself still fires; only its actions are withheld: {checks:?}"
    );
    let suppressed = Analyser::new()
        .analyse(src, "tcl8.6")
        .suppressed_lines
        .clone();

    let actions = check_actions(
        &current,
        whole_document_range(src),
        &checks,
        &std::collections::HashSet::new(),
        &suppressed,
    );
    assert!(
        actions.is_empty(),
        "a silenced S100 must leave no action behind: {actions:?}"
    );
}

/// A shimmering `incr` on a String variable fires S100 with no
/// `CodeFix` attached (there is no generally-safe automatic rewrite —
/// see `build_shimmer_noqa_suppress_action`'s doc comment), so the
/// provider must synthesise the suppress action itself rather than
/// only lifting `diag.fixes`.
#[test]
fn check_actions_surface_shimmer_noqa_suppress_action() {
    use tcl_compiler::compilation_unit::CompilationUnit;
    use tcl_compiler::compiler_checks::run_all_checks;

    let registry = tcl_registry::model::ingress::static_context_for("tcl8.6").commands();
    let src = "set x [string repeat 1 2]\nincr x\n";
    let cu = CompilationUnit::build_for(src, registry, false);
    let checks = run_all_checks(
        &cu,
        registry,
        Some(tcl_dialect::DialectProfile::find("tcl8.6").unwrap()),
    );
    let analysis = check_analysis(
        src,
        registry,
        tcl_dialect::DialectProfile::find("tcl8.6").unwrap(),
        tcl_lexer::LexerConfig::default(),
    );
    let current = DiagnosticEditSource::for_analysis(src, &analysis).unwrap();
    let s100 = checks
        .iter()
        .find(|d| d.code == DiagCode::S100)
        .unwrap_or_else(|| panic!("expected an S100 check, got {checks:?}"));
    assert!(
        s100.fixes.is_empty(),
        "S100 should carry no diag-level CodeFix — this test exercises the synthetic path"
    );

    let none_disabled = std::collections::HashSet::new();
    let actions = check_actions(
        &current,
        whole_document_range(src),
        &checks,
        &none_disabled,
        &no_suppression(),
    );
    let suppress = actions
        .iter()
        .find(|a| a.title == "Suppress S100 with a noqa comment");
    assert!(
        suppress.is_some(),
        "expected an S100 noqa-suppress action, got {actions:?}"
    );
    let suppress = suppress.unwrap();
    assert_eq!(suppress.kind, ActionKind::QuickFix);
    assert_eq!(suppress.edits.len(), 1);
    assert_eq!(suppress.edits[0].new_text, "# noqa: S100\n");
    // Insertion (zero-width range) at the start of the `incr x` line —
    // one line above where the current baseline text puts `incr x`.
    assert_eq!(suppress.edits[0].range.start_line, 1);
    assert_eq!(suppress.edits[0].range.start_character, 0);
    assert_eq!(
        suppress.edits[0].range.start_line,
        suppress.edits[0].range.end_line
    );
    assert_eq!(
        suppress.edits[0].range.start_character,
        suppress.edits[0].range.end_character,
    );
}

/// The suppress action's inserted line is indented to match the
/// command's own indentation, not flush to column 0.
#[test]
fn check_actions_shimmer_noqa_suppress_matches_indentation() {
    use tcl_compiler::compilation_unit::CompilationUnit;
    use tcl_compiler::compiler_checks::run_all_checks;

    let registry = tcl_registry::model::ingress::static_context_for("tcl8.6").commands();
    let src = "proc f {} {\n    set x [string repeat 1 2]\n    incr x\n}\n";
    let cu = CompilationUnit::build_for(src, registry, false);
    let checks = run_all_checks(
        &cu,
        registry,
        Some(tcl_dialect::DialectProfile::find("tcl8.6").unwrap()),
    );
    let analysis = check_analysis(
        src,
        registry,
        tcl_dialect::DialectProfile::find("tcl8.6").unwrap(),
        tcl_lexer::LexerConfig::default(),
    );
    let current = DiagnosticEditSource::for_analysis(src, &analysis).unwrap();
    assert!(checks.iter().any(|d| d.code == DiagCode::S100));

    let none_disabled = std::collections::HashSet::new();
    let actions = check_actions(
        &current,
        whole_document_range(src),
        &checks,
        &none_disabled,
        &no_suppression(),
    );
    let suppress = actions
        .iter()
        .find(|a| a.title == "Suppress S100 with a noqa comment")
        .unwrap_or_else(|| panic!("expected suppress action, got {actions:?}"));
    assert_eq!(suppress.edits[0].new_text, "    # noqa: S100\n");
}

/// A disabled shimmer code must not offer the suppress action either —
/// same "don't re-surface a hidden warning" rule as `IRULE5002` above.
#[test]
fn check_actions_shimmer_noqa_suppress_honours_disabled_codes() {
    use tcl_compiler::compilation_unit::CompilationUnit;
    use tcl_compiler::compiler_checks::run_all_checks;

    let registry = tcl_registry::CommandRegistry::build_default();
    let src = "set x hello\nincr x\n";
    let cu = CompilationUnit::build_for(src, &registry, false);
    let checks = run_all_checks(&cu, &registry, None);
    let analysis = check_analysis(
        src,
        &registry,
        tcl_dialect::DialectProfile::find("tcl8.6").unwrap(),
        tcl_lexer::LexerConfig::default(),
    );
    let current = DiagnosticEditSource::for_analysis(src, &analysis).unwrap();

    let mut disabled = std::collections::HashSet::new();
    disabled.insert("S100".to_string());
    let actions = check_actions(
        &current,
        whole_document_range(src),
        &checks,
        &disabled,
        &no_suppression(),
    );
    assert!(
        !actions.iter().any(|a| a.title.starts_with("Suppress S100")),
        "disabled S100 must not offer a suppress action, got {actions:?}",
    );
}

// refactor-engine dispatch (extract/inline var, if↔switch,
//    switch→dict, extract-datagroup)

fn analyse(source: &str) -> AnalysisResult {
    Analyser::new().analyse(source, "tcl8.6").clone()
}

/// An O127 pair is one quick-fix carrying both members' edits, never an
/// action for either member alone (#2149).
#[test]
fn a_grouped_rewrite_is_one_action_with_every_edit() {
    let src = "proc p {y} {\n    set x [llength $y]\n    puts $x\n}\n";
    let registry = tcl_registry::CommandRegistry::build_default();
    let pair: Vec<Finding> = tcl_compiler::optimiser::optimise_with_dialect(
        src,
        &registry,
        Some(crate::profile_for_dialect("tcl8.6")),
    )
    .into_iter()
    .filter(|o| o.code == DiagCode::O127)
    .map(Finding::from)
    .collect();
    assert_eq!(pair.len(), 2, "the fixture yields the O127 pair: {pair:?}");
    let policy = Policy {
        optimiser: crate::diagnostic_policy::OptimiserPolicy::all_on(),
        ..Policy::default()
    };
    let report = apply(pair, &policy);
    let actions: Vec<CodeAction> =
        code_actions(src, whole_document_range(src), Some(&analyse(src)), &report)
            .into_iter()
            .filter(|a| a.kind == ActionKind::QuickFix)
            .collect();
    assert_eq!(actions.len(), 1, "{actions:?}");
    assert_eq!(actions[0].edits.len(), 2, "{actions:?}");
    assert!(
        actions[0].edits.iter().any(|e| e.new_text.is_empty()),
        "the delete member rides the same action: {actions:?}"
    );
}

#[test]
fn extract_variable_surfaces_with_selection() {
    let src = "set x [string length $name]";
    let analysis = analyse(src);
    // Select the `[string length $name]` value (cols 6..27).
    let range = LspRange {
        start_line: 0,
        start_character: 6,
        end_line: 0,
        end_character: 27,
    };
    let actions = code_actions(
        src,
        range,
        Some(&analysis),
        &report_of(&analysis.diagnostics),
    );
    assert!(
        actions.iter().any(|a| {
            a.kind == ActionKind::RefactorExtract && a.title.to_lowercase().contains("variable")
        }),
        "{actions:?}",
    );
}

#[test]
fn w115_action_ignores_braced_and_quoted_pseudo_comments() {
    for src in [
        "set payload {# pseudo \\\nputs live}\n",
        "set payload \"# pseudo \\\nputs live\"\n",
        "set marker \"noqa\"; # ordinary comment \\\nputs live\n",
    ] {
        let analysis = analyse(src);
        let actions = code_actions(
            src,
            whole_document_range(src),
            Some(&analysis),
            &report_of(&analysis.diagnostics),
        );
        assert!(
            !actions
                .iter()
                .any(|action| action.title == "Convert to per-line comments"),
            "pseudo-comment offered W115 action: {actions:?}"
        );
    }
}

fn w115_test_doc(text: &str) -> crate::diagnostic_report::DocumentSource<'_> {
    crate::diagnostic_report::DocumentSource {
        text,
        analysis_text: text,
        decode: None,
        dialect: crate::profile_for_dialect("tcl8.6"),
        pass: crate::diagnostic_report::SourcePass::Tcl { line_length: 120 },
    }
}

/// The conversion follows a *shown* W115, not the bare comment
/// shape — a layer that turns W115 off, or a directive that silences it,
/// must silence the action too.
#[test]
fn a_conversion_follows_a_shown_w115() {
    let dialect = crate::profile_for_dialect("tcl8.6");
    let offers_conversion = |actions: &[CodeAction]| {
        actions
            .iter()
            .any(|a| a.title.contains("per-line comments"))
    };

    let src = "# trailing \\\nset x 1\n";
    let analysis = analyse(src);
    let shown = crate::diagnostic_report::document_report(
        &w115_test_doc(src),
        Vec::new(),
        &Policy::unrestricted(),
    );
    assert!(
        offers_conversion(&code_actions(src, line_range(0), Some(&analysis), &shown)),
        "a shown W115 must offer the conversion"
    );

    let w115_off = PolicyBuilder::new()
        .layer(
            PolicyLayer::Editor,
            &serde_json::json!({ "diagnostics": { "W115": false } }),
        )
        .build();
    let disabled =
        crate::diagnostic_report::document_report(&w115_test_doc(src), Vec::new(), &w115_off);
    assert!(
        !offers_conversion(&code_actions(
            src,
            line_range(0),
            Some(&analysis),
            &disabled
        )),
        "W115 disabled at a layer must not offer the conversion"
    );

    let marked = "# noqa: W115\n# trailing \\\nset x 1\n";
    let marked_analysis = analyse(marked);
    let scanned = PolicyBuilder::new()
        .directives(Directives::scan(marked, dialect))
        .build();
    let silenced =
        crate::diagnostic_report::document_report(&w115_test_doc(marked), Vec::new(), &scanned);
    assert!(
        !offers_conversion(&code_actions(
            marked,
            line_range(1),
            Some(&marked_analysis),
            &silenced
        )),
        "a `# noqa: W115` must not offer the conversion"
    );
}

#[test]
fn w115_action_uses_actual_schema_and_complete_original_unicode_lines() {
    // naming.core.original-comment-source-context
    // docs/design/analysis/name-resolution-proofs/core-original-comment-source-context.md
    let source = "hold-script {\n    # café 😀 \\\n    puts 😀\n}\n";
    let mut registry = tcl_registry::CommandRegistry::build_default();
    registry.insert(tcl_registry::CommandSpec {
        name: "hold-script",
        arity: tcl_registry::Arity::exact(1),
        arg_roles: &[(0, tcl_registry::ArgRole::Body)],
        ..tcl_registry::CommandSpec::DEFAULT
    });
    let profile = crate::profile_for_dialect("tcl8.6");
    let config = tcl_lexer::LexerConfig {
        strict_quoting: true,
        ..tcl_lexer::LexerConfig::for_file_grammar(profile.grammar)
    };
    let analysis = check_analysis(source, &registry, profile, config);
    let range = LspRange {
        start_line: 1,
        start_character: 4,
        end_line: 1,
        end_character: 4,
    };
    let actions = code_actions(
        source,
        range,
        Some(&analysis),
        &style_report(source, &analysis),
    );
    let action = actions
        .iter()
        .find(|action| action.title == "Convert to per-line comments")
        .unwrap();
    assert_eq!(action.edits.len(), 1);
    assert_eq!(
        action.edits[0].range,
        LspRange {
            start_line: 1,
            start_character: 0,
            end_line: 2,
            end_character: 11
        }
    );
    assert_eq!(action.edits[0].new_text, "    # café 😀\n    # puts 😀");
    assert!(
        !code_actions(
            &format!("{source} "),
            range,
            Some(&analysis),
            &style_report(source, &analysis)
        )
        .iter()
        .any(|action| action.title == "Convert to per-line comments")
    );
}

#[test]
fn w115_action_never_replaces_a_command_prefix_beside_a_body_comment() {
    // naming.core.original-comment-source-context
    // docs/design/analysis/name-resolution-proofs/core-original-comment-source-context.md
    let source = "if 1 {# inline \\\nputs hidden\n}\n";
    let analysis = analyse(source);
    assert!(
        !code_actions(
            source,
            whole_document_range(source),
            Some(&analysis),
            &style_report(source, &analysis)
        )
        .iter()
        .any(|action| action.title == "Convert to per-line comments")
    );
}

#[test]
fn retarget_newlines_rewrites_inserted_text_onto_the_documents_eol() {
    // The builders compose with `\n`; on a CRLF document every newline an
    // action inserts must be `\r\n`, or a "Generate docstring" / "Extract
    // into variable" quietly mixes terminators into the file.
    let src = "proc  p {a  b} {\r\nputs $b\r\n}\r\n";
    let analysis = analyse(src);
    let range = LspRange {
        start_line: 0,
        start_character: 0,
        end_line: 0,
        end_character: 16,
    };
    let mut actions = code_actions(
        src,
        range,
        Some(&analysis),
        &report_of(&analysis.diagnostics),
    );
    assert!(
        actions
            .iter()
            .any(|a| a.edits.iter().any(|e| e.new_text.contains('\n'))),
        "fixture must produce at least one multi-line insertion: {actions:?}",
    );
    retarget_newlines(&mut actions, "\r\n");
    for action in &actions {
        for edit in &action.edits {
            assert!(
                !edit.new_text.replace("\r\n", "").contains('\n'),
                "{}: bare LF survived in {:?}",
                action.title,
                edit.new_text,
            );
            assert!(
                !edit.new_text.replace("\r\n", "").contains('\r'),
                "{}: bare CR survived in {:?}",
                action.title,
                edit.new_text,
            );
        }
    }
}

#[test]
fn retarget_newlines_is_a_no_op_for_lf() {
    let mut actions = vec![CodeAction::new(
        "t".to_owned(),
        vec![crate::rename::TextEdit {
            range: LspRange {
                start_line: 0,
                start_character: 0,
                end_line: 0,
                end_character: 0,
            },
            new_text: "a\nb\n".to_owned(),
        }],
        ActionKind::QuickFix,
        None,
    )];
    retarget_newlines(&mut actions, "\n");
    assert_eq!(actions[0].edits[0].new_text, "a\nb\n");
    // …and folds a mixed insertion onto a single terminator.
    retarget_newlines(&mut actions, "\r");
    assert_eq!(actions[0].edits[0].new_text, "a\rb\r");
}

#[test]
fn if_to_switch_surfaces_at_cursor() {
    let src = "if {$x eq \"a\"} {\n    puts 1\n} elseif {$x eq \"b\"} {\n    puts 2\n}";
    let analysis = analyse(src);
    let cursor = LspRange {
        start_line: 0,
        start_character: 0,
        end_line: 0,
        end_character: 0,
    };
    let actions = code_actions(
        src,
        cursor,
        Some(&analysis),
        &report_of(&analysis.diagnostics),
    );
    assert!(
        actions
            .iter()
            .any(|a| a.title.to_lowercase().contains("switch")),
        "{actions:?}",
    );
}

/// The refactor code actions reach control flow inside an
/// `apply` lambda body too.  `apply`'s literal is
/// `ArgRole::LambdaLiteral`, so the descent has to split it rather than
/// re-segment the whole `{argList body}` blob — which read `{m}` as a
/// command name and left every `body_words`-backed action silently
/// unavailable in there.
#[test]
fn if_to_switch_surfaces_inside_an_apply_lambda_body() {
    let src = "proc handler {} {\n    apply {{m} {\n        if {$m eq \"GET\"} {\n            puts get\n        } elseif {$m eq \"POST\"} {\n            puts post\n        }\n    }} $x\n}\n";
    let analysis = analyse(src);
    // Cursor on the `if` inside the lambda body (line 2, col 8).
    let cursor = LspRange {
        start_line: 2,
        start_character: 8,
        end_line: 2,
        end_character: 8,
    };
    let actions = code_actions(
        src,
        cursor,
        Some(&analysis),
        &report_of(&analysis.diagnostics),
    );
    assert!(
        actions
            .iter()
            .any(|a| a.title.to_lowercase().contains("switch")),
        "{actions:?}",
    );
}

#[test]
fn extract_datagroup_surfaces_and_carries_definition() {
    let src = "if {$host eq \"a.com\"} {\n    pool web_pool\n} elseif {$host eq \"b.com\"} {\n    pool web_pool\n} elseif {$host eq \"c.com\"} {\n    pool web_pool\n}";
    let analysis = Analyser::new().analyse(src, "f5-irules");
    let cursor = LspRange {
        start_line: 0,
        start_character: 0,
        end_line: 0,
        end_character: 0,
    };
    let actions = code_actions(
        src,
        cursor,
        Some(&analysis),
        &report_of(&analysis.diagnostics),
    );
    let dg = actions
        .iter()
        .find(|a| a.title.to_lowercase().contains("data-group"))
        .expect("data-group action");
    let def = dg
        .data_group_definition
        .as_ref()
        .expect("data_group_definition payload");
    assert!(def.contains("ltm data-group internal"), "{def:?}");
    assert!(def.contains("type string"), "{def:?}");
}

/// A pack-load notice about a dropped property becomes the one edit that
/// spells the property the way the grammar in force does.
#[test]
fn a_dropped_property_offers_the_nearest_real_word() {
    let src = "speclib mylib 1 {\n    command mylib::x {\n        arty 1\n    }\n}\n";
    let diags = vec![ContextDiagnostic {
        data: SpecPackNoticeSubject::from_loader(
            src,
            tcl_lexer::LexerConfig::for_file_grammar(crate::profile_for_dialect("spectcl").grammar),
            3,
            "arty",
            SpecPackNoticeKind::Property,
        )
        .and_then(|subject| {
            ContextDiagnosticData::from_spec_pack_notice(
                &subject,
                &Analyser::new().analyse(src, "spectcl"),
            )
        }),
        code: SPEC_PACK_DIAGNOSTIC_CODE.to_owned(),
        message: "mylib::x: unknown property `arty` dropped".to_owned(),
        range: LspRange {
            start_line: 2,
            start_character: 0,
            end_line: 2,
            end_character: 14,
        },
    }];
    let actions = spec_pack_quick_fixes(src, &Analyser::new().analyse(src, "spectcl"), &diags);
    assert_eq!(actions.len(), 1, "{actions:?}");
    assert_eq!(actions[0].title, "Change `arty` to `arity`");
    assert_eq!(actions[0].edits.len(), 1);
    assert_eq!(actions[0].edits[0].new_text, "arity");
    assert_eq!(actions[0].edits[0].range.start_line, 2);
    assert_eq!(actions[0].edits[0].range.start_character, 8);
    assert_eq!(actions[0].edits[0].range.end_character, 12);
}

/// The candidate set is the grammar *in force*, not the pack's whole
/// vocabulary: `summary` is a `hover` word, so a typo of it inside a
/// `command` body must not resolve to it.
#[test]
fn the_candidate_vocabulary_is_the_grammar_in_force() {
    let src = "speclib mylib 1 {\n    command mylib::x {\n        hover {\n            sumary {x}\n        }\n    }\n}\n";
    let diags = vec![ContextDiagnostic {
        data: SpecPackNoticeSubject::from_loader(
            src,
            tcl_lexer::LexerConfig::for_file_grammar(crate::profile_for_dialect("spectcl").grammar),
            4,
            "sumary",
            SpecPackNoticeKind::Property,
        )
        .and_then(|subject| {
            ContextDiagnosticData::from_spec_pack_notice(
                &subject,
                &Analyser::new().analyse(src, "spectcl"),
            )
        }),
        code: SPEC_PACK_DIAGNOSTIC_CODE.to_owned(),
        message: "unknown property `sumary` dropped".to_owned(),
        range: LspRange {
            start_line: 3,
            start_character: 0,
            end_line: 3,
            end_character: 22,
        },
    }];
    let actions = spec_pack_quick_fixes(src, &Analyser::new().analyse(src, "spectcl"), &diags);
    assert_eq!(actions.len(), 1, "{actions:?}");
    assert_eq!(actions[0].edits[0].new_text, "summary");

    // The same misspelling one level out has no `summary` to reach.
    let outer = "speclib mylib 1 {\n    command mylib::x {\n        sumary {x}\n    }\n}\n";
    let outer_diags = vec![ContextDiagnostic {
        data: SpecPackNoticeSubject::from_loader(
            outer,
            tcl_lexer::LexerConfig::for_file_grammar(crate::profile_for_dialect("spectcl").grammar),
            3,
            "sumary",
            SpecPackNoticeKind::Property,
        )
        .and_then(|subject| {
            ContextDiagnosticData::from_spec_pack_notice(
                &subject,
                &Analyser::new().analyse(outer, "spectcl"),
            )
        }),
        code: SPEC_PACK_DIAGNOSTIC_CODE.to_owned(),
        message: "unknown property `sumary` dropped".to_owned(),
        range: LspRange {
            start_line: 2,
            start_character: 0,
            end_line: 2,
            end_character: 18,
        },
    }];
    assert!(
        !spec_pack_quick_fixes(
            outer,
            &Analyser::new().analyse(outer, "spectcl"),
            &outer_diags
        )
        .iter()
        .any(|a| a.edits[0].new_text == "summary")
    );
}

/// A dropped flag is corrected against the option table of the row it was
/// written on, which the registry owns.
#[test]
fn a_dropped_flag_offers_the_nearest_option() {
    let src = "speclib mylib 1 {\n    command mylib::x {\n        arg 0 -rle Body\n    }\n}\n";
    let diags = vec![ContextDiagnostic {
        data: SpecPackNoticeSubject::from_loader(
            src,
            tcl_lexer::LexerConfig::for_file_grammar(crate::profile_for_dialect("spectcl").grammar),
            3,
            "-rle",
            SpecPackNoticeKind::Flag { row: "arg".into() },
        )
        .and_then(|subject| {
            ContextDiagnosticData::from_spec_pack_notice(
                &subject,
                &Analyser::new().analyse(src, "spectcl"),
            )
        }),
        code: SPEC_PACK_DIAGNOSTIC_CODE.to_owned(),
        message: "unknown flag `-rle` on `arg` dropped".to_owned(),
        range: LspRange {
            start_line: 2,
            start_character: 0,
            end_line: 2,
            end_character: 23,
        },
    }];
    let actions = spec_pack_quick_fixes(src, &Analyser::new().analyse(src, "spectcl"), &diags);
    assert_eq!(actions.len(), 1, "{actions:?}");
    assert_eq!(actions[0].edits[0].new_text, "-role");
}

/// Two rows on one line: `;` ends a statement, so the row a word sits on
/// is the segmenter's answer and not the line's first word — which here
/// names the *neighbour* and would offer nothing at all.
#[test]
fn a_row_sharing_its_line_is_corrected_against_its_own_head() {
    let src = "speclib mylib 1 {\n    command mylib::x {\n        arity 1; arty 2\n    }\n}\n";
    let diags = vec![ContextDiagnostic {
        data: SpecPackNoticeSubject::from_loader(
            src,
            tcl_lexer::LexerConfig::for_file_grammar(crate::profile_for_dialect("spectcl").grammar),
            3,
            "arty",
            SpecPackNoticeKind::Property,
        )
        .and_then(|subject| {
            ContextDiagnosticData::from_spec_pack_notice(
                &subject,
                &Analyser::new().analyse(src, "spectcl"),
            )
        }),
        code: SPEC_PACK_DIAGNOSTIC_CODE.to_owned(),
        message: "mylib::x: unknown property `arty` dropped".to_owned(),
        range: LspRange {
            start_line: 2,
            start_character: 0,
            end_line: 2,
            end_character: 23,
        },
    }];
    let actions = spec_pack_quick_fixes(src, &Analyser::new().analyse(src, "spectcl"), &diags);
    assert_eq!(actions.len(), 1, "{actions:?}");
    assert_eq!(actions[0].edits[0].new_text, "arity");
    assert_eq!(actions[0].edits[0].range.start_character, 17);

    // …and the flag case, whose option table comes from the head of the
    // row the flag was written on (`arg`), not the line's (`arity`).
    let src =
        "speclib mylib 1 {\n    command mylib::x {\n        arity 1; arg 0 -rle Body\n    }\n}\n";
    let diags = vec![ContextDiagnostic {
        data: SpecPackNoticeSubject::from_loader(
            src,
            tcl_lexer::LexerConfig::for_file_grammar(crate::profile_for_dialect("spectcl").grammar),
            3,
            "-rle",
            SpecPackNoticeKind::Flag { row: "arg".into() },
        )
        .and_then(|subject| {
            ContextDiagnosticData::from_spec_pack_notice(
                &subject,
                &Analyser::new().analyse(src, "spectcl"),
            )
        }),
        code: SPEC_PACK_DIAGNOSTIC_CODE.to_owned(),
        message: "unknown flag `-rle` on `arg` dropped".to_owned(),
        range: LspRange {
            start_line: 2,
            start_character: 0,
            end_line: 2,
            end_character: 32,
        },
    }];
    let actions = spec_pack_quick_fixes(src, &Analyser::new().analyse(src, "spectcl"), &diags);
    assert_eq!(actions.len(), 1, "{actions:?}");
    assert_eq!(actions[0].edits[0].new_text, "-role");
}

/// A word the buffer no longer carries — the notice is from the last load,
/// the author has since retyped the line — yields no edit at all.
#[test]
fn a_notice_the_buffer_has_moved_past_offers_nothing() {
    let src = "speclib mylib 1 {\n    command mylib::x {\n        arity 1\n    }\n}\n";
    let diags = vec![ContextDiagnostic {
        data: SpecPackNoticeSubject::from_loader(
            src,
            tcl_lexer::LexerConfig::for_file_grammar(crate::profile_for_dialect("spectcl").grammar),
            3,
            "arty",
            SpecPackNoticeKind::Property,
        )
        .and_then(|subject| {
            ContextDiagnosticData::from_spec_pack_notice(
                &subject,
                &Analyser::new().analyse(src, "spectcl"),
            )
        }),
        code: SPEC_PACK_DIAGNOSTIC_CODE.to_owned(),
        message: "unknown property `arty` dropped".to_owned(),
        range: LspRange {
            start_line: 2,
            start_character: 0,
            end_line: 2,
            end_character: 15,
        },
    }];
    assert_eq!(
        spec_pack_quick_fixes(src, &Analyser::new().analyse(src, "spectcl"), &diags),
        [] as [crate::code_actions::CodeAction; 0]
    );
}

/// Only a pack file: the same-shaped diagnostic on an ordinary Tcl
/// document has no closed vocabulary behind it.
#[test]
fn an_ordinary_tcl_document_gets_no_pack_quick_fix() {
    let src = "proc arty {} {}\n";
    let diags = vec![ContextDiagnostic {
        data: SpecPackNoticeSubject::from_loader(
            src,
            tcl_lexer::LexerConfig::for_file_grammar(crate::profile_for_dialect("spectcl").grammar),
            1,
            "arty",
            SpecPackNoticeKind::Property,
        )
        .and_then(|subject| {
            ContextDiagnosticData::from_spec_pack_notice(
                &subject,
                &Analyser::new().analyse(src, "spectcl"),
            )
        }),
        code: SPEC_PACK_DIAGNOSTIC_CODE.to_owned(),
        message: "unknown property `arty` dropped".to_owned(),
        range: LspRange {
            start_line: 0,
            start_character: 0,
            end_line: 0,
            end_character: 15,
        },
    }];
    assert_eq!(
        spec_pack_quick_fixes(src, &Analyser::new().analyse(src, "tcl"), &diags,),
        [] as [crate::code_actions::CodeAction; 0]
    );
}
