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

//! Analyser-carried quick fixes, pinned by title and applied edit, and the evidence-gated package suggestion.

use super::*;

// Analyser-carried quick fixes, end-to-end: each case drives the published
// diagnostic back into `codeAction` (exactly what an editor does) and pins
// the fix by title plus the *applied* edit — never the diagnostic message,
// which is free to be reworded. Covers W120 / W123 / W001 / W213 / W216 /
// W217, plus the style rewrites W104 / W114 / W201 (the last lifted from
// the compiler-checks stream rather than the analyser); the iRules-only
// analyser fixes (IRULE2001/5005/6001) run against the dedicated iRules
// server in `irules.rs`.

/// The quickfix-only actions for `code`, requested over the first such
/// diagnostic's own range with the published diagnostics as context —
/// panics (with the full diagnostic set) when the diagnostic is absent.
fn quickfixes_for_code(lsp: &mut Lsp, uri: &str, diags: &[Value], code: &str) -> Value {
    let matching = with_code(diags, code);
    assert!(
        !matching.is_empty(),
        "expected a {code} diagnostic to drive the quick fix, got {diags:?}"
    );
    let rng = matching[0]["range"].clone();
    code_actions_only(lsp, uri, rng, json!(matching), &["quickfix"])
}

/// E006: an overlong grouped formal is the one malformed parameter shape
/// whose structural repair is unambiguous.  The analyser carries a
/// review-required edit which the LSP publishes as an ordinary quick fix.
#[test]
fn test_e006_offers_split_grouped_formals_fix() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let src = "proc invalid {{a b c}} {}\n";
    let diags = lsp.open_ready(&uri, src);
    let actions = quickfixes_for_code(&mut lsp, &uri, &diags, "E006");
    assert_fix_applies(
        &actions,
        src,
        "Split the grouped fields into separate parameters",
        "proc invalid {a b c} {}\n",
    );
}

/// E006: qualified and array-element names have several plausible intended
/// spellings, so the LSP must not manufacture a rewrite for either form.
#[test]
fn test_e006_abstains_from_ambiguous_name_repairs() {
    for (suffix, src) in [
        ("qualified", "proc invalid {a::b} {}\n"),
        ("array", "proc invalid {a(x)} {}\n"),
    ] {
        let mut lsp = Lsp::tcl();
        let uri = unique_uri(suffix);
        let diags = lsp.open_ready(&uri, src);
        let actions = quickfixes_for_code(&mut lsp, &uri, &diags, "E006");
        assert!(
            !titles(&actions)
                .iter()
                .any(|title| title == "Split the grouped fields into separate parameters"),
            "ambiguous name repair must be withheld: {actions:?}"
        );
    }
}

/// W146: a complete literal trace operation list keeps every valid member and
/// removes only the member the selected trace type rejects. The analyser marks
/// this action `RequiresReview`, so it is offered individually and never through
/// Fix All Safe Issues.
#[test]
fn test_w146_removes_only_invalid_trace_operation_members() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let source = "trace add variable item {read rename write} callback\n";
    let diagnostics = lsp.open_ready(&uri, source);
    let actions = quickfixes_for_code(&mut lsp, &uri, &diagnostics, "W146");
    assert_fix_applies(
        &actions,
        source,
        "Remove invalid member(s) 'rename'",
        "trace add variable item {read write} callback\n",
    );
    let fixed = lsp.execute_command("tcl-lsp.fixAllSafeIssues", serde_json::json!([uri]));
    assert_eq!(
        fixed.get("source").and_then(Value::as_str),
        Some(source),
        "the RequiresReview W146 action must not enter Fix All Safe Issues"
    );
}

/// Removing every member would leave Tcl's invalid empty operation list, so
/// W146 diagnoses the all-invalid literal but offers no rewrite.
#[test]
fn test_w146_all_invalid_trace_operations_offer_no_fix() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let source = "trace add command worker {read write} callback\n";
    let diagnostics = lsp.open_ready(&uri, source);
    let actions = quickfixes_for_code(&mut lsp, &uri, &diagnostics, "W146");
    assert!(
        new_texts(&actions).is_empty(),
        "unexpected W146 fix: {actions:?}"
    );
}

/// W120: the missing-`package require` diagnostic carries an insertion fix
/// that adds the require line at the top of the file (no requires exist yet).
#[test]
fn test_w120_offers_insert_package_require_fix() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let src = "canvas .c -width 10\n";
    let diags = lsp.open_ready(&uri, src);
    let actions = quickfixes_for_code(&mut lsp, &uri, &diags, "W120");
    assert_fix_applies(
        &actions,
        src,
        "Add 'package require Tk'",
        "package require Tk\ncanvas .c -width 10\n",
    );
}

/// W144 lifecycle replacements are registry hooks, so the LSP receives the
/// analyser edit without knowing which deprecated command/subcommand emitted
/// it. `interp slaves` remains executable in Tcl 9, but `children` is the
/// documented, semantics-equivalent spelling.
#[test]
fn test_w144_offers_registry_lifecycle_subcommand_fix() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let src = "# tcl-dialect: tcl9.0\ninterp slaves\n";
    let diags = lsp.open_ready(&uri, src);
    let actions = quickfixes_for_code(&mut lsp, &uri, &diags, "W144");
    assert_fix_applies(
        &actions,
        src,
        "Replace deprecated 'slaves' with 'children'",
        "# tcl-dialect: tcl9.0\ninterp children\n",
    );
}

/// W120 FP guard: with the require already present the diagnostic is silent
/// and no insert fix is offered.
#[test]
fn test_w120_no_insert_fix_when_require_present() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let diags = lsp.open_ready(&uri, "package require Tk\ncanvas .c -width 10\n");
    assert!(
        with_code(&diags, "W120").is_empty(),
        "no W120 with the require present: {diags:?}"
    );
    let actions = code_actions_only(
        &mut lsp,
        &uri,
        range((1, 0), (1, 19)),
        json!([]),
        &["quickfix"],
    );
    assert!(
        !titles(&actions)
            .iter()
            .any(|t| t == "Add 'package require Tk'"),
        "{actions:?}"
    );
}

// The evidence-gated fuzzy package suggestion.
//
// This action changes what the interpreter loads and runs the package's
// initialisation code, so the server must offer it only over a *command
// head* that resolution could not satisfy — never merely any
// identifier-shaped word under the cursor, including one inside a comment or
// a string.

/// The quickfix titles the server offers at `position` in `source`, with the
/// published diagnostics fed back as context — the exact flow an editor's
/// lightbulb drives.
fn quickfix_titles_at(source: &str, position: (u32, u32)) -> Vec<String> {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let diags = lsp.open_ready(&uri, source);
    let actions = code_actions_only(
        &mut lsp,
        &uri,
        range(position, position),
        json!(diags),
        &["quickfix"],
    );
    titles(&actions)
}

#[test]
fn test_package_suggestion_offered_on_an_unresolved_command_head() {
    // `http` names a registered package and `http::fetchall` resolves to
    // nothing, so the suggestion is evidence-backed.
    let offered = quickfix_titles_at("http::fetchall $url\n", (0, 3));
    assert!(
        offered.iter().any(|t| t == "Add 'package require http'"),
        "{offered:?}"
    );
}

#[test]
fn test_package_suggestion_not_offered_inside_a_comment() {
    let offered = quickfix_titles_at("# see http::fetchall for details\n", (0, 10));
    assert!(
        !offered
            .iter()
            .any(|t| t.starts_with("Add 'package require")),
        "a comment is not a call site: {offered:?}"
    );
}

#[test]
fn test_package_suggestion_not_offered_inside_a_string() {
    let offered = quickfix_titles_at("set example \"http::fetchall\"\n", (0, 16));
    assert!(
        !offered
            .iter()
            .any(|t| t.starts_with("Add 'package require")),
        "quoted data is not a call site: {offered:?}"
    );
}

#[test]
fn test_package_suggestion_not_offered_on_an_argument_word() {
    let offered = quickfix_titles_at("dict set docs command http::fetchall\n", (0, 25));
    assert!(
        !offered
            .iter()
            .any(|t| t.starts_with("Add 'package require")),
        "an argument word is not a call site: {offered:?}"
    );
}

#[test]
fn test_package_suggestion_not_offered_on_a_local_definition() {
    // The file defines the very command a package would provide, both at the
    // `proc` name word and at the later call.
    let source = "proc http::fetchall {url} {}\nhttp::fetchall http://x\n";
    for position in [(0u32, 8u32), (1, 3)] {
        let offered = quickfix_titles_at(source, position);
        assert!(
            !offered
                .iter()
                .any(|t| t.starts_with("Add 'package require")),
            "locally defined at {position:?}: {offered:?}"
        );
    }
}

/// W123: the unknown-command did-you-mean suggestion carries a replacement
/// fix covering exactly the misspelt head.
#[test]
fn test_w123_offers_did_you_mean_replacement() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let src = "putss hello\n";
    let diags = lsp.open_ready(&uri, src);
    let actions = quickfixes_for_code(&mut lsp, &uri, &diags, "W123");
    assert_fix_applies(&actions, src, "Replace with 'puts'", "puts hello\n");
}

/// W123 guard: when no candidate is within the edit-distance budget the
/// diagnostic still fires but carries no replacement action.
#[test]
fn test_w123_no_replacement_without_near_candidate() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let diags = lsp.open_ready(&uri, "zq9 1 2\n");
    let actions = quickfixes_for_code(&mut lsp, &uri, &diags, "W123");
    assert!(
        !titles(&actions)
            .iter()
            .any(|t| t.starts_with("Replace with")),
        "no candidate is within one edit of 'zq9': {actions:?}"
    );
}

/// W001: the unknown-subcommand did-you-mean suggestion carries a
/// replacement fix covering exactly the misspelt subcommand word.
#[test]
fn test_w001_offers_subcommand_did_you_mean_replacement() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let src = "string lenght abc\n";
    let diags = lsp.open_ready(&uri, src);
    let actions = quickfixes_for_code(&mut lsp, &uri, &diags, "W001");
    assert_fix_applies(
        &actions,
        src,
        "Replace with 'length'",
        "string length abc\n",
    );
}

/// W123 FP guard: `exists`/`get` called bare from inside
/// a proc lexically defined under `::tcl::dict` are the real, separately
/// -callable `::tcl::dict::exists` / `::tcl::dict::get` builtins, not unknown
/// commands — no diagnostic, and critically no "Replace with 'exit'"
/// quickfix either. Offering that quickfix here would, if applied,
/// silently turn `if {[exists $d $k]} {...}` into
/// `if {[exit $d $k]} {...}` — terminating the process instead of testing
/// dict membership.
#[test]
fn test_w123_no_harmful_exit_replacement_for_dict_ensemble_bare_exists() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let src = "proc ::tcl::dict::myhelper {d k} {\n    if {[exists $d $k]} { return [get $d $k] }\n    return MISSING\n}\n";
    let diags = lsp.open_ready(&uri, src);
    assert!(
        with_code(&diags, "W123").is_empty(),
        "bare exists/get inside ::tcl::dict must not fire W123: {diags:?}"
    );
    let actions = code_actions_only(
        &mut lsp,
        &uri,
        range((1, 9), (1, 15)),
        json!([]),
        &["quickfix"],
    );
    assert!(
        !titles(&actions)
            .iter()
            .any(|t| t.starts_with("Replace with")),
        "must never offer to replace 'exists' with 'exit': {actions:?}"
    );
}

/// W001 FP guard: a unique-prefix abbreviation is valid ensemble dispatch —
/// no diagnostic, and no replacement action either.
#[test]
fn test_w001_no_action_for_unique_prefix_abbreviation() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let diags = lsp.open_ready(&uri, "string le abc\n");
    assert!(
        with_code(&diags, "W001").is_empty(),
        "`string le` abbreviates `length`; must not fire W001: {diags:?}"
    );
    let actions = code_actions_only(
        &mut lsp,
        &uri,
        range((0, 7), (0, 9)),
        json!([]),
        &["quickfix"],
    );
    assert!(
        !titles(&actions)
            .iter()
            .any(|t| t.starts_with("Replace with")),
        "{actions:?}"
    );
}

/// W213: `unset` of a possibly-undefined variable carries a fix that splices
/// ` -nocomplain` in right after the `unset` word.
#[test]
fn test_w213_offers_nocomplain_insertion() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let src = "proc foo {} { unset xs }\n";
    let diags = lsp.open_ready(&uri, src);
    let actions = quickfixes_for_code(&mut lsp, &uri, &diags, "W213");
    assert_fix_applies(
        &actions,
        src,
        "Add '-nocomplain' to unset",
        "proc foo {} { unset -nocomplain xs }\n",
    );
}

/// W213 FP guard: `-nocomplain` already present — no diagnostic and no
/// insertion action.
#[test]
fn test_w213_no_action_when_nocomplain_present() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let diags = lsp.open_ready(&uri, "proc foo {} { unset -nocomplain xs }\n");
    assert!(
        with_code(&diags, "W213").is_empty(),
        "-nocomplain suppresses W213: {diags:?}"
    );
    let actions = code_actions_only(
        &mut lsp,
        &uri,
        range((0, 14), (0, 34)),
        json!([]),
        &["quickfix"],
    );
    assert!(
        !titles(&actions).iter().any(|t| t.contains("-nocomplain")),
        "{actions:?}"
    );
}

/// W216 pattern 1: value-position `${arr}(x)` parses as scalar `${arr}` plus
/// literal `(x)`; the fix rewrites the whole word to `$arr(x)`.
#[test]
fn test_w216_offers_brace_form_array_replacement() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let src = "set arr(x) 1\nputs ${arr}(x)\n";
    let diags = lsp.open_ready(&uri, src);
    let actions = quickfixes_for_code(&mut lsp, &uri, &diags, "W216");
    assert_fix_applies(
        &actions,
        src,
        "Replace with `$arr(x)`",
        "set arr(x) 1\nputs $arr(x)\n",
    );
}

/// W216 pattern 2: `${arr($i)}` applies no substitution to `$i` (the brace
/// form is literal); the fix rewrites to `$arr($i)` so the index substitutes.
#[test]
fn test_w216_offers_fix_for_brace_wrapped_index_substitution() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let src = "set arr(k) 1\nset i k\nputs ${arr($i)}\n";
    let diags = lsp.open_ready(&uri, src);
    let actions = quickfixes_for_code(&mut lsp, &uri, &diags, "W216");
    assert_fix_applies(
        &actions,
        src,
        "Replace with `$arr($i)`",
        "set arr(k) 1\nset i k\nputs $arr($i)\n",
    );
}

/// W216 FP guard: in a variable-name position `${name}(idx)` is the
/// legitimate indirect-array idiom — no diagnostic and no replacement.
#[test]
fn test_w216_no_action_for_indirect_array_name_position() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let diags = lsp.open_ready(&uri, "set token ::http::1\nset ${token}(status) eof\n");
    assert!(
        with_code(&diags, "W216").is_empty(),
        "name-position use is the indirect-array idiom: {diags:?}"
    );
    let actions = code_actions_only(
        &mut lsp,
        &uri,
        range((1, 0), (1, 24)),
        json!([]),
        &["quickfix"],
    );
    assert!(
        !titles(&actions)
            .iter()
            .any(|t| t.starts_with("Replace with `$token")),
        "{actions:?}"
    );
}

/// W217: an `unset` whose options consume every argument unsets nothing; the
/// fix prepends `--` so the following words become variable names.
#[test]
fn test_w217_offers_terminator_insertion() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let src = "unset -nocomplain\n";
    let diags = lsp.open_ready(&uri, src);
    let actions = quickfixes_for_code(&mut lsp, &uri, &diags, "W217");
    assert_fix_applies(
        &actions,
        src,
        "Insert '--' so the following words are variable names",
        "unset -- -nocomplain\n",
    );
}

/// W217 FP guard: a real variable name follows the option — the call unsets
/// something, so no diagnostic and no terminator action.
#[test]
fn test_w217_no_action_when_a_variable_follows() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let diags = lsp.open_ready(&uri, "set x 1\nunset -nocomplain x\n");
    assert!(
        with_code(&diags, "W217").is_empty(),
        "a real variable name is unset: {diags:?}"
    );
    let actions = code_actions_only(
        &mut lsp,
        &uri,
        range((1, 0), (1, 19)),
        json!([]),
        &["quickfix"],
    );
    assert!(
        !titles(&actions)
            .iter()
            .any(|t| t.starts_with("Insert '--'")),
        "{actions:?}"
    );
}

/// W104: the two-word leading-space `append` carries a whole-command
/// `lappend` rewrite.
#[test]
fn test_w104_offers_lappend_rewrite() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let src = "set item x\nset items {}\nappend items \" $item\"\n";
    let diags = lsp.open_ready(&uri, src);
    let actions = quickfixes_for_code(&mut lsp, &uri, &diags, "W104");
    assert_fix_applies(
        &actions,
        src,
        "Rewrite with `lappend`",
        "set item x\nset items {}\nlappend items $item\n",
    );
}

/// W104 guard: the trailing-space shape still fires the hint but offers no
/// rewrite — `lappend` would move the separator to the other side.
#[test]
fn test_w104_no_rewrite_for_trailing_space() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let diags = lsp.open_ready(&uri, "append msg \"item \"\n");
    let actions = quickfixes_for_code(&mut lsp, &uri, &diags, "W104");
    assert!(
        !titles(&actions)
            .iter()
            .any(|t| t == "Rewrite with `lappend`"),
        "trailing-space append is not a mechanical lappend: {actions:?}"
    );
}

/// W114: the redundant nested `[expr {…}]` inside a braced outer expression
/// carries an unwrap fix, parenthesised for a compound inner body.
#[test]
fn test_w114_offers_parenthesised_unwrap() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let src = "set a 1\nset b 3\nif {$a && [expr {$b + 1}]} { puts hi }\n";
    let diags = lsp.open_ready(&uri, src);
    let actions = quickfixes_for_code(&mut lsp, &uri, &diags, "W114");
    assert_fix_applies(
        &actions,
        src,
        "Unwrap the nested `expr`",
        "set a 1\nset b 3\nif {$a && ($b + 1)} { puts hi }\n",
    );
}

/// W114: a lone-`$var` inner body inlines bare — the parentheses would be
/// noise around a single atom.
#[test]
fn test_w114_atom_unwrap_drops_parentheses() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let src = "set x 2\nset y [expr {[expr {$x}] + 1}]\n";
    let diags = lsp.open_ready(&uri, src);
    let actions = quickfixes_for_code(&mut lsp, &uri, &diags, "W114");
    assert_fix_applies(
        &actions,
        src,
        "Unwrap the nested `expr`",
        "set x 2\nset y [expr {$x + 1}]\n",
    );
}

/// W114 guard: an unbraced inner body still fires the warning but offers no
/// unwrap — inlining it would re-expose the text to substitution.
#[test]
fn test_w114_no_unwrap_for_unbraced_inner_body() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let diags = lsp.open_ready(&uri, "set x 1\nif {[expr $x + 1] > 0} {}\n");
    let actions = quickfixes_for_code(&mut lsp, &uri, &diags, "W114");
    assert!(
        !titles(&actions)
            .iter()
            .any(|t| t == "Unwrap the nested `expr`"),
        "unbraced inner body must stay message-only: {actions:?}"
    );
}

/// W201: the manual path concatenation carries a `[file join …]` rewrite of
/// exactly the value word (the leading `/` stays, keeping the path
/// absolute). The rewrite spells the variable in the lowered `${x}` form.
#[test]
fn test_w201_offers_file_join_rewrite() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let src = "set x 42\nset p \"/tmp/$x\"\n";
    let diags = lsp.open_ready(&uri, src);
    let actions = quickfixes_for_code(&mut lsp, &uri, &diags, "W201");
    assert_fix_applies(
        &actions,
        src,
        "Rewrite with `file join`",
        "set x 42\nset p [file join /tmp ${x}]\n",
    );
}

/// W201 guard: a mixed segment (`$n.log` — neither a pure variable nor a
/// pure literal) still fires the hint but offers no rewrite.
#[test]
fn test_w201_no_rewrite_for_mixed_segment() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let diags = lsp.open_ready(&uri, "set n x\nset p \"/var/log/$n.log\"\n");
    let actions = quickfixes_for_code(&mut lsp, &uri, &diags, "W201");
    assert!(
        !titles(&actions)
            .iter()
            .any(|t| t == "Rewrite with `file join`"),
        "a mixed segment does not decompose cleanly: {actions:?}"
    );
}
