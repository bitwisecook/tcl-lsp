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

//! Quick fixes for published diagnostics, requested with the diagnostics in the `codeAction` context as an editor does.

use super::*;

#[test]
fn test_w100_offers_brace_wrap() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let diags = lsp.open_ready(&uri, "if $a {puts x}\n");
    let w100 = with_code(&diags, "W100");
    assert!(
        !w100.is_empty(),
        "expected a W100 diagnostic to drive the quick fix"
    );
    let actions = lsp.code_actions(&uri, range((0, 0), (0, 14)), json!(w100));
    assert!(
        titles(&actions)
            .iter()
            .any(|t| t.to_lowercase().contains("brace"))
    );
    assert!(new_texts(&actions).iter().any(|nt| nt.contains("{$a}")));
}

#[test]
fn test_w003_offers_lsearch_fix_for_in() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let diags = lsp.open_ready(&uri, "# tcl-dialect: tcl8.4\nexpr {2 in {1 2 3}}\n");
    let w003 = with_code(&diags, "W003");
    assert_eq!(w003.len(), 1, "{diags:?}");
    let actions = lsp.code_actions(&uri, range((1, 0), (1, 19)), json!(w003));
    assert!(
        new_texts(&actions)
            .iter()
            .any(|nt| nt == "([lsearch -exact {1 2 3} 2] >= 0)"),
        "{actions:?}"
    );
}

#[test]
fn test_w003_offers_string_compare_fix_for_lt() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let diags = lsp.open_ready(&uri, "# tcl-dialect: tcl8.4\nif {$x lt $y} { puts hi }\n");
    let w003 = with_code(&diags, "W003");
    assert_eq!(w003.len(), 1, "{diags:?}");
    let actions = lsp.code_actions(&uri, range((1, 0), (1, 26)), json!(w003));
    assert!(
        new_texts(&actions)
            .iter()
            .any(|nt| nt == "([string compare $x $y] < 0)"),
        "{actions:?}"
    );
}

#[test]
fn test_w003_no_fix_when_operator_nested_in_larger_expression() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let diags = lsp.open_ready(
        &uri,
        "# tcl-dialect: tcl8.4\nif {$a in $b && $c} { puts hi }\n",
    );
    let w003 = with_code(&diags, "W003");
    assert_eq!(w003.len(), 1, "{diags:?}");
    let actions = lsp.code_actions(&uri, range((1, 0), (1, 32)), json!(w003));
    assert!(
        new_texts(&actions).iter().all(|nt| !nt.contains("lsearch")),
        "a nested occurrence must not offer the lsearch rewrite: {actions:?}"
    );
}

/// The W302 quick-fix actions the server offers for `source`.
///
/// The cursor sits on the `catch` keyword — where the diagnostic anchors and
/// where an editor's lightbulb is invoked — which is precisely the position
/// the broken anchor derived its (wrong) insertion point from.
fn w302_actions(lsp: &mut Lsp, uri: &str, source: &str) -> Value {
    let diags = lsp.open_ready(uri, source);
    let w302 = with_code(&diags, "W302");
    assert!(!w302.is_empty(), "expected a W302 diagnostic: {diags:?}");
    code_actions_only(lsp, uri, range((0, 0), (0, 4)), json!(w302), &["quickfix"])
}

#[test]
fn test_w302_result_capture_action_applies_after_the_body() {
    // The action must insert the right text at the right place — inserting
    // it wrongly would produce `catch result {error oops}`, and C Tcl would
    // then evaluate the script `result` (completion code 1, `invalid
    // command name "result"`) and store it in a variable named `error
    // oops`. Asserting the *applied document* is what catches that;
    // checking only the inserted string and that the range was zero-width
    // would not.
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let source = "catch {error oops}\n";
    let actions = w302_actions(&mut lsp, &uri, source);
    assert_fix_applies(
        &actions,
        source,
        "Add catch result variable(s)",
        "catch {error oops} result\n",
    );
    assert_fix_applies(
        &actions,
        source,
        "Add catch result + options variable(s)",
        "catch {error oops} result options\n",
    );
}

#[test]
fn test_w302_result_capture_action_applies_after_a_multiline_body() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let source = "catch {\n    error oops\n}\n";
    let actions = w302_actions(&mut lsp, &uri, source);
    assert_fix_applies(
        &actions,
        source,
        "Add catch result variable(s)",
        "catch {\n    error oops\n} result\n",
    );
}

#[test]
fn test_w302_result_capture_action_keeps_a_trailing_comment() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let source = "catch {error oops} ;# best effort\n";
    let actions = w302_actions(&mut lsp, &uri, source);
    assert_fix_applies(
        &actions,
        source,
        "Add catch result variable(s)",
        "catch {error oops} result ;# best effort\n",
    );
}

#[test]
fn test_w302_result_capture_action_does_not_overshoot_an_empty_body() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let source = "catch {}\n";
    let actions = w302_actions(&mut lsp, &uri, source);
    assert_fix_applies(
        &actions,
        source,
        "Add catch result variable(s)",
        "catch {} result\n",
    );
}

#[test]
fn test_w302_no_fix_when_result_present() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    lsp.open_ready(&uri, "catch {error oops} result\n");
    // Fabricate the W302 range an editor would send; the server must still
    // decline because the source already captures a result variable.
    let d = json!({
        "range": { "start": { "line": 0, "character": 0 }, "end": { "line": 0, "character": 4 } },
        "code": "W302",
        "message": "catch without a result variable silently swallows errors.",
        "source": "tcl-lsp",
    });
    let actions = code_actions_only(
        &mut lsp,
        &uri,
        range((0, 0), (0, 4)),
        json!([d]),
        &["quickfix"],
    );
    assert_eq!(new_texts(&actions), [] as [std::string::String; 0]);
}

#[test]
fn test_e004_extra_words_offers_merge_into_body_fix() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let diags = lsp.open_ready(&uri, "if {1} {a} {b} {c}\n");
    let e004 = with_code(&diags, "E004");
    assert!(!e004.is_empty(), "expected an E004 diagnostic");
    let actions = code_actions_only(
        &mut lsp,
        &uri,
        range((0, 15), (0, 18)),
        json!(e004),
        &["quickfix"],
    );
    assert!(
        titles(&actions)
            .iter()
            .any(|t| t.to_lowercase().contains("merge"))
    );
    assert!(new_texts(&actions).iter().any(|nt| nt == "{{b} {c}}"));
}

#[test]
fn test_e004_dangling_elseif_offers_remove_clause_fix() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let diags = lsp.open_ready(&uri, "if {1} {a} elseif\n");
    let e004 = with_code(&diags, "E004");
    assert!(!e004.is_empty(), "expected an E004 diagnostic");
    let actions = code_actions_only(
        &mut lsp,
        &uri,
        range((0, 12), (0, 17)),
        json!(e004),
        &["quickfix"],
    );
    assert!(
        titles(&actions)
            .iter()
            .any(|t| t.to_lowercase().contains("remove"))
    );
    assert!(new_texts(&actions).iter().any(String::is_empty));
}

#[test]
fn test_e004_missing_first_clause_offers_no_fix() {
    // `if {1}` has no well-formed prefix to fall back to — no fix.
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let diags = lsp.open_ready(&uri, "if {1}\n");
    let e004 = with_code(&diags, "E004");
    assert!(!e004.is_empty(), "expected an E004 diagnostic");
    let actions = code_actions_only(
        &mut lsp,
        &uri,
        range((0, 3), (0, 6)),
        json!(e004),
        &["quickfix"],
    );
    assert!(new_texts(&actions).is_empty(), "got {actions:?}");
}

#[test]
fn test_w004_offers_remove_option_fix() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let src = "# tcl-dialect: tcl8.6\nlsearch -stride 2 {a b c d} b\n";
    let diags = lsp.open_ready(&uri, src);
    let w004 = with_code(&diags, "W004");
    assert!(!w004.is_empty(), "expected a W004 diagnostic: {diags:?}");
    let actions = code_actions_only(
        &mut lsp,
        &uri,
        range((1, 0), (1, 30)),
        json!(w004),
        &["quickfix"],
    );
    assert!(
        titles(&actions).iter().any(|t| t.contains("-stride")),
        "expected a 'Remove ...-stride...' quick fix: {:?}",
        titles(&actions)
    );
    assert!(
        new_texts(&actions).iter().any(String::is_empty),
        "the remove-option fix deletes text (empty newText): {:?}",
        new_texts(&actions)
    );
}

/// T102 (option injection) is dialect-general, not iRules-specific, so its
/// `--`-insertion fix must reach the code-action response for a plain `tcl`
/// document too — not only under `f5-irules` (see `irules::t102_insert_double_dash`).
#[test]
fn test_t102_insert_double_dash_default_dialect() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let diags = lsp.open_ready(
        &uri,
        "set pattern [gets stdin]\nregexp $pattern $haystack\n",
    );
    let t102 = with_code(&diags, "T102");
    assert!(
        !t102.is_empty(),
        "expected a T102 diagnostic to drive the quick fix, got {diags:?}"
    );
    let t102_range = t102[0]["range"].clone();
    let actions = code_actions_only(&mut lsp, &uri, t102_range, json!(t102), &["quickfix"]);
    let fixes: Vec<Value> = actions
        .as_array()
        .unwrap_or(&Vec::new())
        .iter()
        .filter(|a| action_title(a).contains("--"))
        .cloned()
        .collect();
    assert!(
        !fixes.is_empty(),
        "expected at least one '--' quick fix, got {actions:?}"
    );
    assert!(new_texts(&json!(fixes)).iter().any(|s| s == "-- "));
}

/// The `set matched [regexp $pattern $s]` capture idiom must fire T102
/// exactly like the bare `regexp $pattern $s` call it wraps — a false
/// negative in the taint pass's statement-shape handling, not a code-action
/// concern, but only observable end-to-end through published diagnostics.
#[test]
fn test_t102_fires_for_assign_value_wrapped_call() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let diags = lsp.open_ready(
        &uri,
        "set pattern [gets stdin]\nset result [regexp $pattern $haystack]\n",
    );
    let t102 = with_code(&diags, "T102");
    assert!(
        !t102.is_empty(),
        "expected T102 for tainted $pattern inside `set result [regexp ...]`, got {diags:?}"
    );
}

#[test]
fn test_simple_continuation_fix() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    lsp.open_ready(&uri, "# hello \\\nworld");
    let d = diag("W115", "Backslash-newline in comment", (0, 0), (1, 4));
    let actions = lsp.code_actions(&uri, range((0, 0), (1, 4)), json!([d]));
    let fixes: Vec<Value> = actions
        .as_array()
        .unwrap_or(&Vec::new())
        .iter()
        .filter(|a| action_title(a).contains("per-line"))
        .cloned()
        .collect();
    assert_eq!(fixes.len(), 1);
    let snippet = &new_texts(&json!(fixes))[0];
    assert!(snippet.contains("# hello"));
    assert!(snippet.contains("# world"));
    assert!(!snippet.contains('\\'));
}

#[test]
fn no_conversion_for_a_disabled_w115() {
    // The lightbulb offers the conversion only for a *shown* W115 —
    // a layer that turns the code off silences the action too, even though
    // the comment shape below is still detectable.
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    lsp.apply_configuration_settle(
        json!({ "diagnostics": { "W115": false } }),
        &uri,
        |config| {
            config["disabled_diagnostics"]
                .as_array()
                .is_some_and(|codes| codes.iter().any(|code| code == "W115"))
        },
    );
    lsp.open_ready(&uri, "# trailing \\\nset x 1\n");
    let actions = lsp.code_actions(&uri, range((0, 0), (0, 0)), json!([]));
    assert!(
        titles(&actions).iter().all(|t| !t.contains("per-line")),
        "{actions:?}"
    );
}

#[test]
fn test_chained_continuation_fix() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    lsp.open_ready(&uri, "# line1 \\\nline2 \\\nline3");
    let d = diag("W115", "Backslash-newline in comment", (0, 0), (2, 4));
    let actions = lsp.code_actions(&uri, range((0, 0), (2, 4)), json!([d]));
    let fixes: Vec<Value> = actions
        .as_array()
        .unwrap_or(&Vec::new())
        .iter()
        .filter(|a| action_title(a).contains("per-line"))
        .cloned()
        .collect();
    assert_eq!(fixes.len(), 1);
    let snippet = &new_texts(&json!(fixes))[0];
    assert!(snippet.contains("# line1"));
    assert!(snippet.contains("# line2"));
    assert!(snippet.contains("# line3"));
}

#[test]
fn test_already_commented_continuation_not_doubled() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    lsp.open_ready(&uri, "# line1 \\\n# line2");
    let d = diag("W115", "Backslash-newline in comment", (0, 0), (1, 6));
    let actions = lsp.code_actions(&uri, range((0, 0), (1, 6)), json!([d]));
    let fixes: Vec<Value> = actions
        .as_array()
        .unwrap_or(&Vec::new())
        .iter()
        .filter(|a| action_title(a).contains("per-line"))
        .cloned()
        .collect();
    let texts = new_texts(&json!(fixes));
    let lines: Vec<&str> = texts[0].split('\n').collect();
    assert_eq!(lines[0], "# line1");
    assert_eq!(lines[1], "# line2");
}

#[test]
fn test_eval_string_to_list_action() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let diags = lsp.open_ready(&uri, "eval \"process $x\"\n");
    let w101 = with_code(&diags, "W101");
    assert_ne!(w101, [] as [serde_json::Value; 0]);
    let actions = lsp.code_actions(&uri, range((0, 0), (0, 17)), json!(w101));
    assert!(
        new_texts(&actions)
            .iter()
            .any(|s| s.contains("[list process $x]"))
    );
}

#[test]
fn test_no_profiles_action_for_tcl_dialect() {
    // On the plain-Tcl server, a proc never gets an F5 "# Profiles:" header
    // action.
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    lsp.open_ready(&uri, "proc hello {} { puts hi }\n");
    let actions = lsp.code_actions(&uri, range((0, 0), (0, 0)), json!([]));
    let sa = kinds(&actions, "source");
    assert!(sa.iter().all(|a| !action_title(a).contains("Profiles")));
}

#[test]
fn test_e100_offers_insert_bracket_before_known_command() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let diags = lsp.open_ready(&uri, "puts string]\n");
    let e100 = with_code(&diags, "E100");
    assert!(!e100.is_empty(), "expected an E100 diagnostic");
    let actions = code_actions_only(
        &mut lsp,
        &uri,
        range((0, 0), (0, 12)),
        json!(e100),
        &["quickfix"],
    );
    assert!(
        titles(&actions)
            .iter()
            .any(|t| t.to_lowercase().contains("insert") && t.contains('[')),
        "{:?}",
        titles(&actions)
    );
    assert!(new_texts(&actions).iter().any(|s| s == "["));
}

#[test]
fn test_e100_offers_insert_bracket_before_user_proc() {
    // The bracket-insertion heuristic must recognise a call to an
    // already-declared user proc, not just a registry builtin.
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let src = "proc myHelper {a b} {return $a}\nset y myHelper arg1 arg2]\n";
    let diags = lsp.open_ready(&uri, src);
    let e100 = with_code(&diags, "E100");
    assert!(!e100.is_empty(), "expected an E100 diagnostic");
    let actions = code_actions_only(
        &mut lsp,
        &uri,
        range((1, 0), (1, 26)),
        json!(e100),
        &["quickfix"],
    );
    assert!(new_texts(&actions).iter().any(|s| s == "["));
}

#[test]
fn test_e100_no_fix_offered_without_known_command_or_overflow() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let diags = lsp.open_ready(&uri, "set x blah]\n");
    let e100 = with_code(&diags, "E100");
    assert!(!e100.is_empty(), "expected an E100 diagnostic");
    let actions = code_actions_only(
        &mut lsp,
        &uri,
        range((0, 0), (0, 11)),
        json!(e100),
        &["quickfix"],
    );
    // No recognisable command / arity overflow, so the E100-specific
    // bracket-insertion fix must not be offered (other, unrelated
    // quickfixes for the range — e.g. a W123 "did you mean" suggestion
    // on the unresolved `blah` command — may still appear).
    assert!(
        !titles(&actions)
            .iter()
            .any(|t| t.to_lowercase().contains("insert") && t.contains('[')),
        "{:?}",
        titles(&actions)
    );
}

#[test]
fn test_e102_offers_remove_extra_brace_for_isolated_line() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let diags = lsp.open_ready(&uri, "set x 1\n}\n");
    let e102 = with_code(&diags, "E102");
    assert!(!e102.is_empty(), "expected an E102 diagnostic");
    let actions = code_actions_only(
        &mut lsp,
        &uri,
        range((1, 0), (1, 1)),
        json!(e102),
        &["quickfix"],
    );
    assert!(
        titles(&actions)
            .iter()
            .any(|t| t.to_lowercase().contains("remove")),
        "{:?}",
        titles(&actions)
    );
    assert!(new_texts(&actions).iter().any(String::is_empty));
}

#[test]
fn test_e102_no_fix_offered_for_embedded_brace() {
    // `foo}bar` is flagged but not auto-fixed — deleting one character
    // out of a bareword isn't a safe guess the way an isolated stray-brace
    // line is.
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let diags = lsp.open_ready(&uri, "set x foo}bar\n");
    let e102 = with_code(&diags, "E102");
    assert!(!e102.is_empty(), "expected an E102 diagnostic");
    let actions = code_actions_only(
        &mut lsp,
        &uri,
        range((0, 0), (0, 13)),
        json!(e102),
        &["quickfix"],
    );
    assert!(
        !titles(&actions)
            .iter()
            .any(|t| t.to_lowercase().contains("remove") && t.contains('}')),
        "{:?}",
        titles(&actions)
    );
}

#[test]
fn test_s100_offers_noqa_suppress_action() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let diags = lsp.open_ready(&uri, "set x hello\nincr x\n");
    let s100 = with_code(&diags, "S100");
    assert!(!s100.is_empty(), "expected an S100 to drive the quick fix");
    let actions = lsp.code_actions(&uri, range((1, 0), (1, 6)), json!(s100));
    assert!(
        titles(&actions)
            .iter()
            .any(|t| t == "Suppress S100 with a noqa comment"),
        "expected an S100 noqa-suppress action, got {actions:?}"
    );
    assert!(new_texts(&actions).iter().any(|s| s == "# noqa: S100\n"));
}

/// (Regression guard) `eq` against a numeric literal never offers a
/// numeric-comparison rewrite, even when both operands are provably
/// numeric — `eq`/`ne`/`lt`/`le`/`gt`/`ge` compare the operands' *string*
/// representations, never their numeric value (`"10" lt "2"` is true,
/// `10 < 2` is false), so no such rewrite is ever semantics-preserving. The
/// S100 diagnostic still fires, and the generic noqa suppress is still
/// offered — only the (unsound) mechanical rewrite is withheld.
#[test]
fn test_s100_eq_numeric_offers_no_numeric_comparison_fix() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let diags = lsp.open_ready(&uri, "set x 42\nset z [expr {$x eq 42}]\n");
    let s100 = with_code(&diags, "S100");
    assert!(!s100.is_empty(), "expected an S100 to drive the quick fix");
    let actions = lsp.code_actions(&uri, range((1, 0), (1, 25)), json!(s100));
    assert!(
        !titles(&actions)
            .iter()
            .any(|t| t.contains("numeric comparison")),
        "must never offer a semantics-changing numeric rewrite, got {actions:?}"
    );
    assert!(
        titles(&actions)
            .iter()
            .any(|t| t == "Suppress S100 with a noqa comment"),
        "expected the noqa suppress action to still be offered, got {actions:?}"
    );
}

/// A shown optimiser rewrite is a code action like any other
/// (`docs/design/compiler/diagnostic-policy.md` § Adapters, code actions): the
/// O101 fold of `set x [expr {1 + 2}]` is offered as a quick-fix whose edit
/// rewrites the statement, under a profile that shows it — and not under the
/// default `readability` profile, which keeps constant folding off.
#[test]
fn an_optimiser_rewrite_is_offered_as_a_quick_fix() {
    let source = "set x [expr {1 + 2}]\n";
    let fold = |actions: &Value| {
        kinds(actions, "quickfix")
            .iter()
            .any(|action| new_texts(&json!([action])).iter().any(|t| t == "set x 3"))
    };

    let mut lsp = Lsp::with_config(json!({ "optimiser": { "profile": "full" } }));
    let uri = unique_uri("tcl");
    let diags = lsp.open_ready(&uri, source);
    let actions = lsp.code_actions(&uri, range((0, 0), (0, 20)), json!(diags));
    assert!(
        fold(&actions),
        "the shown O101 rewrite is a quick-fix: {actions:?}"
    );

    let mut default = Lsp::tcl();
    let uri = unique_uri("tcl");
    let diags = default.open_ready(&uri, source);
    let actions = default.code_actions(&uri, range((0, 0), (0, 20)), json!(diags));
    assert!(
        !fold(&actions),
        "`readability` keeps the fold off, so nothing offers it: {actions:?}"
    );
}

/// A fix is offered for a shown finding and for no other: a `# noqa: W100`
/// over the command leaves its line with no brace refactor, where the same
/// command without the directive offers one.
#[test]
fn no_quick_fix_for_a_finding_a_noqa_silences() {
    const BRACE: &str = "Brace expr for safety and performance";
    let mut lsp = Lsp::tcl();

    let control = unique_uri("tcl");
    let diags = lsp.open_ready(&control, "set a 1\nset y [expr $a + 1]\n");
    let actions = lsp.code_actions(&control, range((1, 0), (1, 19)), json!(diags));
    assert!(
        titles(&actions).iter().any(|t| t == BRACE),
        "the unmarked command offers the refactor: {actions:?}"
    );

    let marked = unique_uri("tcl");
    let diags = lsp.open_ready(&marked, "set a 1\n# noqa: W100\nset y [expr $a + 1]\n");
    let actions = lsp.code_actions(&marked, range((2, 0), (2, 19)), json!(diags));
    assert!(
        !titles(&actions).iter().any(|t| t == BRACE),
        "a silenced W100 offers no refactor: {actions:?}"
    );
}
