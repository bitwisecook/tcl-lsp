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

//! Diagnostics keep the original written source: alias ordinals, selected words, effective roles and fix proposals.

use super::*;

/// A finding on a captured alias operand: reported at the call's real error,
/// anchored on the captured word where the alias definition writes it, with
/// no fix (diagnostic-registry-source-ownership.md: captured slots borrow no
/// call-site anchor and receive no review-fix anchor).
fn assert_captured_finding(source: &str, code: DiagCode, captured_word: &str) {
    assert_captured_finding_in(source, "tcl8.6", code, captured_word);
}

fn assert_captured_finding_in(source: &str, dialect: &str, code: DiagCode, captured_word: &str) {
    let result = Analyser::new().analyse(source, dialect);
    let found = result
        .diagnostics
        .iter()
        .filter(|diagnostic| diagnostic.code == code)
        .collect::<Vec<_>>();
    assert_eq!(found.len(), 1, "{source}: {:?}", result.diagnostics);
    let anchor = source.find(captured_word).expect("captured word");
    assert_eq!(
        found[0].span.start() as usize..found[0].span.end() as usize,
        anchor..anchor + captured_word.len(),
        "{source}"
    );
    assert!(found[0].fixes.is_empty(), "{source}: {:?}", found[0].fixes);
    assert!(
        found[0]
            .message
            .contains("reached through the call at line"),
        "{source}: {}",
        found[0].message
    );
}

#[test]
fn w144_core_subcommand_lifecycle_uses_registry_safe_fix() {
    // naming.diagnostic.registry-source-ownership
    // docs/design/analysis/name-resolution-proofs/diagnostic-registry-source-ownership.md
    let src = "interp slaves\n";
    for dialect in ["tcl8.6", "tcl9.0"] {
        let result = crate::analyser::Analyser::new().analyse(src, dialect);
        let diagnostic = result
            .diagnostics
            .iter()
            .find(|diagnostic| diagnostic.code == DiagCode::W144)
            .unwrap_or_else(|| panic!("{dialect}: expected W144, got {:?}", result.diagnostics));
        assert_eq!(&src[diagnostic.span.as_range()], "slaves");
        assert!(
            diagnostic.message.contains("Tcl 8.6") || diagnostic.message.contains("Tcl 9.0"),
            "{dialect}: W144 must use the active Tcl-core axis: {diagnostic:?}"
        );
        assert_eq!(diagnostic.fixes.len(), 1, "{dialect}: {diagnostic:?}");
        let fix = &diagnostic.fixes[0];
        assert_eq!(&src[fix.span.as_range()], "slaves");
        assert_eq!(fix.new_text, "children");
        assert_eq!(fix.safety, crate::irules_checks::FixSafety::RequiresReview);
    }

    let legacy = crate::analyser::Analyser::new().analyse(src, "tcl8.5");
    assert!(
        !legacy
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == DiagCode::W144),
        "slaves predates the Tcl 8.6 preferred spelling: {:?}",
        legacy.diagnostics
    );
}

/// Invariant I4: analyser-hook selection requires the
/// binding proof — a version-gated head outside the document's release
/// window resolves under no proof (`Absent`), so no hook specialises and
/// the generic walk handles the call; the same head under a release that
/// provides it selects its hook as before. The walk state is set up the
/// way `analyse*` does mid-run (`resolve_walk_environment` + the profile
/// registry); `analyse` itself clears the run state on exit.
#[test]
fn analyser_hook_selection_requires_binding_proof() {
    // `oo::define` is 8.6+ (`try`, also 8.6+, carries no analyser hook, so
    // it cannot serve here).
    let args = vec!["Foo".to_string(), "{ }".to_string()];
    let mut old = crate::analyser::Analyser::new();
    let _ = old.resolve_walk_environment("tcl8.4");
    old.registry = Some(old.profile_registry());
    assert!(
        old.resolve_analyser_hook("oo::define", &args).is_none(),
        "`oo::define` is 8.6+: under tcl8.4 the binding is Absent, so no \
         analyser hook may specialise (I4)"
    );
    let mut new = crate::analyser::Analyser::new();
    let _ = new.resolve_walk_environment("tcl9.0");
    new.registry = Some(new.profile_registry());
    assert_eq!(
        new.resolve_analyser_hook("oo::define", &args),
        Some(tcl_registry::hooks::AnalyserHookId::OoDefine),
        "a proved binding keeps its hook"
    );
}

#[test]
fn w120_retains_selected_package_key_independently_of_message() {
    let mut analyser = Analyser::new();
    let result = analyser.analyse("button .b", "tcl8.6");
    let mut diagnostic = result
        .diagnostics
        .iter()
        .find(|diagnostic| diagnostic.code == DiagCode::W120)
        .unwrap()
        .clone();
    let key = diagnostic.required_package_key().unwrap().clone();
    assert!(key.matches_ascii("Tk"));
    diagnostic.message = "translated package advice".to_owned();
    assert_eq!(diagnostic.required_package_key(), Some(&key));
}

#[test]
fn w123_retains_original_missing_word_and_position_independently_of_message() {
    let source = "namespace eval ns {}\nns::missing hello";
    let mut analyser = Analyser::new();
    let result = analyser.analyse(source, "tcl8.6");
    let mut diagnostic = result
        .diagnostics
        .iter()
        .find(|diagnostic| diagnostic.code == DiagCode::W123)
        .unwrap()
        .clone();
    let subject = diagnostic.unresolved_command().unwrap();
    assert_eq!(subject.reporting_name(), "ns::missing");
    assert_eq!(subject.name_input().bytes(), b"ns::missing");
    assert_eq!(
        subject.invocation().invocation_site().unwrap().offset,
        diagnostic.span.start()
    );
    assert_eq!(
        subject.name_input().source_image().bytes(),
        source.as_bytes()
    );
    let subject = subject.clone();
    diagnostic.message = "translated unresolved advice".to_owned();
    assert_eq!(diagnostic.unresolved_command(), Some(&subject));
}

#[test]
fn diagnostic_without_owned_subject_does_not_parse_identity_from_message() {
    let diagnostic = Diagnostic::new(
        DiagCode::W120,
        Span::new(0, 1),
        "requires `package require Tk`",
        Severity::Warning,
    );
    assert!(diagnostic.subject().is_none());
    assert!(diagnostic.required_package_key().is_none());
    assert!(diagnostic.unresolved_command().is_none());
}

#[test]
fn w123_retains_original_math_function_context_without_command_head_authority() {
    // naming.diagnostic.original-math-function-subject
    // docs/design/analysis/name-resolution-proofs/diagnostic-original-math-function-subject.md
    use crate::command_binding::SourceCommandSlotPresence;
    use tcl_registry::mathfunc::NativeMathFunctionDispatch;
    let source = "expr {Pi()}";
    for dialect in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1", "jim"] {
        let analysis = Analyser::new().analyse(source, dialect);
        let mut diagnostic = analysis
            .diagnostics
            .iter()
            .find(|diagnostic| {
                diagnostic.code == DiagCode::W123 && diagnostic.span == Span::new(6, 8)
            })
            .expect("original missing function advice")
            .clone();
        let subject = diagnostic
            .unresolved_math_function()
            .expect("genuine original function")
            .clone();
        let occurrence = subject.occurrence();
        assert_eq!(subject.reporting_name(), "Pi", "{dialect}");
        assert_eq!(subject.presence(), SourceCommandSlotPresence::Absent);
        assert_eq!(occurrence.source_image().bytes(), source.as_bytes());
        assert_eq!(occurrence.span(), Span::new(6, 8));
        assert_eq!(occurrence.ordinal(), 0);
        assert_eq!(occurrence.argument_count(), 0);
        assert_eq!(
            occurrence.dispatch(),
            if matches!(dialect, "tcl8.4" | "jim") {
                NativeMathFunctionDispatch::FixedTable
            } else {
                NativeMathFunctionDispatch::CommandTable
            }
        );
        assert!(occurrence.command_reference().is_none());
        assert!(occurrence.registry_identity().is_none());
        assert_eq!(
            occurrence.lexer_config(),
            analysis.resolved_input.as_ref().unwrap().lexer_config()
        );
        assert!(diagnostic.unresolved_command().is_none());
        diagnostic.message = "translated misleading 'Other' command text".to_owned();
        diagnostic.fixes.clear();
        assert_eq!(diagnostic.unresolved_math_function(), Some(&subject));
    }
    let plain = Analyser::new().analyse("Pi", "tcl8.6");
    let plain = plain
        .diagnostics
        .iter()
        .find(|diagnostic| diagnostic.code == DiagCode::W123)
        .unwrap();
    assert!(plain.unresolved_math_function().is_none());
    assert_eq!(plain.unresolved_command().unwrap().reporting_name(), "Pi");
}

#[test]
fn original_option_diagnostics_share_alias_ordinals_and_stop_at_unknown_layout() {
    // naming.diagnostic.registry-source-ownership
    // docs/design/analysis/name-resolution-proofs/diagnostic-registry-source-ownership.md
    for source in [
        "interp alias {} replace {} regsub\nreplace -command x y z\n",
        "rename regsub replace\nreplace -command x y z\n",
    ] {
        let result = Analyser::new().analyse(source, "tcl8.6");
        let found = result
            .diagnostics
            .iter()
            .filter(|diagnostic| diagnostic.code == DiagCode::W004)
            .collect::<Vec<_>>();
        assert_eq!(found.len(), 1, "{source}: {found:?}");
        let subject = found[0]
            .registry_source()
            .expect("typed disabled option subject");
        assert_eq!(
            subject.kind(),
            crate::analyser::RegistrySourceDiagnosticKind::DisabledOption
        );
        assert_eq!(subject.argument(), Some(0));
        assert_eq!(subject.written_argument(), Some(0));
        assert_eq!(&source[subject.span().as_range()], "-command");
    }
    for source in [
        "proc regsub args {}; regsub -command x y z",
        "regsub $flag -command x y z",
        "regsub {*}$arguments -command x y z",
        "regsub x y -command out",
    ] {
        assert!(
            !Analyser::new()
                .analyse(source, "tcl8.6")
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic.code == DiagCode::W004),
            "{source}"
        );
    }
    // tclsh8.4.20/8.6.18: replace x y z raises bad option "-command"; 9.0.4
    // and 9.1.0 accept it (evidence/tcl/naming-diagnostic-registry-source-
    // ownership-captured-alias-call-errors.md).
    let captured = "interp alias {} replace {} regsub -command\nreplace x y z";
    assert_captured_finding_in(captured, "tcl8.6", DiagCode::W004, "-command");
    assert!(
        !Analyser::new()
            .analyse(captured, "tcl9.0")
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == DiagCode::W004)
    );
}

/// W100's produced set is the fact `brace_expr_hints` reads
/// for O111, so it must cover every unbraced expression argument — one
/// finding, at the expression word's own span, for each EXPR-role form.
#[test]
fn w100_marks_every_unbraced_expression() {
    fn w100_texts(src: &str) -> Vec<String> {
        let mut a = crate::analyser::Analyser::new();
        a.analyse(src, "tcl8.6")
            .diagnostics
            .iter()
            .filter(|d| d.code == DiagCode::W100)
            .map(|d| src[d.span.start() as usize..d.span.end() as usize].to_string())
            .collect()
    }

    for (src, word) in [
        ("expr $a+1\n", "$a+1"),
        ("expr \"$a + 1\"\n", "\"$a + 1\""),
        ("if \"$x\" {puts hi}\n", "\"$x\""),
        ("while $c {}\n", "$c"),
        ("for {} $c {} {}\n", "$c"),
        ("set y [expr $x+1]\n", "$x+1"),
    ] {
        assert_eq!(
            w100_texts(src),
            vec![word.to_string()],
            "expected exactly one W100 at {word:?} for {src:?}"
        );
    }

    for src in [
        "expr {$a+1}\n",
        "expr {$a + 1}\n",
        "if {$x} {puts hi}\n",
        "while {$c} {}\n",
        "for {} {$c} {} {}\n",
        "set y [expr {$x+1}]\n",
    ] {
        assert!(
            w100_texts(src).is_empty(),
            "the braced form must draw no W100 for {src:?}"
        );
    }
}

#[test]
fn original_terminator_advice_retains_source_words_without_current_value_claims() {
    // naming.diagnostic.registry-source-ownership
    // docs/design/analysis/name-resolution-proofs/diagnostic-registry-source-ownership.md
    let source = concat!(
        "interp alias {} remove {} file delete -force\n",
        r#"remove "$path""#,
        "\n"
    );
    let result = Analyser::new().analyse(source, "tcl8.6");
    let found = result
        .diagnostics
        .iter()
        .filter(|diagnostic| diagnostic.code == DiagCode::W304)
        .collect::<Vec<_>>();
    assert_eq!(found.len(), 1, "{found:?}");
    let subject = found[0]
        .registry_source()
        .expect("typed conditional data-or-option subject");
    assert_eq!(subject.argument(), Some(2));
    assert_eq!(subject.written_argument(), Some(0));
    assert_eq!(&source[subject.span().as_range()], "\"$path\"");
    assert_eq!(found[0].fixes[0].new_text, "-- \"$path\"");
    assert_eq!(found[0].severity, Severity::Suggestion);
    for source in [
        "proc regexp args {}; regexp $pattern $text",
        "rename regexp {}; regexp $pattern $text",
        "regexp {*}$arguments $pattern $text",
        "regexp -- $pattern $text",
    ] {
        assert!(
            !Analyser::new()
                .analyse(source, "tcl8.6")
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic.code == DiagCode::W304),
            "{source}"
        );
    }
    // tclsh8.4.20/8.6.18/9.0.4: test value raises bad option "-dashPattern".
    assert_captured_finding(
        "interp alias {} test {} regexp -dashPattern\ntest value",
        DiagCode::W304,
        "-dashPattern",
    );
}

#[test]
fn original_registry_diagnostics_follow_written_alias_operands() {
    // naming.diagnostic.registry-source-ownership
    // docs/design/analysis/name-resolution-proofs/diagnostic-registry-source-ownership.md
    // Source metadata and reporting geometry; no native alias-installation claim.
    let source = "interp alias {} observe {} trace add variable value\nobserve {read bad} callback";
    let analysis = Analyser::new().analyse(source, "tcl8.6");
    let diagnostic = analysis
        .diagnostics
        .iter()
        .find(|diagnostic| diagnostic.code == tcl_core_types::DiagCode::W146)
        .expect("selected trace operation grammar still applies after the retained prefix");
    let subject = diagnostic.registry_source().unwrap();
    assert_eq!(
        subject.kind(),
        crate::analyser::RegistrySourceDiagnosticKind::LiteralArgument
    );
    assert_eq!(subject.words().command(), "trace");
    assert_eq!(subject.argument(), Some(3));
    assert_eq!(subject.written_argument(), Some(0));
    assert_eq!(&source[subject.span().as_range()], "{read bad}");
    assert_eq!(diagnostic.span, subject.span());
    assert!(
        diagnostic
            .fixes
            .iter()
            .all(|fix| fix.span == subject.span())
    );
}

#[test]
fn original_registry_variable_role_diagnostics_keep_shadow_and_capture_boundaries() {
    // naming.diagnostic.registry-source-ownership
    // docs/design/analysis/name-resolution-proofs/diagnostic-registry-source-ownership.md
    for source in [
        "rename set write\nwrite $name 1",
        "interp alias {} write {} set\nwrite $name 1",
    ] {
        let analysis = Analyser::new().analyse(source, "tcl8.6");
        let diagnostic = analysis
            .diagnostics
            .iter()
            .find(|diagnostic| diagnostic.code == tcl_core_types::DiagCode::W212)
            .expect("original source alias retains its variable-name role");
        let subject = diagnostic.registry_source().unwrap();
        assert_eq!(
            subject.kind(),
            crate::analyser::RegistrySourceDiagnosticKind::VariableName
        );
        assert_eq!(subject.argument(), Some(0));
        assert_eq!(subject.written_argument(), Some(0));
        assert_eq!(&source[subject.span().as_range()], "$name");
    }
    for source in [
        "proc set args {}; set $name 1",
        "namespace eval n {proc set args {}; set $name 1}",
        "rename set {}; set $name 1",
        "interp alias {} write {} set captured\nwrite $value",
        "interp alias {} write {} set\nwrite {*}$arguments",
    ] {
        assert!(
            !Analyser::new()
                .analyse(source, "tcl8.6")
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic.code == tcl_core_types::DiagCode::W212),
            "{source}"
        );
    }
    let source = "interp alias {} write {} set\nwrite ${name}(key) value";
    assert_eq!(
        w216_count(source),
        0,
        "selected alias role preserves the indirect array idiom"
    );
}

#[test]
fn original_selector_diagnostics_share_alias_ordinals_and_declared_default_forms() {
    // naming.diagnostic.registry-source-ownership
    // docs/design/analysis/name-resolution-proofs/diagnostic-registry-source-ownership.md
    // This checks source-schema selection and written coordinates, not native execution.
    for source in [
        "interp alias {} strings {} string\nstrings {lenght} abc",
        "rename string strings\nstrings {lenght} abc",
        "::::string {lenght} abc",
    ] {
        let result = crate::analyser::Analyser::new().analyse(source, "tcl8.6");
        let diagnostic = result
            .diagnostics
            .iter()
            .find(|d| d.code == DiagCode::W001)
            .expect(source);
        let subject = diagnostic
            .registry_source()
            .expect("typed original selector");
        assert_eq!(subject.argument(), Some(0));
        assert_eq!(subject.written_argument(), Some(0));
        assert_eq!(
            &source[diagnostic.span.start() as usize..diagnostic.span.end() as usize],
            "{lenght}"
        );
        assert_eq!(diagnostic.fixes[0].new_text, "length");
    }
    assert!(has_code("string .bogus abc", "tcl8.6", "W001"));
    for source in [
        "after 10 {set x 1}",
        "set selector lenght; string $selector abc",
        "proc string args {}; string lenght abc",
        "string {*}{lenght} abc",
    ] {
        assert!(!has_code(source, "tcl8.6", "W001"), "{source}");
    }
    // tclsh8.6.18/9.0.4: strings abc raises unknown or ambiguous subcommand
    // "lenght" (8.4.20: bad option).
    assert_captured_finding(
        "interp alias {} strings {} string lenght\nstrings abc",
        DiagCode::W001,
        "lenght",
    );
    let result = crate::analyser::Analyser::new().analyse(
        "interp alias {} strings {} string\nstrings {t} abc",
        "tcl8.6",
    );
    let diagnostic = result
        .diagnostics
        .iter()
        .find(|d| d.code == DiagCode::W145)
        .expect("ambiguous original selector");
    assert!(diagnostic.registry_source().is_some());
}

#[test]
fn original_deprecation_advice_preserves_whole_source_words_and_declines_captures() {
    // naming.diagnostic.registry-source-ownership
    // docs/design/analysis/name-resolution-proofs/diagnostic-registry-source-ownership.md
    // This proves source proposal geometry, not BIG-IP command equivalence.
    let source = "when HTTP_REQUEST {\n    ::::matchclass [HTTP::uri] starts_with {paths\nmore}\n}";
    let result = crate::analyser::Analyser::new().analyse(source, "f5-irules");
    let diagnostic = result
        .diagnostics
        .iter()
        .find(|d| d.code == DiagCode::Irule2001)
        .expect("selected source advice");
    assert!(diagnostic.registry_source().is_some());
    assert_eq!(diagnostic.fixes.len(), 1);
    assert_eq!(
        diagnostic.fixes[0].new_text,
        "class match [HTTP::uri] starts_with {paths\nmore}"
    );
    assert_eq!(
        &source[diagnostic.fixes[0].span.as_range()],
        "::::matchclass [HTTP::uri] starts_with {paths\nmore}"
    );
    assert_eq!(
        diagnostic.fixes[0].safety,
        crate::irules_checks::FixSafety::RequiresReview
    );
    // The default TMM source grammar does not enable {*} expansion. A single
    // opaque original operand has no declared two/three-word replacement.
    let incomplete = crate::analyser::Analyser::new()
        .analyse("when HTTP_REQUEST { matchclass $args }", "f5-irules");
    let diagnostic = incomplete
        .diagnostics
        .iter()
        .find(|d| d.code == DiagCode::Irule2001)
        .expect("advice has no incomplete-shape edit");
    assert_eq!(diagnostic.fixes, [] as [crate::irules_checks::CodeFix; 0]);
    let shadow = "proc matchclass args {}; matchclass item paths";
    assert!(!has_code(shadow, "f5-irules", "IRULE2001"));
}

#[test]
fn original_expression_diagnostics_keep_effective_roles_and_whole_written_words() {
    // naming.diagnostic.registry-source-ownership
    // docs/design/analysis/name-resolution-proofs/diagnostic-registry-source-ownership.md
    // This proves source grammar/coordinates, not native evaluation or rewrite equivalence.
    for source in [
        "interp alias {} calculate {} expr\ncalculate \"$a + $b\"",
        "rename expr calculate\ncalculate \"$a + $b\"",
        "::::expr \"$a + $b\"",
    ] {
        let result = Analyser::new().analyse(source, "tcl8.6");
        let diagnostic = result
            .diagnostics
            .iter()
            .find(|d| d.code == DiagCode::W100)
            .expect(source);
        assert_eq!(&source[diagnostic.span.as_range()], "\"$a + $b\"");
        let subject = diagnostic
            .registry_source()
            .expect("original expression source purpose");
        assert_eq!(
            subject.kind(),
            crate::analyser::RegistrySourceDiagnosticKind::Expression
        );
        assert_eq!(subject.span(), diagnostic.span);
        assert_eq!(subject.written_argument(), Some(0));
        assert_eq!(diagnostic.fixes[0].new_text, "{$a + $b}");
        // Bracing removes the second substitution round, so it can change
        // behaviour: with `a` holding `$x`, tclsh 9.0 evaluates the unbraced
        // form and raises on the braced one (`tests/it_4/fix_safety.rs`).
        assert_eq!(
            diagnostic.fixes[0].safety,
            crate::irules_checks::FixSafety::BehaviourHardening
        );
    }
    for source in [
        "interp alias {} calculate {} expr {$a + $b}\ncalculate",
        "proc expr args {}; expr $a",
        "expr {*}$arguments",
    ] {
        assert_eq!(count_code(source, "W100"), 0, "{source}");
    }
    let source = "expr \"$a +\" \"$b\"";
    let result = Analyser::new().analyse(source, "tcl8.6");
    let diagnostic = result
        .diagnostics
        .iter()
        .find(|d| d.code == DiagCode::W100)
        .expect("grouped source tail");
    assert_eq!(&source[diagnostic.span.as_range()], "\"$a +\" \"$b\"");
    assert!(
        diagnostic.fixes.is_empty(),
        "mixed grouping is not one quoted source word"
    );
}

#[test]
fn original_source_arity_and_relationships_keep_effective_counts_and_written_suffixes() {
    // naming.diagnostic.registry-source-ownership
    // docs/design/analysis/name-resolution-proofs/diagnostic-registry-source-ownership.md
    use crate::analyser::RegistrySourceDiagnosticKind;
    for source in [
        "::::set a 1 {extra value}",
        "rename set assign\nassign a 1 {extra value}",
        "interp alias {} assign {} set\nassign a 1 {extra value}",
    ] {
        let result = crate::analyser::Analyser::new().analyse(source, "tcl8.6");
        let diagnostic = result
            .diagnostics
            .iter()
            .find(|diagnostic| diagnostic.code == DiagCode::E003)
            .unwrap_or_else(|| panic!("missing source arity for {source:?}"));
        assert_eq!(&source[diagnostic.span.as_range()], "{extra value}");
        let subject = diagnostic.registry_source().unwrap_or_else(|| {
            panic!(
                "source signature subject missing for {source:?} at {:?}, code {:?}",
                diagnostic.span, diagnostic.code
            )
        });
        assert_eq!(subject.kind(), RegistrySourceDiagnosticKind::Arity);
        assert_eq!(subject.span(), diagnostic.span);
        assert_eq!(subject.written_argument(), Some(2));
        assert_eq!(diagnostic.fixes.len(), 1);
        assert_eq!(
            &source[diagnostic.fixes[0].span.as_range()],
            " {extra value}"
        );
        assert_eq!(
            diagnostic.fixes[0].safety,
            crate::irules_checks::FixSafety::RequiresReview
        );
    }
    for source in [
        "proc set {a b c} {}\nset a b c",
        "interp alias {} take {} string length\ntake {*}$values",
    ] {
        let result = crate::analyser::Analyser::new().analyse(source, "tcl8.6");
        assert!(!result.diagnostics.iter().any(|diagnostic| {
            diagnostic
                .registry_source()
                .is_some_and(|subject| subject.kind() == RegistrySourceDiagnosticKind::Arity)
        }));
    }
    let source = "::::regexp -expanded -line {x} x";
    let result = crate::analyser::Analyser::new().analyse(source, "tcl8.6");
    assert!(
        !result
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == DiagCode::E003)
    );
    for source in [
        "::::glob -path prefix -directory root *.tcl",
        "interp alias {} listfiles {} glob -directory root\nlistfiles -path prefix *.tcl",
    ] {
        let result = crate::analyser::Analyser::new().analyse(source, "tcl8.6");
        let diagnostic = result
            .diagnostics
            .iter()
            .find(|diagnostic| diagnostic.code == DiagCode::W147)
            .unwrap_or_else(|| panic!("missing source relationship for {source:?}"));
        let subject = diagnostic
            .registry_source()
            .expect("original relationship subject");
        assert_eq!(
            subject.kind(),
            RegistrySourceDiagnosticKind::ArgumentRelationship
        );
        assert_eq!(subject.span(), diagnostic.span);
        assert_eq!(diagnostic.fixes, [] as [crate::irules_checks::CodeFix; 0]);
    }
}

#[test]
fn declared_source_diagnostics_use_only_the_declared_roles_and_signature() {
    // naming.source.original-declared-command-word-contract
    // docs/design/analysis/name-resolution-proofs/source-original-declared-command-word-contract.md
    use crate::analyser::DeclaredSourceDiagnosticKind;
    for profile in ["tcl8.6", "tcl9.0", "tcl"] {
        let source = "# tcl-lsp: stubs-begin\n# tcl-lsp: stub select {?kind? expression:expr}\n# tcl-lsp: stubs-end\nselect \"$value + 1\"";
        let result = crate::analyser::Analyser::new().analyse(source, profile);
        let diagnostic = result
            .diagnostics
            .iter()
            .find(|diagnostic| diagnostic.code == DiagCode::W100)
            .unwrap_or_else(|| panic!("missing declared expression for {profile}"));
        assert_eq!(&source[diagnostic.span.as_range()], "\"$value + 1\"");
        let subject = diagnostic
            .declared_source()
            .expect("declaration source subject");
        assert_eq!(subject.kind(), DeclaredSourceDiagnosticKind::Expression);
        assert_eq!(subject.argument(), Some(0));
        assert_eq!(
            subject.words().descriptor().provenance(),
            tcl_dialect::model::Provenance::Document
        );
        assert_eq!(
            diagnostic.fixes[0].safety,
            crate::irules_checks::FixSafety::RequiresReview
        );
        assert!(diagnostic.registry_source().is_none());
        let source = "# tcl-lsp: stubs-begin\n# tcl-lsp: stub select {value}\n# tcl-lsp: stubs-end\nselect one {extra value}";
        let result = crate::analyser::Analyser::new().analyse(source, profile);
        let diagnostic = result
            .diagnostics
            .iter()
            .find(|diagnostic| diagnostic.code == DiagCode::E003)
            .unwrap_or_else(|| panic!("missing declared arity for {profile}"));
        assert_eq!(&source[diagnostic.span.as_range()], "{extra value}");
        assert_eq!(
            diagnostic.declared_source().unwrap().kind(),
            DeclaredSourceDiagnosticKind::Arity
        );
        assert_eq!(
            &source[diagnostic.fixes[0].span.as_range()],
            " {extra value}"
        );
    }
    for source in [
        "# tcl-lsp: stub select {expression:expr}\nselect \"$value + 1\"",
        "# tcl-lsp: stubs-begin\n# tcl-lsp: stub expr {value}\n# tcl-lsp: stubs-end\nexpr \"$value + 1\"",
        "# tcl-lsp: stubs-begin\n# tcl-lsp: stub select {value:expr}\n# tcl-lsp: stubs-end\nproc select {value} {}\nselect \"$value + 1\"",
        "# tcl-lsp: stubs-begin\n# tcl-lsp: stub select {value:expr}\n# tcl-lsp: stubs-end\nselect {*}$values",
    ] {
        let result = crate::analyser::Analyser::new().analyse(source, "tcl8.6");
        assert!(
            !result
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic.declared_source().is_some())
        );
    }
}

#[test]
fn original_lambda_and_indirect_builtin_arity_keep_value_and_operand_owners() {
    // naming.diagnostic.registry-source-ownership
    // docs/design/analysis/name-resolution-proofs/diagnostic-registry-source-ownership.md
    for source in [
        "::::apply {{a b} {return ok}} 1",
        "rename apply invoke\ninvoke {{a b} {return ok}} 1",
        "interp alias {} invoke {} apply\ninvoke {{a b} {return ok}} 1",
        "interp alias {} invoke {} apply {{a b} {return ok}}\ninvoke 1",
    ] {
        let result = crate::analyser::Analyser::new().analyse(source, "tcl8.6");
        let errors = result
            .diagnostics
            .iter()
            .filter(|diagnostic| diagnostic.code == DiagCode::E002)
            .collect::<Vec<_>>();
        assert_eq!(
            errors.len(),
            1,
            "one genuine source count owner: {source:?}"
        );
        assert_eq!(
            errors[0].registry_source().unwrap().kind(),
            crate::analyser::RegistrySourceDiagnosticKind::Arity
        );
    }
    for source in [
        "interp alias {} put {} set\nput a 1 extra",
        "rename set put\nput a 1 extra",
        "interp alias {} length {} string length\nlength a extra",
    ] {
        let result = crate::analyser::Analyser::new().analyse(source, "tcl8.6");
        let errors = result
            .diagnostics
            .iter()
            .filter(|diagnostic| diagnostic.code == DiagCode::E003)
            .collect::<Vec<_>>();
        assert_eq!(
            errors.len(),
            1,
            "builtin arity must not be duplicated by a reporting-name chase: {source:?}"
        );
        assert_eq!(&source[errors[0].span.as_range()], "extra");
        assert_eq!(errors[0].registry_source().unwrap().span(), errors[0].span);
    }
    for source in [
        "proc apply {a b} {}\napply {{a b} {return ok}} 1",
        "apply $lambda 1",
        "apply {{a b} {return ok}} {*}$values",
        "apply {{{a b c}} {return ok}} 1",
        "interp alias {} invoke {} apply $lambda\ninvoke 1",
    ] {
        let result = crate::analyser::Analyser::new().analyse(source, "tcl8.6");
        assert!(!result.diagnostics.iter().any(|diagnostic| {
            diagnostic.registry_source().is_some_and(|subject| {
                subject.kind() == crate::analyser::RegistrySourceDiagnosticKind::Arity
            })
        }));
    }
}

#[test]
fn declared_variable_name_positions_keep_only_supplied_contract_roles() {
    // naming.source.original-declared-command-word-contract
    // docs/design/analysis/name-resolution-proofs/source-original-declared-command-word-contract.md
    use crate::analyser::DeclaredSourceDiagnosticKind;
    for profile in ["tcl8.6", "tcl9.0", "tcl"] {
        let source = "# tcl-lsp: stubs-begin\n# tcl-lsp: stub store {?mode? destination:var value}\n# tcl-lsp: stubs-end\nstore $name payload";
        let analysis = Analyser::new().analyse(source, profile);
        let diagnostic = analysis
            .diagnostics
            .iter()
            .find(|diagnostic| diagnostic.code == DiagCode::W212)
            .unwrap_or_else(|| panic!("declared variable name missing for {profile}"));
        let subject = diagnostic
            .declared_source()
            .expect("authored declaration purpose");
        assert_eq!(subject.kind(), DeclaredSourceDiagnosticKind::VariableName);
        assert_eq!(subject.argument(), Some(0));
        assert_eq!(&source[subject.span().as_range()], "$name");
        assert!(diagnostic.registry_source().is_none());
        let source = "# tcl-lsp: stubs-begin\n# tcl-lsp: stub store {destination:var value}\n# tcl-lsp: stubs-end\nstore ${array}(index) payload";
        assert_eq!(
            Analyser::new()
                .analyse(source, profile)
                .diagnostics
                .iter()
                .filter(|diagnostic| diagnostic.code == DiagCode::W216)
                .count(),
            0,
            "indirect declaration variable name"
        );
    }
    for source in [
        "# tcl-lsp: stub store {destination:var value}\nstore $name payload",
        "# tcl-lsp: stubs-begin\n# tcl-lsp: stub store {destination value}\n# tcl-lsp: stubs-end\nstore $name payload",
        "# tcl-lsp: stubs-begin\n# tcl-lsp: stub store {destination:var value}\n# tcl-lsp: stubs-end\nproc store {destination value} {}\nstore $name payload",
        "# tcl-lsp: stubs-begin\n# tcl-lsp: stub store {destination:var value}\n# tcl-lsp: stubs-end\nstore {*}$names payload",
    ] {
        let analysis = Analyser::new().analyse(source, "tcl8.6");
        assert!(
            !analysis
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic.code == DiagCode::W212
                    && diagnostic.declared_source().is_some()),
            "{source}"
        );
    }
}

fn original_shape_advice_case(
    source: &str,
    code: DiagCode,
    kind: crate::analyser::RegistrySourceDiagnosticKind,
    written: &str,
    fixes: usize,
) {
    let analysis = Analyser::new().analyse(source, "tcl8.6");
    let found = analysis
        .diagnostics
        .iter()
        .filter(|diagnostic| diagnostic.code == code)
        .collect::<Vec<_>>();
    assert_eq!(found.len(), 1, "{source}");
    let diagnostic = found[0];
    assert_eq!(&source[diagnostic.span.as_range()], written, "{source}");
    assert_eq!(diagnostic.registry_source().unwrap().kind(), kind);
    assert_eq!(diagnostic.fixes.len(), fixes, "{source}");
}

#[test]
fn original_append_advice_keeps_effective_roles_and_whole_written_operands() {
    // naming.diagnostic.registry-source-ownership
    // docs/design/analysis/name-resolution-proofs/diagnostic-registry-source-ownership.md
    use crate::analyser::RegistrySourceDiagnosticKind as Kind;
    for (source, code, kind, written, fixes) in [
        (
            "::append out \" $item\"",
            DiagCode::W104,
            Kind::AppendList,
            "\" $item\"",
            1,
        ),
        (
            "rename append add\nadd out \" $item\"",
            DiagCode::W104,
            Kind::AppendList,
            "\" $item\"",
            1,
        ),
        (
            "interp alias {} add {} append out\nadd \" $item\"",
            DiagCode::W104,
            Kind::AppendList,
            "\" $item\"",
            0,
        ),
    ] {
        original_shape_advice_case(source, code, kind, written, fixes);
    }
}

#[test]
fn original_case_advice_keeps_effective_roles_and_whole_written_operands() {
    // naming.diagnostic.registry-source-ownership
    // docs/design/analysis/name-resolution-proofs/diagnostic-registry-source-ownership.md
    use crate::analyser::RegistrySourceDiagnosticKind as Kind;
    for (source, code, kind, written, fixes) in [
        (
            "::switch $value $pattern $body",
            DiagCode::W106,
            Kind::CaseBody,
            "$body",
            0,
        ),
        (
            "rename switch choose\nchoose $value a \"$body\"",
            DiagCode::W106,
            Kind::CaseBody,
            "\"$body\"",
            0,
        ),
        (
            "interp alias {} choose {} switch -regexp subject\nchoose a $body",
            DiagCode::W106,
            Kind::CaseBody,
            "$body",
            0,
        ),
    ] {
        original_shape_advice_case(source, code, kind, written, fixes);
    }
}

#[test]
fn original_option_only_advice_keeps_effective_roles_and_whole_written_operands() {
    // naming.diagnostic.registry-source-ownership
    // docs/design/analysis/name-resolution-proofs/diagnostic-registry-source-ownership.md
    use crate::analyser::RegistrySourceDiagnosticKind as Kind;
    for (source, code, kind, written, fixes) in [
        (
            "::unset -nocomplain --",
            DiagCode::W217,
            Kind::OptionOnly,
            "-nocomplain --",
            1,
        ),
        (
            "rename unset clean\nclean --",
            DiagCode::W217,
            Kind::OptionOnly,
            "--",
            1,
        ),
        (
            "interp alias {} clean {} unset -nocomplain\nclean",
            DiagCode::W217,
            Kind::OptionOnly,
            "clean",
            0,
        ),
    ] {
        original_shape_advice_case(source, code, kind, written, fixes);
    }
}

#[test]
fn original_shape_advice_withdraws_unknown_geometry_and_known_shadows() {
    // naming.diagnostic.registry-source-ownership
    // docs/design/analysis/name-resolution-proofs/diagnostic-registry-source-ownership.md
    for (source, code) in [
        (
            "proc append {args} {}\nappend out \" value\"",
            DiagCode::W104,
        ),
        // tclsh8.6.18/9.0.4: the captured padded append runs without error.
        ("interp alias {} add {} append out { }\nadd", DiagCode::W104),
        ("append {*}$prefix \" value\"", DiagCode::W104),
        (
            "proc switch {args} {}\nswitch $value a $body",
            DiagCode::W106,
        ),
        ("switch $value {*}$arms", DiagCode::W106),
        (
            "interp alias {} choose {} switch subject a body\nchoose",
            DiagCode::W106,
        ),
        ("proc unset {args} {}\nunset --", DiagCode::W217),
        ("unset {*}$options", DiagCode::W217),
        ("unset -nocomplain -nocomplain", DiagCode::W217),
    ] {
        assert!(
            !Analyser::new()
                .analyse(source, "tcl8.6")
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic.code == code),
            "{source}"
        );
    }
    let analysis = Analyser::new().analyse("unset -nocomplain -nocomplain", "jim");
    assert_eq!(
        analysis
            .diagnostics
            .iter()
            .filter(|diagnostic| diagnostic.code == DiagCode::W217)
            .count(),
        1
    );
}

#[test]
fn original_pattern_advice_keeps_selected_roles_and_complete_written_spans() {
    // naming.diagnostic.registry-source-ownership
    // docs/design/analysis/name-resolution-proofs/diagnostic-registry-source-ownership.md
    use crate::analyser::RegistrySourceDiagnosticKind as Kind;
    for source in [
        "::regexp -- \"prefix$pattern\" $subject",
        "rename regexp match\nmatch -- \"prefix$pattern\" $subject",
        "interp alias {} match {} regexp --\nmatch \"prefix$pattern\" $subject",
    ] {
        let analysis = Analyser::new().analyse(source, "tcl8.6");
        let found = analysis
            .diagnostics
            .iter()
            .filter(|diagnostic| diagnostic.code == DiagCode::W306)
            .collect::<Vec<_>>();
        assert_eq!(found.len(), 1, "{source}");
        let diagnostic = found[0];
        assert_eq!(&source[diagnostic.span.as_range()], "\"prefix$pattern\"");
        assert_eq!(
            diagnostic.registry_source().unwrap().kind(),
            Kind::PatternSubstitution
        );
    }
    for source in [
        "regexp \"prefix$pattern\" $subject",
        "regexp -- \"$pattern\" $subject",
        "regexp -- {prefix$pattern} $subject",
        "regexp -- \"\\[literal\\]\" $subject",
        "proc regexp {args} {}\nregexp -- \"prefix$pattern\" $subject",
        "regexp {*}$options \"prefix$pattern\" $subject",
        "interp alias {} match {} regexp -- {prefix$pattern}\nmatch $subject",
    ] {
        assert!(
            !Analyser::new()
                .analyse(source, "tcl8.6")
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic.code == DiagCode::W306),
            "{source}"
        );
    }
}

#[test]
fn w216_literal_root_replacement_preserves_bytes_and_index_source() {
    // naming.diagnostics.original-w216-literal-source-replacement
    // docs/design/analysis/name-resolution-proofs/diagnostic-original-w216-literal-source-replacement.md
    // Actual diagnostic/fix source projection only. No Native name/cell,
    // completed read, inserted ::set identity or index evaluation is granted.
    for dialect in [
        "tcl", "tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1", "jimtcl",
    ] {
        for (root, quoted) in [
            ("$arr", "\\$arr"),
            ("a\"b", "a\\\"b"),
            ("a[b]", "a\\[b\\]"),
            ("a\\b", "a\\\\b"),
            ("é", "é"),
        ] {
            let source = format!("puts ${{{root}($key)}}");
            let analysis = Analyser::new().analyse(&source, dialect);
            let diagnostic = analysis
                .diagnostics
                .iter()
                .find(|d| d.code == DiagCode::W216)
                .unwrap();
            assert_eq!(
                &source[diagnostic.span.as_range()],
                format!("${{{root}($key)}}")
            );
            assert_eq!(
                diagnostic.fixes[0].new_text,
                format!("[::set \"{quoted}($key)\"]")
            );
            assert_eq!(
                diagnostic.fixes[0].safety,
                crate::irules_checks::FixSafety::RequiresReview
            );
            assert!(
                diagnostic
                    .message
                    .contains("stock variable-reading meaning")
            );
        }
        for (source, expected) in [
            (r#"puts ${$arr($key"x")}"#, r#"[::set "\$arr($key\"x\")"]"#),
            (
                r#"puts ${$arr([format "%s" $key])}"#,
                r#"[::set "\$arr([format "%s" $key])"]"#,
            ),
        ] {
            let analysis = Analyser::new().analyse(source, dialect);
            let diagnostic = analysis
                .diagnostics
                .iter()
                .find(|d| d.code == DiagCode::W216)
                .unwrap();
            assert_eq!(diagnostic.fixes[0].new_text, expected);
        }
        let analysis = Analyser::new().analyse("puts ${$arr($key}", dialect);
        assert!(
            !analysis
                .diagnostics
                .iter()
                .any(|d| d.code == DiagCode::W216)
        );
        let analysis = Analyser::new().analyse("puts ${a(b}(key)", dialect);
        let diagnostic = analysis
            .diagnostics
            .iter()
            .find(|d| d.code == DiagCode::W216)
            .unwrap();
        assert!(
            diagnostic.fixes.is_empty(),
            "literal '(' cannot donate a combined array root"
        );
    }
}

#[test]
fn w216_fixed_proposals_match_captured_public_value_examples() {
    // naming.variable.original-w216-literal-replacement-public-values
    // docs/design/analysis/name-resolution-proofs/variable-original-w216-literal-replacement-public-values.md
    // Archive association and genuine proposal spelling controls. The native
    // rows measure these eight stock-set value contexts, not editor safety,
    // original native name identity or runtime availability of an inserted head.
    let probe = include_str!(
        "../../../../../tcl-registry/tests/data/native_w216_literal_replacement_value325/probe.tcl"
    );
    for dialect in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1", "jimtcl"] {
        for source in [
            "puts ${$arr($key)}",
            "puts ${a\"b($key)}",
            "puts ${a[b]($key)}",
            "puts ${a\\b($key)}",
            "puts ${é($key)}",
            "puts ${$arr($key\"x\")}",
            "puts ${$arr([format \"%s\" $key])}",
            "puts ${a\nb($key)}",
        ] {
            let analysis = Analyser::new().analyse(source, dialect);
            let diagnostic = analysis
                .diagnostics
                .iter()
                .find(|d| d.code == DiagCode::W216)
                .unwrap();
            let proposal = &diagnostic.fixes[0].new_text;
            assert!(
                probe.contains(proposal),
                "{dialect} {source:?}: {proposal:?}"
            );
        }
    }
}

#[test]
fn w216_public_value_capture_keeps_whole_provider_rows() {
    // naming.variable.original-w216-literal-replacement-public-values
    // docs/design/analysis/name-resolution-proofs/variable-original-w216-literal-replacement-public-values.md
    // Stored provider observations are checked without rerunning native Tcl
    // or treating their value equality as private identity/editor permission.
    let expected = concat!(
        "dollar 0 1 1 VALUE_dollar\n",
        "quote 0 1 1 VALUE_quote\n",
        "brackets 0 1 1 VALUE_brackets\n",
        "backslash 0 1 1 VALUE_backslash\n",
        "unicode 0 1 1 VALUE_unicode\n",
        "quoted_index 0 1 1 VALUE_quoted_index\n",
        "nested_command 0 1 1 VALUE_nested_command\n",
        "split_root 0 1 1 VALUE_split_root\n",
        "replaced_set CHANGED\n",
    );
    for (provider, stdout, stderr) in [
        (
            "8.4.20",
            include_str!(
                "../../../../../tcl-registry/tests/data/native_w216_literal_replacement_value325/8.4.20/stdout"
            ),
            include_str!(
                "../../../../../tcl-registry/tests/data/native_w216_literal_replacement_value325/8.4.20/stderr"
            ),
        ),
        (
            "8.5.19",
            include_str!(
                "../../../../../tcl-registry/tests/data/native_w216_literal_replacement_value325/8.5.19/stdout"
            ),
            include_str!(
                "../../../../../tcl-registry/tests/data/native_w216_literal_replacement_value325/8.5.19/stderr"
            ),
        ),
        (
            "8.6.18",
            include_str!(
                "../../../../../tcl-registry/tests/data/native_w216_literal_replacement_value325/8.6.18/stdout"
            ),
            include_str!(
                "../../../../../tcl-registry/tests/data/native_w216_literal_replacement_value325/8.6.18/stderr"
            ),
        ),
        (
            "9.0.4",
            include_str!(
                "../../../../../tcl-registry/tests/data/native_w216_literal_replacement_value325/9.0.4/stdout"
            ),
            include_str!(
                "../../../../../tcl-registry/tests/data/native_w216_literal_replacement_value325/9.0.4/stderr"
            ),
        ),
        (
            "9.1.0",
            include_str!(
                "../../../../../tcl-registry/tests/data/native_w216_literal_replacement_value325/9.1.0/stdout"
            ),
            include_str!(
                "../../../../../tcl-registry/tests/data/native_w216_literal_replacement_value325/9.1.0/stderr"
            ),
        ),
        (
            "jim",
            include_str!(
                "../../../../../tcl-registry/tests/data/native_w216_literal_replacement_value325/jim/stdout"
            ),
            include_str!(
                "../../../../../tcl-registry/tests/data/native_w216_literal_replacement_value325/jim/stderr"
            ),
        ),
    ] {
        assert_eq!(stdout, expected, "{provider}");
        assert!(stderr.is_empty(), "{provider}");
    }
}
