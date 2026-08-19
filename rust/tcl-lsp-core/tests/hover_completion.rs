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

//! Behaviour-driven coverage of the `tcl-lsp-core` hover,
//! signature-help, and completion providers as a single integration
//! surface. The tests below are guided by each provider's public API
//! (`hover::hover`, `signature_help::signature_help`,
//! `completion::completions`) and by Tcl semantics, to raise
//! `tcl-lsp-core` coverage.
//!
//! ## Proof split (which assertions are pinned to C-Tcl vs. editor presentation)
//!
//! The *facts* these providers surface are Tcl-semantic and are pinned
//! against real `tclsh` (8.6 and 9.0 are installed here; verified via
//! `scripts/dev/tclsh_check.sh`). The relevant ground truths, cited at
//! each test with `// tclsh:`, are:
//!
//!   * `proc greet {name body} {}; info args greet`  → `name body`
//!   * default param: `info args` → `{name greeting}`,
//!     `info default greet greeting d` → `hi`
//!   * variadic: `proc p {first args} {}; info args p` → `first args`
//!   * `info commands puts|lindex|string|set|while` → the command name
//!     (these built-ins exist in both 8.6 and 9.0)
//!   * `string length hello` → `5`  (`length` is a real `string` subcommand)
//!   * `string is alnum|alpha|ascii|digit abc` → `1` (real char classes)
//!   * namespace-qualified proc: `info args ::myns::helper` → `x y`
//!   * `namespace which parray` → `::parray` (a real autoloaded Tcl proc)
//!   * `string bogus x` / `info commands fakecmdxyz` → no such command
//!   * a user `proc puts {x} {...}` really redefines the built-in.
//!
//! Everything to do with *how* a fact is rendered — markdown framing
//! (bold-name spans and fenced `tcl` code blocks), table layout,
//! completion-item ordering, sort-text buckets, the exact synopsis wording
//! in the registry — is editor presentation, not a Tcl fact. Those are asserted
//! structurally (membership / kind / "contains") and noted inline so a
//! cosmetic copy-edit to the registry doesn't break the suite.
//!
//! F5/iRules-only completions (`when EVENT`, `call PROC`) live behind a
//! loaded iRules dialect and are out of this plain-Tcl port's surface;
//! they are marked `// f5-dialect` where mentioned.

use tcl_compiler::analyser::{Analyser, AnalysisResult};
use tcl_lsp_core::completion::{CompletionKind, completions};
use tcl_lsp_core::hover::{HoverKind, hover};
use tcl_lsp_core::signature_help::signature_help;
use tcl_registry::CommandRegistry;

// ------------------------------------------------------------------
// harness — mirrors call_hierarchy.rs / semantic_tokens.rs
// ------------------------------------------------------------------

/// Analyse `source` under the `tcl8.6` dialect (the shared harness shape
/// across these integration tests).
fn analyse(source: &str) -> AnalysisResult {
    let mut a = Analyser::new();
    a.analyse(source, "tcl8.6").clone()
}

/// A default (plain-Tcl) command registry. Built fresh per call — these
/// are cheap and the providers borrow it immutably.
fn registry() -> CommandRegistry {
    CommandRegistry::build_default()
}

// ==================================================================
// hover
// ==================================================================

#[test]
fn hover_on_user_proc_shows_signature_and_params() {
    // tclsh: `proc greet {name body} {}; info args greet` → `name body`.
    // The hover must surface both real parameters; the `proc … {…} {...}`
    // framing and ```tcl fence are presentation.
    let src = "proc greet {name body} { puts \"hi $name\" }\ngreet a b\n";
    let analysis = analyse(src);
    let h = hover(src, 0, 6, &analysis, None).expect("hover on proc name");
    assert_eq!(h.kind, HoverKind::Markdown);
    assert!(h.value.contains("greet"), "{}", h.value);
    assert!(
        h.value.contains("name"),
        "param `name` missing: {}",
        h.value
    );
    assert!(
        h.value.contains("body"),
        "param `body` missing: {}",
        h.value
    );
}

#[test]
fn hover_on_paramless_proc_has_empty_param_list() {
    // tclsh: `proc noop {} {}; info args noop` → {} (empty).
    let src = "proc noop {} { return }\nnoop\n";
    let analysis = analyse(src);
    let h = hover(src, 0, 6, &analysis, None).expect("hover on paramless proc");
    assert!(h.value.contains("noop"), "{}", h.value);
    // Rendered as `proc ::noop {} {...}` — the empty brace pair is present.
    assert!(
        h.value.contains("{}"),
        "expected empty param list: {}",
        h.value
    );
}

#[test]
fn hover_on_proc_with_default_param_shows_default() {
    // tclsh: `proc greet {name {greeting hi}} {}` →
    //   info args greet            → {name greeting}
    //   info default greet greeting d → 1, d == "hi"
    // The provider formats the optional param as `{greeting hi}`.
    let src = "proc greet {name {greeting hi}} { return }\ngreet x\n";
    let analysis = analyse(src);
    let h = hover(src, 0, 6, &analysis, None).expect("hover on proc");
    assert!(
        h.value.contains("{greeting hi}"),
        "expected defaulted-param rendering `{{greeting hi}}`: {}",
        h.value,
    );
}

#[test]
fn hover_on_variadic_proc_shows_args_param() {
    // tclsh: `proc variadic {first args} {}; info args variadic` →
    //   `first args`. `args` is surfaced as a normal trailing param name.
    let src = "proc variadic {first args} { return }\nvariadic 1 2 3\n";
    let analysis = analyse(src);
    let h = hover(src, 0, 6, &analysis, None).expect("hover on variadic proc");
    assert!(h.value.contains("first"), "{}", h.value);
    assert!(h.value.contains("args"), "{}", h.value);
}

#[test]
fn hover_on_namespace_qualified_proc() {
    // tclsh: `namespace eval ::myns {proc helper {x y} {}}`;
    //   `info args ::myns::helper` → `x y`.
    // Hovering the unqualified `helper` token resolves to the qualified
    // proc (lookup_proc matches simple name).
    let src = "namespace eval ::myns {\n    proc helper {x y} { return }\n}\n";
    let analysis = analyse(src);
    // Line 1, the `helper` token (col 9 is within `helper`).
    let h = hover(src, 1, 11, &analysis, None).expect("hover on namespaced proc");
    assert!(h.value.contains("helper"), "{}", h.value);
    assert!(
        h.value.contains('x') && h.value.contains('y'),
        "{}",
        h.value
    );
}

#[test]
fn hover_on_builtin_command_shows_summary_and_synopsis() {
    // tclsh: `info commands puts` → `puts` (exists in 8.6 and 9.0).
    // With a registry, hovering `puts` surfaces its built-in summary +
    // synopsis. Exact wording ("Write text to a channel…") is registry
    // copy (presentation); the structural facts are that it's flagged a
    // built-in and the synopsis mentions a `string` argument.
    let src = "puts hello\n";
    let analysis = analyse(src);
    let reg = registry();
    let h = hover(src, 0, 1, &analysis, Some(&reg)).expect("hover on builtin puts");
    assert!(h.value.contains("puts"), "{}", h.value);
    assert!(
        h.value.contains("built-in"),
        "expected built-in marker: {}",
        h.value,
    );
    // The synopsis fence carries the real argument shape.
    assert!(
        h.value.contains("string"),
        "synopsis arg missing: {}",
        h.value
    );
}

#[test]
fn hover_on_builtin_string_lists_subcommands() {
    // tclsh: `info commands string` → `string`; `string length hello` → 5,
    // so `length` is a real subcommand. The hover for the bare `string`
    // command lists its subcommands; `length` must be among them.
    let src = "string toupper hi\n";
    let analysis = analyse(src);
    let reg = registry();
    let h = hover(src, 0, 1, &analysis, Some(&reg)).expect("hover on string");
    assert!(h.value.contains("string"), "{}", h.value);
    assert!(
        h.value.contains("length"),
        "expected `length` in subcommand list: {}",
        h.value,
    );
}

#[test]
fn hover_on_subcommand_word_shows_subcommand() {
    // tclsh: `string length hello` → 5. With the cursor on the `length`
    // word (col 9), the provider surfaces the `string length` subcommand
    // hover, not the bare `string` command hover.
    let src = "string length hello\n";
    let analysis = analyse(src);
    let reg = registry();
    let h = hover(src, 0, 9, &analysis, Some(&reg)).expect("hover on subcommand");
    assert!(
        h.value.contains("length"),
        "expected `length` subcommand hover: {}",
        h.value,
    );
}

#[test]
fn hover_on_variable_reference_shows_reference_count() {
    // `$x` is read twice → the var hover renders a reference count. The
    // `**Variable** `x`` framing is presentation; the structural fact is
    // it resolves to the VarDef and reports the references.
    let src = "set x 1\nset y $x\nset z $x\n";
    let analysis = analyse(src);
    // Line 1, the `$x` reference — cursor on the `x` after `$` (col 7).
    let h = hover(src, 1, 7, &analysis, None).expect("hover on $x");
    assert!(h.value.contains("Variable"), "{}", h.value);
    assert!(h.value.contains('x'), "{}", h.value);
    assert!(
        h.value.to_lowercase().contains("reference"),
        "expected a reference count: {}",
        h.value,
    );
}

#[test]
fn hover_on_whitespace_yields_nothing() {
    // Cursor on a blank line / trailing whitespace resolves no word and
    // must return None (no panic).
    let src = "proc greet {name} { return }\n\n   \n";
    let analysis = analyse(src);
    assert!(hover(src, 1, 0, &analysis, None).is_none(), "blank line");
    assert!(
        hover(src, 2, 2, &analysis, None).is_none(),
        "whitespace run"
    );
}

#[test]
fn hover_on_unknown_word_without_registry_yields_nothing() {
    // tclsh: `info commands fakecmdxyz` → "" (no such command). Without a
    // registry and with no matching proc/class/var, hover returns None.
    let src = "fakecmdxyz arg\n";
    let analysis = analyse(src);
    assert!(hover(src, 0, 2, &analysis, None).is_none());
}

#[test]
fn hover_out_of_bounds_position_yields_nothing() {
    // A line index past the end of the source must not panic.
    let src = "puts hi\n";
    let analysis = analyse(src);
    assert!(hover(src, 99, 0, &analysis, None).is_none());
}

#[test]
fn hover_user_proc_shadows_builtin_of_same_name() {
    // tclsh: `proc puts {x} {return CUSTOM}; puts hello` runs the user
    // proc — the redefinition is real. So hover on `puts` here must show
    // the user proc (its `custom_chan` param), not the built-in synopsis,
    // even when a registry is supplied (proc lookup precedes the registry).
    let src = "proc puts {custom_chan} { return }\nputs hi\n";
    let analysis = analyse(src);
    let reg = registry();
    let h = hover(src, 1, 1, &analysis, Some(&reg)).expect("hover on shadowing proc");
    assert!(
        h.value.contains("custom_chan"),
        "expected the user proc's param, not the builtin synopsis: {}",
        h.value,
    );
}

// ==================================================================
// signature_help
// ==================================================================

#[test]
fn signature_on_user_proc_first_argument() {
    // tclsh: `info args greet` → `name body`. Cursor just past `greet `
    // is the first argument position → active param 0, two params shown.
    let src = "proc greet {name body} {}\ngreet \n";
    let analysis = analyse(src);
    let h = signature_help(src, 1, 6, &analysis, None).expect("sig help on first arg");
    assert_eq!(h.signatures.len(), 1);
    assert_eq!(h.signatures[0].parameters.len(), 2, "two real params");
    assert_eq!(h.active_parameter, 0);
    assert_eq!(h.signatures[0].parameters[0].label, "name");
    assert_eq!(h.signatures[0].parameters[1].label, "body");
}

#[test]
fn signature_active_param_advances_with_argument_index() {
    // tclsh: same `name body` params. After typing the first arg + a
    // space, the active parameter advances to index 1 (`body`).
    let src = "proc greet {name body} {}\ngreet alice \n";
    let analysis = analyse(src);
    let h = signature_help(src, 1, 12, &analysis, None).expect("sig help advanced");
    assert_eq!(h.active_parameter, 1);
}

#[test]
fn signature_clamps_active_param_to_last_known() {
    // tclsh: `proc one {a} {}; info args one` → `a` (one param). Typing
    // more args than the proc declares clamps the active param to 0.
    let src = "proc one {a} {}\none x y z \n";
    let analysis = analyse(src);
    let h = signature_help(src, 1, 10, &analysis, None).expect("sig help clamped");
    assert_eq!(h.active_parameter, 0, "{h:?}");
}

#[test]
fn signature_renders_default_param_brackets() {
    // tclsh: `proc greet {name {greeting hi}} {}` → params `name greeting`
    // with default `hi`. The optional param is shown as `?greeting?` in the
    // parameter list and `?greeting hi?` in the label (presentation form).
    let src = "proc greet {name {greeting hi}} {}\ngreet \n";
    let analysis = analyse(src);
    let h = signature_help(src, 1, 6, &analysis, None).expect("sig help defaults");
    let labels: Vec<&str> = h.signatures[0]
        .parameters
        .iter()
        .map(|p| p.label.as_str())
        .collect();
    assert!(labels.contains(&"name"), "{labels:?}");
    assert!(
        labels.iter().any(|l| l.contains("greeting")),
        "expected optional `greeting` param: {labels:?}",
    );
    // The signature label carries the default value.
    assert!(
        h.signatures[0].label.contains("hi"),
        "expected default `hi` in label: {}",
        h.signatures[0].label,
    );
}

#[test]
fn signature_on_builtin_command_with_registry() {
    // tclsh: `info commands puts` → `puts`. No user proc named `puts`, so
    // with a registry the cursor's command resolves to the built-in spec;
    // its synopsis (`puts ?-nonewline? ?channelId? string`) drives the
    // signature. Label starts with the command word; params are non-empty.
    let src = "puts \n";
    let analysis = analyse(src);
    let reg = registry();
    let h = signature_help(src, 0, 5, &analysis, Some(&reg)).expect("builtin sig help");
    assert_eq!(h.signatures.len(), 1);
    assert!(
        h.signatures[0].label.starts_with("puts"),
        "label {:?}",
        h.signatures[0].label,
    );
    assert!(
        !h.signatures[0].parameters.is_empty(),
        "puts takes args: {:?}",
        h.signatures[0].parameters,
    );
    // Documentation surfaces the registry summary (presentation copy).
    assert!(h.signatures[0].documentation.is_some());
}

#[test]
fn signature_on_unknown_command_without_registry_is_none() {
    // tclsh: `info commands fakecmd` → "". No proc, no registry → None.
    let src = "fakecmd arg \n";
    let analysis = analyse(src);
    assert!(signature_help(src, 0, 12, &analysis, None).is_none());
}

#[test]
fn signature_at_command_name_itself_is_none() {
    // Cursor still inside the command-name word (no argument typed) → no
    // signature help.
    let src = "proc greet {name} {}\ngre\n";
    let analysis = analyse(src);
    assert!(signature_help(src, 1, 3, &analysis, None).is_none());
}

#[test]
fn signature_user_proc_wins_over_builtin() {
    // tclsh: a user `proc puts {custom_arg} {}` redefines the built-in.
    // The signature must reflect the user proc's param, not `puts`'s
    // synopsis.
    let src = "proc puts {custom_arg} {}\nputs \n";
    let analysis = analyse(src);
    let reg = registry();
    let h = signature_help(src, 1, 5, &analysis, Some(&reg)).expect("user proc sig");
    assert!(
        h.signatures[0]
            .parameters
            .iter()
            .any(|p| p.label == "custom_arg"),
        "expected user param `custom_arg`: {:?}",
        h.signatures[0].parameters,
    );
}

#[test]
fn signature_subcommand_resolves_for_string_length() {
    // tclsh: `string length hello` → 5; `length` is a real subcommand.
    // The signature for `string length ` is the subcommand synopsis, not
    // the generic `string` one.
    let src = "string length \n";
    let analysis = analyse(src);
    let reg = registry();
    let h = signature_help(src, 0, 14, &analysis, Some(&reg)).expect("subcommand sig");
    assert!(
        h.signatures[0].label.contains("string length"),
        "label {:?}",
        h.signatures[0].label,
    );
}

#[test]
fn signature_unknown_subcommand_is_none() {
    // tclsh: `string bogus x` → "unknown or ambiguous subcommand". `bogus`
    // is not a real `string` subcommand, so the provider offers no
    // signature (it must not fall back to a generic command-level one).
    let src = "string bogus \n";
    let analysis = analyse(src);
    let reg = registry();
    assert!(
        signature_help(src, 0, 13, &analysis, Some(&reg)).is_none(),
        "unknown subcommand should yield no signature",
    );
}

// ==================================================================
// completion
// ==================================================================

#[test]
fn completion_of_command_prefix_lists_matching_builtins() {
    // tclsh: `info commands while` → `while` (real built-in). Partial `whi`
    // at a command position should surface `while` and exclude unrelated
    // commands like `puts`.
    let src = "whi\n";
    let analysis = analyse(src);
    let reg = registry();
    let items = completions(src, 0, 3, &analysis, Some(&reg), None, tcl_dialect::DialectProfile::by_name("tcl8.6"));
    let labels: Vec<&str> = items.iter().map(|i| i.label.as_str()).collect();
    assert!(labels.contains(&"while"), "expected `while`: {labels:?}");
    assert!(
        !labels.contains(&"puts"),
        "`puts` should not match `whi`: {labels:?}"
    );
}

#[test]
fn completion_of_var_prefix_lists_in_scope_variables() {
    // `$ap` should list the in-scope `$apple` variable (a `set` binding),
    // not `$banana`. The `$`-prefixed label and Variable kind are the
    // structural contract.
    let src = "set apple 1\nset banana 2\nset z $ap\n";
    let analysis = analyse(src);
    let items = completions(src, 2, 8, &analysis, None, None, tcl_dialect::DialectProfile::by_name("tcl8.6"));
    let labels: Vec<&str> = items.iter().map(|i| i.label.as_str()).collect();
    assert_eq!(labels, vec!["$apple"], "{items:?}");
    assert_eq!(items[0].kind, CompletionKind::Variable);
}

#[test]
fn completion_of_bare_dollar_lists_all_globals_sorted() {
    // A bare `$` lists every global, alphabetically (ordering is the
    // provider's presentation choice).
    let src = "set apple 1\nset banana 2\nset cherry 3\nset z $\n";
    let analysis = analyse(src);
    let items = completions(src, 3, 7, &analysis, None, None, tcl_dialect::DialectProfile::by_name("tcl8.6"));
    let labels: Vec<&str> = items.iter().map(|i| i.label.as_str()).collect();
    assert!(labels.contains(&"$apple"), "{labels:?}");
    assert!(labels.contains(&"$banana"), "{labels:?}");
    assert!(labels.contains(&"$cherry"), "{labels:?}");
    let mut sorted = labels.clone();
    sorted.sort_unstable();
    assert_eq!(labels, sorted, "expected alphabetical order");
}

#[test]
fn completion_inside_proc_includes_its_params_and_locals() {
    // A proc parameter (`count`) and a body-local (`total`) are both
    // in-scope variables. Completing `$` inside the body lists both, plus
    // they out-rank any enclosing-scope-only names. (Scope resolution is
    // the Tcl fact; presentation is the `$name` form.)
    let src = concat!(
        "proc tally {count} {\n",
        "    set total 0\n",
        "    set x $\n",
        "}\n",
    );
    let analysis = analyse(src);
    // Line 2, cursor right after the `$` (col 12).
    let items = completions(src, 2, 12, &analysis, None, None, tcl_dialect::DialectProfile::by_name("tcl8.6"));
    let labels: Vec<&str> = items.iter().map(|i| i.label.as_str()).collect();
    assert!(
        labels.contains(&"$count"),
        "param `count` missing: {labels:?}"
    );
    assert!(
        labels.contains(&"$total"),
        "local `total` missing: {labels:?}"
    );
}

#[test]
fn completion_lists_user_defined_procs() {
    // tclsh: a user proc `greet` is a real command. Partial `g` at command
    // position surfaces it as a Function-kind item.
    let src = "proc greet {} {}\nproc shout {} {}\ng\n";
    let analysis = analyse(src);
    let items = completions(src, 2, 1, &analysis, None, None, tcl_dialect::DialectProfile::by_name("tcl8.6"));
    let labels: Vec<&str> = items.iter().map(|i| i.label.as_str()).collect();
    assert!(labels.contains(&"greet"), "{labels:?}");
    assert!(
        !labels.contains(&"shout"),
        "`shout` shouldn't match `g`: {labels:?}"
    );
    let greet = items.iter().find(|i| i.label == "greet").unwrap();
    assert_eq!(greet.kind, CompletionKind::Function);
}

#[test]
fn completion_of_namespace_qualified_command() {
    // tclsh: `namespace which parray` → `::parray` (parray is a real
    // autoloaded Tcl proc; the registry carries it as a stdlib command).
    // Partial `par` surfaces `parray`. The insert/label uses the
    // namespace-qualified name shape the registry records.
    let src = "par\n";
    let analysis = analyse(src);
    let reg = registry();
    let items = completions(src, 0, 3, &analysis, Some(&reg), None, tcl_dialect::DialectProfile::by_name("tcl8.6"));
    let labels: Vec<&str> = items.iter().map(|i| i.label.as_str()).collect();
    assert!(labels.contains(&"parray"), "expected `parray`: {labels:?}");
}

#[test]
fn completion_merges_user_procs_and_builtins() {
    // A user proc and built-ins sharing a prefix both surface. tclsh:
    // `parray` is a real stdlib proc; `parade` is the user's. Both start
    // with `par`.
    let src = "proc parade {} {}\npar\n";
    let analysis = analyse(src);
    let reg = registry();
    let items = completions(src, 1, 3, &analysis, Some(&reg), None, tcl_dialect::DialectProfile::by_name("tcl8.6"));
    let labels: Vec<&str> = items.iter().map(|i| i.label.as_str()).collect();
    assert!(labels.contains(&"parade"), "user proc: {labels:?}");
    assert!(labels.contains(&"parray"), "built-in: {labels:?}");
}

#[test]
fn completion_subcommand_at_word_index_1() {
    // tclsh: `string length hello` → 5; `length` is a real `string`
    // subcommand. Partial `l` at word-index 1 of `string` lists `length`
    // and excludes unrelated commands.
    let src = "string l\n";
    let analysis = analyse(src);
    let reg = registry();
    let items = completions(src, 0, 8, &analysis, Some(&reg), None, tcl_dialect::DialectProfile::by_name("tcl8.6"));
    let labels: Vec<&str> = items.iter().map(|i| i.label.as_str()).collect();
    assert!(labels.contains(&"length"), "expected `length`: {labels:?}");
    assert!(
        !labels.contains(&"puts"),
        "no `puts` in subcommand context: {labels:?}"
    );
}

#[test]
fn completion_string_is_lists_real_character_classes() {
    // tclsh: `string is alnum|alpha|ascii abc` → 1 (all real classes);
    // `digit` is also real. Partial `a` at the class arg of `string is`
    // lists the `a*` classes as EnumValue items, excluding `digit`.
    let src = "string is a\n";
    let analysis = analyse(src);
    let reg = registry();
    let items = completions(src, 0, 11, &analysis, Some(&reg), None, tcl_dialect::DialectProfile::by_name("tcl8.6"));
    let labels: Vec<&str> = items.iter().map(|i| i.label.as_str()).collect();
    assert!(labels.contains(&"alnum"), "{labels:?}");
    assert!(labels.contains(&"alpha"), "{labels:?}");
    assert!(labels.contains(&"ascii"), "{labels:?}");
    assert!(
        !labels.contains(&"digit"),
        "`digit` filtered by partial `a`: {labels:?}"
    );
    for it in &items {
        assert_eq!(it.kind, CompletionKind::EnumValue, "{it:?}");
    }
}

#[test]
fn completion_switch_completes_command_options() {
    // tclsh: `lsearch -nocase …` is a real switch. Partial `-n` after
    // `lsearch` lists matching options; every item starts with `-`.
    let src = "lsearch -n\n";
    let analysis = analyse(src);
    let reg = registry();
    let items = completions(src, 0, 10, &analysis, Some(&reg), None, tcl_dialect::DialectProfile::by_name("tcl8.6"));
    let labels: Vec<&str> = items.iter().map(|i| i.label.as_str()).collect();
    assert!(!labels.is_empty(), "expected `-n*` switches: {labels:?}");
    assert!(
        labels.iter().all(|l| l.starts_with('-')),
        "all switch items must start with `-`: {labels:?}",
    );
}

#[test]
fn completion_without_registry_yields_procs_only() {
    // No registry → no built-in commands, only the user proc surfaces.
    let src = "proc helper {} {}\nhe\n";
    let analysis = analyse(src);
    let items = completions(src, 1, 2, &analysis, None, None, tcl_dialect::DialectProfile::by_name("tcl8.6"));
    let labels: Vec<&str> = items.iter().map(|i| i.label.as_str()).collect();
    assert_eq!(labels, vec!["helper"], "{items:?}");
}

#[test]
fn completion_variable_trigger_suppresses_commands() {
    // A `$`-led partial is a variable context: even with a registry, only
    // Variable-kind items come back (the command/proc fallback is skipped).
    let src = "set apple 1\nset z $ap\n";
    let analysis = analyse(src);
    let reg = registry();
    let items = completions(src, 1, 8, &analysis, Some(&reg), None, tcl_dialect::DialectProfile::by_name("tcl8.6"));
    assert!(
        !items.is_empty(),
        "expected variable completions: {items:?}"
    );
    for it in &items {
        assert_eq!(it.kind, CompletionKind::Variable, "{it:?}");
    }
}

#[test]
fn completion_on_empty_source_does_not_panic() {
    // Degenerate inputs must return cleanly (empty / no panic).
    let analysis = analyse("");
    let items = completions("", 0, 0, &analysis, None, None, tcl_dialect::DialectProfile::by_name("tcl8.6"));
    // No procs, no registry → nothing to suggest at an empty command pos.
    assert!(
        items.iter().all(|i| i.kind == CompletionKind::Snippet),
        "only snippet templates may appear on empty source: {items:?}",
    );
}

// f5-dialect: `when EVENT` event-name completion and `call PROC` user-proc
// completion require a loaded iRules dialect registry and are exercised in
// completion.rs's in-crate tests; they are out of this plain-Tcl port's
// surface.

// ==================================================================
// Issue #806 — report::defstyle scoped command environment.
//
// Inside a report::defstyle style script, the report configuration
// methods (top/data/columns/…) are available as commands.  Hover and
// completion resolve them from the registry-declared scoped environment.
// ==================================================================

const DEFSTYLE_BODY: &str =
    "::report::defstyle simpletable {} {\n    top set foo\n    columns\n}\n";

#[test]
fn hover_on_scoped_line_command() {
    let analysis = analyse(DEFSTYLE_BODY);
    let reg = registry();
    // `top` on line 1 (char 5).
    let h = hover(DEFSTYLE_BODY, 1, 5, &analysis, Some(&reg)).expect("hover on scoped `top`");
    assert!(
        h.value.contains("report style"),
        "env name present: {}",
        h.value
    );
    assert!(h.value.contains("separator"), "top described: {}", h.value);
}

#[test]
fn hover_on_scoped_operation() {
    let analysis = analyse(DEFSTYLE_BODY);
    let reg = registry();
    // `set` operation on line 1 (char 9).
    let h = hover(DEFSTYLE_BODY, 1, 9, &analysis, Some(&reg)).expect("hover on scoped op `set`");
    assert!(
        h.value.contains("operation"),
        "operation marker: {}",
        h.value
    );
    assert!(h.value.contains("top set"), "head+op named: {}", h.value);
}

#[test]
fn hover_on_scoped_config_command() {
    let analysis = analyse(DEFSTYLE_BODY);
    let reg = registry();
    // `columns` on line 2 (char 7).
    let h = hover(DEFSTYLE_BODY, 2, 7, &analysis, Some(&reg)).expect("hover on scoped `columns`");
    assert!(h.value.contains("columns"), "name present: {}", h.value);
    assert!(
        h.value.contains("number of columns"),
        "described: {}",
        h.value
    );
}

#[test]
fn hover_on_scoped_command_only_inside_body() {
    // `top` outside any style body is not a scoped command → no scoped hover.
    let src = "top set foo\n";
    let analysis = analyse(src);
    let reg = registry();
    let h = hover(src, 0, 1, &analysis, Some(&reg));
    // Either no hover, or at least not the scoped-environment hover.
    assert!(h.is_none_or(|x| !x.value.contains("report style")));
}

#[test]
fn completion_offers_scoped_command_heads() {
    let src = "::report::defstyle st {} {\n    to\n}\n";
    let analysis = analyse(src);
    let reg = registry();
    let items = completions(src, 1, 6, &analysis, Some(&reg), None, tcl_dialect::DialectProfile::by_name("tcl8.6"));
    let labels: Vec<&str> = items.iter().map(|i| i.label.as_str()).collect();
    assert!(labels.contains(&"top"), "offers `top`: {labels:?}");
    assert!(labels.contains(&"topdata"), "offers `topdata`: {labels:?}");
    assert!(
        labels.contains(&"topdatasep"),
        "offers `topdatasep`: {labels:?}"
    );
}

#[test]
fn completion_offers_scoped_operations() {
    let src = "::report::defstyle st {} {\n    top \n}\n";
    let analysis = analyse(src);
    let reg = registry();
    let items = completions(src, 1, 8, &analysis, Some(&reg), None, tcl_dialect::DialectProfile::by_name("tcl8.6"));
    let labels: Vec<&str> = items.iter().map(|i| i.label.as_str()).collect();
    for op in ["set", "get", "enable", "disable", "enabled"] {
        assert!(labels.contains(&op), "offers op `{op}`: {labels:?}");
    }
}

#[test]
fn completion_scoped_heads_not_offered_outside_body() {
    // At top level, `to` must not surface the scoped-only `topdatasep`.
    let src = "to\n";
    let analysis = analyse(src);
    let reg = registry();
    let items = completions(src, 0, 2, &analysis, Some(&reg), None, tcl_dialect::DialectProfile::by_name("tcl8.6"));
    let labels: Vec<&str> = items.iter().map(|i| i.label.as_str()).collect();
    assert!(
        !labels.contains(&"topdatasep"),
        "scoped-only head leaked: {labels:?}"
    );
}

// ===========================================================================
// M11 (TIP 278) — completion qualifies globals per the dialect's
// namespace-scope semantics.
// ===========================================================================

#[test]
fn completion_ns_scope_global_qualification_follows_the_dialect_m11() {
    // tclsh8.6: a bare name at namespace scope reaches an unshadowed global
    // (`set flag 1; namespace eval foo { set flag }` → 1), so `$flag` is the
    // right insertion.  tclsh9.0 removed the fallback (the same script
    // errors), so only the qualified `$::flag` reaches the global.
    let src = "set flag 1\nnamespace eval foo {\n    set x $\n}\n";
    let col = u32::try_from(src.lines().nth(2).unwrap().find('$').unwrap())
        .expect("tiny test source")
        + 1;

    let mut a = Analyser::new();
    let a86 = a.analyse(src, "tcl8.6").clone();
    let labels: Vec<String> = completions(src, 2, col, &a86, None, None, tcl_dialect::DialectProfile::by_name("tcl8.6"))
        .into_iter()
        .map(|i| i.label)
        .collect();
    assert!(
        labels.iter().any(|l| l == "$flag"),
        "8.x: the bare `$flag` genuinely reaches the global: {labels:?}"
    );
    assert!(
        !labels.iter().any(|l| l == "$::flag"),
        "8.x: no forced qualification: {labels:?}"
    );

    let mut a = Analyser::new();
    let a90 = a.analyse(src, "tcl9.0").clone();
    let labels: Vec<String> = completions(src, 2, col, &a90, None, None, tcl_dialect::DialectProfile::by_name("tcl9.0"))
        .into_iter()
        .map(|i| i.label)
        .collect();
    assert!(
        labels.iter().any(|l| l == "$::flag"),
        "9.0: only `$::flag` reaches the global: {labels:?}"
    );
    assert!(
        !labels.iter().any(|l| l == "$flag"),
        "9.0: inserting a bare `$flag` would error at runtime: {labels:?}"
    );
}

#[test]
fn completion_proc_scope_still_qualifies_globals_in_every_dialect_m11() {
    // Proc frames never had the namespace fallback in any Tcl version —
    // the 8.x gate must not leak into them.
    let src = "set flag 1\nproc p {} {\n    set x $\n}\n";
    let col = u32::try_from(src.lines().nth(2).unwrap().find('$').unwrap())
        .expect("tiny test source")
        + 1;
    for dialect in ["tcl8.6", "tcl9.0"] {
        let mut a = Analyser::new();
        let an = a.analyse(src, dialect).clone();
        let labels: Vec<String> = completions(src, 2, col, &an, None, None, tcl_dialect::DialectProfile::by_name(dialect))
            .into_iter()
            .map(|i| i.label)
            .collect();
        assert!(
            labels.iter().any(|l| l == "$::flag"),
            "[{dialect}] proc scope: global offered qualified: {labels:?}"
        );
    }
}

// ---------------------------------------------------------------------------
// Issue #1256 — completion and hover read the declared boolean argument role.
//
// tclsh (8.6.16 / 9.0.4): `fconfigure` / `chan configure -blocking` accepts
// every boolean spelling and reports the canonical integer back —
//   set c [open /dev/null]; fconfigure $c -blocking yes; fconfigure $c -blocking
//     -> 1
// so the whole boolean vocabulary is legal at the position; the registry now
// declares that with `ArgRole::Boolean` rather than leaving it implicit.
// ---------------------------------------------------------------------------

#[test]
fn completion_offers_the_boolean_vocabulary_at_a_boolean_option_value() {
    let src = "fconfigure $c -blocking \n";
    let analysis = analyse(src);
    let reg = registry();
    let items = completions(src, 0, 24, &analysis, Some(&reg), None, tcl_dialect::DialectProfile::by_name("tcl8.6"));
    let labels: Vec<&str> = items.iter().map(|i| i.label.as_str()).collect();
    for want in ["true", "false", "yes", "no", "on", "off", "0", "1"] {
        assert!(labels.contains(&want), "expected `{want}` in {labels:?}");
    }
    for it in &items {
        assert_eq!(it.kind, CompletionKind::EnumValue, "{it:?}");
    }
}

#[test]
fn completion_filters_the_boolean_vocabulary_by_partial() {
    let src = "fconfigure $c -blocking t\n";
    let analysis = analyse(src);
    let reg = registry();
    let items = completions(src, 0, 25, &analysis, Some(&reg), None, tcl_dialect::DialectProfile::by_name("tcl8.6"));
    let labels: Vec<&str> = items.iter().map(|i| i.label.as_str()).collect();
    assert!(labels.contains(&"true"), "{labels:?}");
    assert!(!labels.contains(&"false"), "filtered by `t`: {labels:?}");
}

#[test]
fn hover_on_a_boolean_option_names_the_vocabulary() {
    let src = "fconfigure $c -blocking yes\n";
    let analysis = analyse(src);
    let reg = registry();
    let h = hover(src, 0, 16, &analysis, Some(&reg)).expect("option hover");
    assert!(
        h.value.contains("boolean") && h.value.contains("`yes`"),
        "hover must name the boolean vocabulary: {}",
        h.value
    );
}
