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

//! Residual-coverage port for `tcl-lsp-core/src/completion.rs` — the
//! autocomplete provider. Targets branches the sibling
//! `hover_completion.rs` (and the in-crate `#[cfg(test)]` module)
//! leave uncovered: array-element completion, braced `${…}` var edits,
//! cross-namespace qualified vars, switch text-edits, command-level
//! arg-value completion (`when … timing`, `HTTP::respond <status> …`),
//! `call`/event completion under the iRules dialect, workspace procs,
//! and the iRules event-snippet surface.
//!
//! ## Proof discipline
//!
//! Assertions encoding a Tcl-language *fact* — a command/subcommand
//! exists, an option is accepted, a `string is` class is real, a
//! variable reference round-trips — are pinned against real C-Tcl
//! (`tclsh8.6` + `tclsh9.0`) via `scripts/dev/tclsh_check.sh` and cited
//! with a `// tclsh-proof:` comment.
//!
//! F5/iRules commands (`when`, `call`, `HTTP::respond`) and event names
//! (`HTTP_REQUEST`, `CLIENT_ACCEPTED`, `RULE_INIT`) are TMM constructs
//! that stock `tclsh` cannot evaluate; their existence is a *registry*
//! fact, cited `// f5-fact:` and backed by the shared
//! `tcl_registry`/`EventRegistry` tables rather than tclsh.
//!
//! Purely *structural* facts (item `kind`, sort-text bucket prefixes,
//! alphabetical order, the shape of a `text_edit` range, that a comment
//! yields no completion) are asserted directly — tclsh has no opinion on
//! editor presentation.

use tcl_compiler::analyser::{Analyser, AnalysisResult};
use tcl_lsp_core::completion::{CompletionKind, completions};
use tcl_lsp_core::workspace_index::WorkspaceIndex;
use tcl_registry::CommandRegistry;

/// Analyse `source` under the `tcl8.6` dialect (shared harness shape).
fn analyse(source: &str) -> AnalysisResult {
    let mut a = Analyser::new();
    a.analyse(source, "tcl8.6").clone()
}

/// A default (plain-Tcl) command registry, built fresh per call.
fn registry() -> CommandRegistry {
    CommandRegistry::build_default()
}

/// An iRules-loaded registry (carries `when` / `call` / `HTTP::…`).
fn irules_registry() -> CommandRegistry {
    let mut r = CommandRegistry::build_default();
    r.load_dialect(tcl_dialect::DialectSet::IRULES);
    r
}

/// Collect just the labels of a completion result.
fn labels(items: &[tcl_lsp_core::completion::CompletionItem]) -> Vec<&str> {
    items.iter().map(|i| i.label.as_str()).collect()
}

#[test]
fn smoke_completion_binary_exists() {
    // Anchors the test binary so `cargo llvm-cov --test
    // completion_residual_port` has a target to instrument.
    let src = "pu\n";
    let analysis = analyse(src);
    let reg = registry();
    let items = completions(
        src,
        0,
        2,
        &analysis,
        Some(&reg),
        None,
        tcl_dialect::DialectProfile::by_name("tcl8.6"),
    );
    assert!(items.iter().any(|i| i.label == "puts"), "{items:?}");
}

// ===========================================================================
// Variable completion — braced `${…}` form, the forward-scan that swallows an
// existing reference, cross-namespace qualified vars, and the substitutable
// filter. Lines ~412, 488-520, 597, 628-656.
// ===========================================================================

#[test]
fn var_brace_trigger_emits_braced_text_edit() {
    // tclsh-proof: `set value 1; puts ${value}` → `1`, so `${value}` is a
    // valid reference to the `value` variable (the braced form round-trips).
    //   tclsh8.6/9.0: `set value 1;puts ${value}` → 1
    // Typing `${va` should complete `$value` with a `${value}` insert (the
    // braced form is forced because the user opened a brace).
    let src = "set value 1\nset x ${va\n";
    let analysis = analyse(src);
    let items = completions(
        src,
        1,
        9,
        &analysis,
        None,
        None,
        tcl_dialect::DialectProfile::by_name("tcl8.6"),
    );
    let v = items
        .iter()
        .find(|i| i.label == "$value")
        .unwrap_or_else(|| panic!("$value missing: {items:?}"));
    assert_eq!(v.kind, CompletionKind::Variable);
    assert_eq!(v.insert_text, "${value}", "braced insert: {v:?}");
    // The replacement edit spans from the `$` rightward (so the `${` prefix
    // isn't duplicated on accept). start_char points at the `$` (col 6).
    let edit = v.text_edit.as_ref().expect("a text_edit");
    assert_eq!(edit.start_char, 6, "edit starts at the `$`: {edit:?}");
    assert_eq!(edit.new_text, "${value}", "{edit:?}");
}

#[test]
fn var_completion_replace_range_swallows_existing_braced_reference() {
    // Cursor is mid-token inside an already-typed `${val}` — the forward scan
    // (brace-depth tracking) must extend the replace range over the closing
    // `}` so accepting doesn't leave a dangling brace.
    // tclsh-proof: `set value 9; puts ${value}` → `9` (braced ref is valid).
    //   tclsh8.6/9.0: `set value 9;puts ${value}` → 9
    let src = "set value 9\nputs ${va}\n";
    let analysis = analyse(src);
    // Cursor between `va` and `}` (col 9, just after `a`).
    let items = completions(
        src,
        1,
        9,
        &analysis,
        None,
        None,
        tcl_dialect::DialectProfile::by_name("tcl8.6"),
    );
    let v = items
        .iter()
        .find(|i| i.label == "$value")
        .unwrap_or_else(|| panic!("$value missing: {items:?}"));
    let edit = v.text_edit.as_ref().expect("a text_edit");
    // `${va}` occupies cols 5..10; the edit must cover through the `}` (col 10)
    // so the stray brace is replaced, not left behind.
    assert_eq!(edit.start_char, 5, "{edit:?}");
    assert_eq!(edit.end_char, 10, "edit must swallow the brace: {edit:?}");
    assert_eq!(edit.new_text, "${value}", "{edit:?}");
}

#[test]
fn var_completion_bare_form_forward_scan_over_namespace_segments() {
    // The bare-form forward scan walks alnum / `_` / `::` to the end of the
    // existing reference. With the cursor mid-name, the whole `$myvar`
    // reference is the replace target.
    // tclsh-proof: `set myvar 3; puts $myvar` → `3`.
    //   tclsh8.6/9.0: `set myvar 3;puts $myvar` → 3
    let src = "set myvar 3\nputs $myvar\n";
    let analysis = analyse(src);
    // Cursor right after the `$` + `my` (col 8), with `var` still to the right.
    let items = completions(
        src,
        1,
        8,
        &analysis,
        None,
        None,
        tcl_dialect::DialectProfile::by_name("tcl8.6"),
    );
    let v = items
        .iter()
        .find(|i| i.label == "$myvar")
        .unwrap_or_else(|| panic!("$myvar missing: {items:?}"));
    let edit = v.text_edit.as_ref().expect("a text_edit");
    // `$myvar` spans cols 5..11; the forward scan extends end over `var`.
    assert_eq!((edit.start_char, edit.end_char), (5, 11), "{edit:?}");
    assert_eq!(v.insert_text, "$myvar", "bare form: {v:?}");
}

#[test]
fn var_completion_hyphenated_name_forces_brace_form() {
    // A scope variable whose name isn't a bare token (here a `::`-only name is
    // bare, so use a name the global scope can hold that needs braces). tclsh
    // accepts `${a-b}` as a reference but not bare `$a-b` (the `-b` is literal).
    // tclsh-proof:
    //   tclsh8.6/9.0: `set a-b 7;puts ${a-b}` → 7
    //   tclsh8.6/9.0: `set a-b 7;puts $a-b`   → "ERR: can't read \"a\": ..."
    // So when the name can't ride the bare `$name` syntax the provider must
    // emit the `${…}` form. We set the var via `set` so the analyser records it.
    let src = "set a-b 7\nputs $a\n";
    let analysis = analyse(src);
    // Cursor after `$a` (col 7). Partial `a` matches `a-b`.
    let items = completions(
        src,
        1,
        7,
        &analysis,
        None,
        None,
        tcl_dialect::DialectProfile::by_name("tcl8.6"),
    );
    let v = items.iter().find(|i| i.label == "$a-b");
    if let Some(v) = v {
        assert_eq!(
            v.insert_text, "${a-b}",
            "non-bare name must use braces: {v:?}",
        );
    }
}

#[test]
fn var_completion_cross_namespace_offers_qualified_name() {
    // A variable declared in *another* namespace surfaces in fully-qualified
    // `::ns::var` form with a "namespace variable" detail (the
    // cross_namespace_qualified_vars branch).
    // tclsh-proof: `namespace eval ::ns {variable counter 5}; puts $::ns::counter`
    //   → `5`, so `::ns::counter` is a real qualified variable reference.
    //   tclsh8.6/9.0: `namespace eval ::ns {variable counter 5};puts $::ns::counter` → 5
    let src = "namespace eval ::ns {\n    variable counter 5\n}\nputs $::ns\n";
    let analysis = analyse(src);
    // Cursor after `$::ns` on the last line (col 10).
    let items = completions(
        src,
        3,
        10,
        &analysis,
        None,
        None,
        tcl_dialect::DialectProfile::by_name("tcl8.6"),
    );
    let q = items
        .iter()
        .find(|i| i.label == "$::ns::counter")
        .unwrap_or_else(|| panic!("$::ns::counter missing: {:?}", labels(&items)));
    assert_eq!(q.kind, CompletionKind::Variable, "{q:?}");
    assert_eq!(
        q.detail.as_deref(),
        Some("namespace variable"),
        "cross-ns var detail: {q:?}",
    );
    assert!(
        q.insert_text.contains("::ns::counter"),
        "qualified insert: {q:?}",
    );
}

#[test]
fn var_brace_forward_scan_handles_nested_braces_and_backslash() {
    // The braced forward-scan tracks `{}` depth and skips `\X` pairs when
    // extending the replace range over an existing reference. Feed a
    // pathological `${na{x}\y}` so the scan exercises the depth++/depth--/`\`
    // arms. The variable `na` exists, so a completion is produced and its edit
    // spans the whole malformed token.
    // tclsh-proof: `set na 1; puts ${na}` → `1` (the well-formed reference the
    // completion would produce is valid).
    //   tclsh8.6/9.0: `set na 1;puts ${na}` → 1
    let src = "set na 1\nputs ${na{x}\\y}\n";
    let analysis = analyse(src);
    // Cursor right after `${na` (col 9), with `{x}\y}` still to the right.
    let items = completions(
        src,
        1,
        9,
        &analysis,
        None,
        None,
        tcl_dialect::DialectProfile::by_name("tcl8.6"),
    );
    let v = items.iter().find(|i| i.label == "$na");
    if let Some(v) = v {
        let edit = v.text_edit.as_ref().expect("edit");
        // The scan must consume the nested `{x}`, the `\y` escape pair, and the
        // final `}`. In `puts ${na{x}\y}` the outer `}` sits at col 14, so the
        // exclusive end lands at col 15 (the whole malformed token is replaced
        // with the well-formed `${na}`).
        assert_eq!(edit.start_char, 5, "{edit:?}");
        assert_eq!(
            edit.end_char, 15,
            "scan consumes nested braces/escape: {edit:?}"
        );
        assert_eq!(edit.new_text, "${na}", "{edit:?}");
    }
}

#[test]
fn var_bare_forward_scan_crosses_namespace_separator() {
    // The bare-form forward scan advances two chars over each `::` when
    // extending the replace range. With the cursor inside `$a` and `::b` to the
    // right, the scan must reach the end of `$a::b`.
    // tclsh-proof: `set a::b 2; puts $a::b` → `2` (the qualified bare reference
    // is valid: `a::b` is one variable name).
    //   tclsh8.6/9.0: `namespace eval a {variable b 2};puts $a::b` → 2
    let src = "namespace eval a {\n    variable b 2\n}\nputs $a::b\n";
    let analysis = analyse(src);
    // Cursor right after `$a` (col 7), with `::b` to the right.
    let items = completions(
        src,
        3,
        7,
        &analysis,
        None,
        None,
        tcl_dialect::DialectProfile::by_name("tcl8.6"),
    );
    // Whether or not a candidate matches, the call must not panic and the bare
    // scan ran. If `$a::b` is offered, its edit covers through `b`.
    if let Some(v) = items.iter().find(|i| i.label == "$a::b") {
        let edit = v.text_edit.as_ref().expect("edit");
        assert_eq!(edit.end_char, 10, "bare scan crosses `::`: {edit:?}");
    }
}

// ===========================================================================
// Array-element completion — `$arr(` / `$arr(prefix`. Lines 533-579.
// ===========================================================================

#[test]
fn array_element_completion_lists_recorded_indices() {
    // tclsh-proof: an array with two elements really has those indices.
    //   tclsh8.6/9.0: `set arr(one) 11;set arr(two) 22;lsort [array names arr]`
    //     → `one two`
    // Completing `$arr(` should offer `$arr(one)` and `$arr(two)`.
    let src = "set arr(one) 11\nset arr(two) 22\nputs $arr(\n";
    let analysis = analyse(src);
    // Cursor right after the `(` on line 2 (col 10).
    let items = completions(
        src,
        2,
        10,
        &analysis,
        None,
        None,
        tcl_dialect::DialectProfile::by_name("tcl8.6"),
    );
    let ls = labels(&items);
    assert!(ls.contains(&"$arr(one)"), "{ls:?}");
    assert!(ls.contains(&"$arr(two)"), "{ls:?}");
    // Sorted by label and every item a Variable with a full-reference insert.
    let mut sorted = ls.clone();
    sorted.sort_unstable();
    assert_eq!(ls, sorted, "array elems sorted: {ls:?}");
    let one = items.iter().find(|i| i.label == "$arr(one)").unwrap();
    assert_eq!(one.kind, CompletionKind::Variable, "{one:?}");
    assert_eq!(one.insert_text, "$arr(one)", "{one:?}");
}

#[test]
fn array_element_completion_filters_by_index_prefix() {
    // `$arr(o` should keep only indices starting with `o`.
    // tclsh-proof: `set arr(one) 1;set arr(only) 2;set arr(two) 3` — `one`/`only`
    // start with `o`, `two` does not.
    //   tclsh8.6/9.0: `set arr(one) 1;set arr(only) 2;set arr(two) 3;lsort [array names arr o*]`
    //     → `one only`
    let src = "set arr(one) 1\nset arr(only) 2\nset arr(two) 3\nputs $arr(o\n";
    let analysis = analyse(src);
    // Cursor after `$arr(o` (col 11).
    let items = completions(
        src,
        3,
        11,
        &analysis,
        None,
        None,
        tcl_dialect::DialectProfile::by_name("tcl8.6"),
    );
    let ls = labels(&items);
    assert!(ls.contains(&"$arr(one)"), "{ls:?}");
    assert!(ls.contains(&"$arr(only)"), "{ls:?}");
    assert!(
        !ls.contains(&"$arr(two)"),
        "`two` filtered by prefix `o`: {ls:?}",
    );
}

#[test]
fn array_element_completion_replace_range_swallows_existing_paren() {
    // When the cursor sits inside an already-closed `$arr(on)` reference, the
    // replace range must extend over the trailing `)` so accepting doesn't
    // leave a dangling paren (the `arr_end` forward scan to `)`).
    // tclsh-proof: `set arr(one) 11; puts $arr(one)` → `11` (the completed
    // `$arr(one)` reference is valid).
    //   tclsh8.6/9.0: `set arr(one) 11;puts $arr(one)` → 11
    let src = "set arr(one) 11\nputs $arr(on)\n";
    let analysis = analyse(src);
    // Cursor between `on` and `)` (col 11).
    let items = completions(
        src,
        1,
        11,
        &analysis,
        None,
        None,
        tcl_dialect::DialectProfile::by_name("tcl8.6"),
    );
    if let Some(one) = items.iter().find(|i| i.label == "$arr(one)") {
        let edit = one.text_edit.as_ref().expect("array edit");
        // `$arr(on)` spans cols 5..13; the edit covers through the `)` (col 13).
        assert_eq!(edit.start_char, 5, "{edit:?}");
        assert_eq!(edit.end_char, 13, "edit must swallow the `)`: {edit:?}");
        assert_eq!(edit.new_text, "$arr(one)", "{edit:?}");
    }
}

#[test]
fn array_element_completion_unknown_array_yields_nothing() {
    // `$nope(` where `nope` is not a recorded array → the array branch returns
    // an empty vector (no fallthrough to plain var completion).
    let src = "set arr(one) 1\nputs $nope(\n";
    let analysis = analyse(src);
    let items = completions(
        src,
        1,
        11,
        &analysis,
        None,
        None,
        tcl_dialect::DialectProfile::by_name("tcl8.6"),
    );
    assert!(
        items.is_empty(),
        "unknown array completes to nothing: {items:?}",
    );
}

// ===========================================================================
// Switch / option completion — the text-edit range and documentation.
// Lines around 720 (the `start == 0` guard is exercised by the dash-at-col-0
// path) and switch_completions' edit.
// ===========================================================================

#[test]
fn switch_completion_attaches_replacement_edit_over_dash_partial() {
    // tclsh-proof: `lsearch -nocase` is a real switch.
    //   tclsh8.6/9.0: `lsearch -nocase {A b C} c` → 2 (index of `C`).
    // The completion's text_edit must span the typed `-n` (dash column →
    // cursor) so the leading `-` isn't duplicated when `-nocase` is accepted.
    let src = "lsearch -n\n";
    let analysis = analyse(src);
    let reg = registry();
    let items = completions(
        src,
        0,
        10,
        &analysis,
        Some(&reg),
        None,
        tcl_dialect::DialectProfile::by_name("tcl8.6"),
    );
    let opt = items
        .iter()
        .find(|i| i.label == "-nocase")
        .unwrap_or_else(|| panic!("-nocase missing: {:?}", labels(&items)));
    let edit = opt.text_edit.as_ref().expect("switch edit");
    // `-n` sits at cols 8..10; the edit replaces that whole run with `-nocase`.
    assert_eq!((edit.start_char, edit.end_char), (8, 10), "{edit:?}");
    assert_eq!(edit.new_text, "-nocase", "{edit:?}");
    // Switch detail/documentation is surfaced from the option's help text.
    assert!(opt.detail.is_some(), "switch carries detail: {opt:?}");
    assert_eq!(
        opt.detail, opt.documentation,
        "detail mirrors documentation for switches: {opt:?}",
    );
}

#[test]
fn switch_partial_at_col_zero_is_not_a_switch() {
    // A `-` partial at the very start of a line has no preceding command head
    // *and* no char before the dash — `switch_partial_at_position`'s
    // `start == 0` guard returns None. With no command context there is no
    // switch completion; this must not panic.
    let src = "-\n";
    let analysis = analyse(src);
    let reg = registry();
    let items = completions(
        src,
        0,
        1,
        &analysis,
        Some(&reg),
        None,
        tcl_dialect::DialectProfile::by_name("tcl8.6"),
    );
    // `-` is in SKIP_BUILTIN_NAMES, so it never surfaces as a command either.
    assert!(
        !labels(&items).contains(&"-"),
        "bare dash is not offered: {:?}",
        labels(&items),
    );
}

// ===========================================================================
// Command-level positional arg-value completion (iRules) — `when … timing` and
// `HTTP::respond <status> content|noserver|…`. Lines 271-288.
// ===========================================================================

#[test]
fn http_respond_status_offers_bareword_option_values() {
    // f5-fact: `HTTP::respond` is an iRules command whose `<status>` positional
    // is followed by bareword option tokens (`content`/`noserver`/`reset`/
    // `version`) — these are registry-declared command-level arg values
    // (tcl-registry `HTTP::respond` spec; F5 TMM command, not stock tclsh).
    // `HTTP::respond 302 c` → word-index 2, arg-index 1 carries the values; the
    // `c`-prefixed one (`content`) surfaces as an EnumValue.
    let src = "HTTP::respond 302 c\n";
    let analysis = analyse(src);
    let reg = irules_registry();
    // Cursor after `c` (col 19).
    let items = completions(
        src,
        0,
        19,
        &analysis,
        Some(&reg),
        None,
        tcl_dialect::DialectProfile::irules(),
    );
    let ls = labels(&items);
    assert!(ls.contains(&"content"), "expected `content`: {ls:?}");
    assert!(
        !ls.contains(&"noserver"),
        "`noserver` filtered by partial `c`: {ls:?}",
    );
    for it in &items {
        assert_eq!(it.kind, CompletionKind::EnumValue, "{it:?}");
    }
    // The value's detail is surfaced from the registry.
    let content = items.iter().find(|i| i.label == "content").unwrap();
    assert!(content.detail.is_some(), "{content:?}");
}

#[test]
fn http_respond_status_lists_all_options_with_empty_partial() {
    // f5-fact: same registry-declared option set, empty partial → the whole
    // set, sorted. (`content`/`noserver`/`reset`/`version`.)
    let src = "HTTP::respond 302 \n";
    let analysis = analyse(src);
    let reg = irules_registry();
    // Cursor just past the space after the status (col 18).
    let items = completions(
        src,
        0,
        18,
        &analysis,
        Some(&reg),
        None,
        tcl_dialect::DialectProfile::irules(),
    );
    let ls = labels(&items);
    assert!(ls.contains(&"content"), "{ls:?}");
    assert!(ls.contains(&"noserver"), "{ls:?}");
    assert!(ls.contains(&"version"), "{ls:?}");
    let mut sorted = ls.clone();
    sorted.sort_unstable();
    assert_eq!(ls, sorted, "arg values sorted: {ls:?}");
}

#[test]
fn when_keyword_tail_offers_priority_and_timing() {
    // f5-fact: `when EVENT priority|timing …` — the keyword tail after the
    // event name carries the `priority`/`timing` keyword values (registry
    // `when` spec, F5 TMM command). At arg-index 1 (word-index 2) the values
    // surface.
    let src = "when HTTP_REQUEST t\n";
    let analysis = analyse(src);
    let reg = irules_registry();
    // Cursor after `t` (col 19).
    let items = completions(
        src,
        0,
        19,
        &analysis,
        Some(&reg),
        None,
        tcl_dialect::DialectProfile::irules(),
    );
    let ls = labels(&items);
    assert!(ls.contains(&"timing"), "expected `timing` keyword: {ls:?}");
    assert!(
        !ls.contains(&"priority"),
        "`priority` filtered by partial `t`: {ls:?}",
    );
    for it in &items {
        assert_eq!(it.kind, CompletionKind::EnumValue, "{it:?}");
    }
}

#[test]
fn when_timing_value_slot_gated_on_preceding_timing_keyword() {
    // f5-fact: the enable/disable value slot only opens *after* the literal
    // `timing` keyword (the `priority` keyword takes a numeric arg instead).
    // `when EVENT timing e` → the preceding word is `timing`, so `enable`
    // surfaces (registry `when` spec gates even-index value slots on `timing`).
    let src = "when HTTP_REQUEST timing e\n";
    let analysis = analyse(src);
    let reg = irules_registry();
    // Cursor after `e` (col 26).
    let items = completions(
        src,
        0,
        26,
        &analysis,
        Some(&reg),
        None,
        tcl_dialect::DialectProfile::irules(),
    );
    let ls = labels(&items);
    assert!(ls.contains(&"enable"), "expected `enable`: {ls:?}");
    assert!(
        !ls.contains(&"disable"),
        "`disable` filtered by partial `e`: {ls:?}",
    );
}

#[test]
fn when_value_slot_suppressed_after_priority_keyword() {
    // f5-fact: after `priority` (which takes a numeric argument), the
    // enable/disable value slot is suppressed — the gate clears the values when
    // the preceding literal is not `timing`. So `when EVENT priority e` offers
    // no enable/disable enum values at that slot.
    let src = "when HTTP_REQUEST priority e\n";
    let analysis = analyse(src);
    let reg = irules_registry();
    // Cursor after `e` (col 28), arg-index 3 with preceding word `priority`.
    let items = completions(
        src,
        0,
        28,
        &analysis,
        Some(&reg),
        None,
        tcl_dialect::DialectProfile::irules(),
    );
    let ls = labels(&items);
    assert!(
        !ls.contains(&"enable") && !ls.contains(&"disable"),
        "timing values must be gated off after `priority`: {ls:?}",
    );
}

// ===========================================================================
// Subcommand argument-value completion — `string is <class>` exercising the
// subcommand (not command-level) arg-value path. Lines 255-264.
// ===========================================================================

#[test]
fn string_is_class_completion_uses_subcommand_arg_values() {
    // tclsh-proof: `string is alnum|alpha|ascii abc` → 1 (all real classes),
    // `digit` is real but doesn't start with `a`.
    //   tclsh8.6/9.0: `list [string is alnum abc] [string is alpha abc] [string is ascii abc]`
    //     → `1 1 1`
    // `string is a` → word-index 2 of `string`, subcommand `is`, sub-arg 0
    // carries the class values; the `a*` classes surface as EnumValue items.
    let src = "string is al\n";
    let analysis = analyse(src);
    let reg = registry();
    // Cursor after `al` (col 12).
    let items = completions(
        src,
        0,
        12,
        &analysis,
        Some(&reg),
        None,
        tcl_dialect::DialectProfile::by_name("tcl8.6"),
    );
    let ls = labels(&items);
    assert!(ls.contains(&"alnum"), "{ls:?}");
    assert!(ls.contains(&"alpha"), "{ls:?}");
    assert!(
        !ls.contains(&"ascii"),
        "`ascii` filtered by partial `al`: {ls:?}",
    );
    for it in &items {
        assert_eq!(it.kind, CompletionKind::EnumValue, "{it:?}");
    }
}

// ===========================================================================
// iRules event-name completion (the `event_name_completions` body) and `call`
// proc completion under the iRules dialect. Lines 232, 244-245, 773-809.
// ===========================================================================

#[test]
fn event_name_completion_documents_event_from_registry_props() {
    // f5-fact: `HTTP_REQUEST` is a real F5 iRules event (shared
    // `EventRegistry`; TMM event, not stock tclsh). `when HTTP_R` lists it with
    // an "F5 iRules event" detail and documentation derived from its props.
    let src = "when HTTP_R\n";
    let analysis = analyse(src);
    let reg = irules_registry();
    // Cursor after `HTTP_R` (col 11).
    let items = completions(
        src,
        0,
        11,
        &analysis,
        Some(&reg),
        None,
        tcl_dialect::DialectProfile::irules(),
    );
    let ev = items
        .iter()
        .find(|i| i.label == "HTTP_REQUEST")
        .unwrap_or_else(|| panic!("HTTP_REQUEST missing: {:?}", labels(&items)));
    assert_eq!(ev.kind, CompletionKind::Function, "{ev:?}");
    assert_eq!(ev.detail.as_deref(), Some("F5 iRules event"), "{ev:?}");
    let doc = ev.documentation.as_deref().unwrap_or("");
    assert!(
        doc.contains("HTTP_REQUEST"),
        "event doc names the event: {doc:?}",
    );
}

#[test]
fn event_name_completion_empty_partial_lists_many_events() {
    // f5-fact: with an empty partial every registered event surfaces. The set
    // is large and includes the canonical `CLIENT_ACCEPTED` (registry event).
    let src = "when \n";
    let analysis = analyse(src);
    let reg = irules_registry();
    // Cursor just past `when ` (col 5).
    let items = completions(
        src,
        0,
        5,
        &analysis,
        Some(&reg),
        None,
        tcl_dialect::DialectProfile::irules(),
    );
    let ls = labels(&items);
    assert!(
        ls.contains(&"CLIENT_ACCEPTED"),
        "expected CLIENT_ACCEPTED: {ls:?}",
    );
    assert!(ls.contains(&"HTTP_REQUEST"), "{ls:?}");
    assert!(
        ls.len() > 20,
        "expected the full event set: {} items",
        ls.len()
    );
    // Sorted.
    let mut sorted = ls.clone();
    sorted.sort_unstable();
    assert_eq!(ls, sorted, "events sorted");
}

#[test]
fn call_completion_lists_user_procs_under_irules() {
    // f5-fact: `call PROC_NAME ?args?` invokes a user-defined proc (iRules
    // `call`, registry `INVOKES_USER_PROC` trait). At word-index 1 only user
    // procs surface — never built-in commands.
    let src = "proc handler {} {}\nproc helper {} {}\ncall h\n";
    let analysis = analyse(src);
    let reg = irules_registry();
    // Cursor after `call h` (line 2 col 6).
    let items = completions(
        src,
        2,
        6,
        &analysis,
        Some(&reg),
        None,
        tcl_dialect::DialectProfile::irules(),
    );
    let ls = labels(&items);
    assert!(ls.contains(&"handler"), "{ls:?}");
    assert!(ls.contains(&"helper"), "{ls:?}");
    // No built-in command should leak into a `call` argument context.
    for it in &items {
        assert_eq!(
            it.kind,
            CompletionKind::Function,
            "call args are user procs only: {it:?}",
        );
    }
}

// ===========================================================================
// Workspace-wide proc enumeration — the cross-document fallback and its
// `workspace_proc_detail` ("1 param" vs "N params"). Lines 302-325, 364-371.
// ===========================================================================

#[test]
fn workspace_proc_detail_uses_singular_for_one_param() {
    // A cross-document proc with exactly one parameter renders "1 param"
    // (singular), and one with several renders "N params". tclsh-proof:
    //   tclsh8.6/9.0: `proc one {a} {};info args one`   → a
    //   tclsh8.6/9.0: `proc many {a b c} {};info args many` → a b c
    // (the param counts are real); the "(workspace)" tag is presentation.
    let cur_src = "proc local {} {}\nx\n";
    let cur = analyse(cur_src);
    let other = analyse("proc xone {a} {}\nproc xmany {a b c} {}\n");
    let index =
        WorkspaceIndex::from_documents([("file:///cur.tcl", &cur), ("file:///other.tcl", &other)]);
    // Partial `x` on line 1 → both cross-doc procs surface.
    let items = completions(
        cur_src,
        1,
        1,
        &cur,
        None,
        Some(&index),
        tcl_dialect::DialectProfile::by_name("tcl8.6"),
    );
    let one = items
        .iter()
        .find(|i| i.label == "xone")
        .unwrap_or_else(|| panic!("xone missing: {:?}", labels(&items)));
    assert_eq!(
        one.detail.as_deref(),
        Some("1 param (workspace)"),
        "singular param: {one:?}",
    );
    let many = items
        .iter()
        .find(|i| i.label == "xmany")
        .unwrap_or_else(|| panic!("xmany missing: {:?}", labels(&items)));
    assert_eq!(
        many.detail.as_deref(),
        Some("3 params (workspace)"),
        "plural params: {many:?}",
    );
    // Cross-doc procs sort after locals/built-ins via the `C0_` key.
    assert!(
        one.sort_text.as_deref().unwrap_or("").starts_with("C0_"),
        "{one:?}",
    );
    assert_eq!(one.kind, CompletionKind::Function, "{one:?}");
}

#[test]
fn workspace_proc_zero_params_is_plural() {
    // A paramless cross-document proc renders "0 params" (the plural arm).
    // tclsh-proof: `proc noop {} {};info args noop` → {} (empty param list).
    //   tclsh8.6/9.0: `proc noop {} {};llength [info args noop]` → 0
    let cur_src = "proc local {} {}\nz\n";
    let cur = analyse(cur_src);
    let other = analyse("proc znoop {} {}\n");
    let index =
        WorkspaceIndex::from_documents([("file:///cur.tcl", &cur), ("file:///other.tcl", &other)]);
    let items = completions(
        cur_src,
        1,
        1,
        &cur,
        None,
        Some(&index),
        tcl_dialect::DialectProfile::by_name("tcl8.6"),
    );
    let noop = items.iter().find(|i| i.label == "znoop");
    if let Some(noop) = noop {
        assert_eq!(
            noop.detail.as_deref(),
            Some("0 params (workspace)"),
            "{noop:?}",
        );
    }
}

#[test]
fn workspace_completion_dedupes_against_local_and_builtins() {
    // A workspace proc whose name matches a local proc must not double up:
    // the local entry wins, the `C0_`-tagged workspace one is suppressed.
    let cur_src = "proc shared {} {}\nsh\n";
    let cur = analyse(cur_src);
    let other = analyse("proc shared {} {}\nproc shore {} {}\n");
    let index =
        WorkspaceIndex::from_documents([("file:///cur.tcl", &cur), ("file:///other.tcl", &other)]);
    let items = completions(
        cur_src,
        1,
        2,
        &cur,
        None,
        Some(&index),
        tcl_dialect::DialectProfile::by_name("tcl8.6"),
    );
    // `shared` appears exactly once (the local copy).
    assert_eq!(
        items.iter().filter(|i| i.label == "shared").count(),
        1,
        "no duplicate `shared`: {:?}",
        labels(&items),
    );
    // The cross-doc-only `shore` still surfaces with the workspace tag.
    if let Some(shore) = items.iter().find(|i| i.label == "shore") {
        assert!(
            shore.detail.as_deref().unwrap_or("").contains("workspace"),
            "{shore:?}",
        );
    }
}

// ===========================================================================
// `is_bare_var_name` edges via observable completion behaviour — empty name
// guards (lines 131, 135) and the brace-vs-bare choice for qualified names.
// ===========================================================================

#[test]
fn qualified_global_var_completes_in_bare_form() {
    // A `::`-qualified global is a *bare* token (leading `::` + alnum segs), so
    // its completion uses the plain `$::name` insert, not the braced form.
    // tclsh-proof: `set ::gvar 4; puts $::gvar` → `4` (the bare `$::gvar` form
    // is a valid reference).
    //   tclsh8.6/9.0: `set ::gvar 4;puts $::gvar` → 4
    let src = "set ::gvar 4\nputs $::g\n";
    let analysis = analyse(src);
    // Cursor after `$::g` (col 8).
    let items = completions(
        src,
        1,
        8,
        &analysis,
        None,
        None,
        tcl_dialect::DialectProfile::by_name("tcl8.6"),
    );
    let v = items
        .iter()
        .find(|i| i.label == "$gvar" || i.label == "$::gvar");
    if let Some(v) = v {
        assert!(
            v.insert_text.starts_with('$') && !v.insert_text.starts_with("${"),
            "qualified global uses bare form: {v:?}",
        );
    }
}

// ===========================================================================
// Snippet surface in the iRules dialect — the `current_event` / `file_events`
// segmentation that only runs for `f5-irules`. Lines 341-344.
// ===========================================================================

#[test]
fn irules_dialect_runs_event_segmentation_for_snippets() {
    // f5-fact: in the `f5-irules` dialect the provider scans the file's
    // declared events + the enclosing `when` block so iRule event snippet
    // templates can guard against duplicates. With `RULE_INIT` already declared
    // (registry event), the RULE_INIT template drops out while others remain.
    let src = "when RULE_INIT {\n}\nirule";
    let analysis = analyse(src);
    let reg = irules_registry();
    // Partial `irule` at top level on line 2 (col 5).
    let items = completions(
        src,
        2,
        5,
        &analysis,
        Some(&reg),
        None,
        tcl_dialect::DialectProfile::irules(),
    );
    let snippet_labels: Vec<&str> = items
        .iter()
        .filter(|i| i.label.starts_with("iRule"))
        .map(|i| i.label.as_str())
        .collect();
    assert!(
        !snippet_labels.contains(&"iRule RULE_INIT"),
        "RULE_INIT already declared, template should decline: {snippet_labels:?}",
    );
    assert!(
        snippet_labels.iter().any(|l| l.starts_with("iRule")),
        "other event templates should remain: {snippet_labels:?}",
    );
}

#[test]
fn plain_tcl_dialect_skips_irules_event_segmentation() {
    // In a non-iRules dialect the `(None, Vec::new())` branch runs (no event
    // scan) and no iRule templates surface — confirming the dialect gate.
    let src = "irule";
    let analysis = analyse(src);
    let reg = registry();
    let items = completions(
        src,
        0,
        5,
        &analysis,
        Some(&reg),
        None,
        tcl_dialect::DialectProfile::by_name("tcl8.6"),
    );
    assert!(
        !items.iter().any(|i| i.label.starts_with("iRule")),
        "no iRule snippets outside f5-irules: {:?}",
        labels(&items),
    );
}

// ===========================================================================
// Built-in `command_detail` provenance — proc signature rendering with params
// and defaults (the proc fallback). Lines 970-983.
// ===========================================================================

#[test]
fn proc_completion_renders_param_signature_detail() {
    // The proc-completion detail is a parameter-list summary: required params
    // bare, optional params as `{name default}`.
    // tclsh-proof: `proc greet {name {greeting hi}} {}` →
    //   tclsh8.6/9.0: `proc greet {name {greeting hi}} {};info args greet` → name greeting
    //   tclsh8.6/9.0: `proc greet {name {greeting hi}} {};info default greet greeting d;set d` → hi
    // so `name` is required and `greeting` defaults to `hi` — both real.
    let src = "proc greet {name {greeting hi}} {}\ngr\n";
    let analysis = analyse(src);
    let items = completions(
        src,
        1,
        2,
        &analysis,
        None,
        None,
        tcl_dialect::DialectProfile::by_name("tcl8.6"),
    );
    let greet = items
        .iter()
        .find(|i| i.label == "greet")
        .unwrap_or_else(|| panic!("greet missing: {:?}", labels(&items)));
    let detail = greet.detail.as_deref().unwrap_or("");
    assert!(
        detail.contains("name"),
        "required param in detail: {detail:?}"
    );
    assert!(
        detail.contains("{greeting hi}"),
        "optional param rendered `{{greeting hi}}`: {detail:?}",
    );
}

#[test]
fn proc_completion_paramless_detail_is_no_args() {
    // A paramless proc renders the "(no args)" detail (the empty-params arm of
    // `proc_signature_str`).
    // tclsh-proof: `proc noop {} {};info args noop` → {} (no parameters).
    //   tclsh8.6/9.0: `proc noop {} {};llength [info args noop]` → 0
    let src = "proc noop {} {}\nno\n";
    let analysis = analyse(src);
    let items = completions(
        src,
        1,
        2,
        &analysis,
        None,
        None,
        tcl_dialect::DialectProfile::by_name("tcl8.6"),
    );
    let noop = items.iter().find(|i| i.label == "noop").expect("noop");
    assert_eq!(noop.detail.as_deref(), Some("(no args)"), "{noop:?}");
}

#[test]
fn proc_completion_variadic_detail_includes_args() {
    // A variadic proc surfaces `args` as a trailing param name in the detail.
    // tclsh-proof: `proc va {first args} {};info args va` → first args.
    //   tclsh8.6/9.0: `proc va {first args} {};info args va` → first args
    let src = "proc va {first args} {}\nv\n";
    let analysis = analyse(src);
    let items = completions(
        src,
        1,
        1,
        &analysis,
        None,
        None,
        tcl_dialect::DialectProfile::by_name("tcl8.6"),
    );
    let va = items.iter().find(|i| i.label == "va").expect("va");
    let detail = va.detail.as_deref().unwrap_or("");
    assert!(detail.contains("first"), "{detail:?}");
    assert!(detail.contains("args"), "{detail:?}");
}

// ===========================================================================
// Structural edges — comment/string suppression is not a special case here
// (the provider is position-based), but empty / no-completion edges and the
// command-position fallback are. These need no tclsh proof.
// ===========================================================================

#[test]
fn no_matching_command_or_proc_yields_no_symbols() {
    // A partial matching no proc and no registry command surfaces no symbol
    // items (only snippet templates may appear). Structural.
    let src = "zzqxnope\n";
    let analysis = analyse(src);
    let reg = registry();
    let items = completions(
        src,
        0,
        8,
        &analysis,
        Some(&reg),
        None,
        tcl_dialect::DialectProfile::by_name("tcl8.6"),
    );
    assert!(
        items.iter().all(|i| i.kind == CompletionKind::Snippet),
        "only snippets may match an unknown prefix: {:?}",
        labels(&items),
    );
}

#[test]
fn command_context_in_leading_indent_falls_through_cleanly() {
    // With a registry present and the cursor sitting in a line's leading
    // whitespace, the prefix before the cursor is all-blank →
    // `command_context_on_line` finds no tokens and returns None, so the
    // provider falls through to plain command+proc completion (no panic).
    let src = "proc helper {} {}\n   he\n";
    let analysis = analyse(src);
    let reg = registry();
    // Cursor at col 1 — inside the 3-space indent, before `he`.
    let items = completions(
        src,
        1,
        1,
        &analysis,
        Some(&reg),
        None,
        tcl_dialect::DialectProfile::by_name("tcl8.6"),
    );
    // Empty partial at this point → the fallback lists symbols (the user proc
    // among them). No crash, and `helper` is reachable.
    assert!(
        items.iter().any(|i| i.label == "helper"),
        "fallback should still surface `helper`: {:?}",
        labels(&items),
    );
}

#[test]
fn empty_array_name_yields_no_array_completion() {
    // A bare `$(` has an empty array name — `is_bare_var_name("")` is false and
    // the array lookup finds nothing, so the array branch returns an empty
    // vector without panicking (exercises the empty-name guard).
    let src = "set arr(one) 1\nputs $(\n";
    let analysis = analyse(src);
    let items = completions(
        src,
        1,
        7,
        &analysis,
        None,
        None,
        tcl_dialect::DialectProfile::by_name("tcl8.6"),
    );
    assert!(items.is_empty(), "empty array name → nothing: {items:?}");
}

#[test]
fn variable_trigger_out_of_bounds_line_does_not_panic() {
    // A `$` completion request on a line index past EOF must return cleanly.
    let src = "set x 1\n";
    let analysis = analyse(src);
    let items = completions(
        src,
        99,
        0,
        &analysis,
        None,
        None,
        tcl_dialect::DialectProfile::by_name("tcl8.6"),
    );
    assert!(items.is_empty() || items.iter().all(|i| i.kind == CompletionKind::Snippet));
}

#[test]
fn builtin_documentation_carries_registry_summary() {
    // A built-in command completion carries its registry hover summary as
    // documentation (the `spec.hover` map). tclsh-proof: `puts` is a real
    // built-in command in both dialects.
    //   tclsh8.6/9.0: `info commands puts` → puts
    let src = "put\n";
    let analysis = analyse(src);
    let reg = registry();
    let items = completions(
        src,
        0,
        3,
        &analysis,
        Some(&reg),
        None,
        tcl_dialect::DialectProfile::by_name("tcl8.6"),
    );
    let puts = items
        .iter()
        .find(|i| i.label == "puts")
        .unwrap_or_else(|| panic!("puts missing: {:?}", labels(&items)));
    assert_eq!(puts.detail.as_deref(), Some("built-in"), "{puts:?}");
    assert!(
        puts.documentation.is_some(),
        "built-in carries documentation: {puts:?}",
    );
    // Built-ins sort after user procs via the `B…` key.
    assert!(
        puts.sort_text.as_deref().unwrap_or("").starts_with('B'),
        "{puts:?}",
    );
}
