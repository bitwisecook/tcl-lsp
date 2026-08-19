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

//! TP/FP/TN/FN matrix for issue #974 (`expr` math functions in hover /
//! completion / navigation), issue #1054 (hover type inference built without
//! the document's dialect), and the issue #923 differential-audit findings
//! idx 24 / 30 / 48 / 54 / 103 / 104.
//!
//! Every oracle claim in here was checked against tclsh 9.0.4 **and** 8.6.16:
//!
//! ```tcl
//! # namespace-relative mathfunc resolution (TIP 232)
//! namespace eval ::tcl::mathfunc { proc g {x} { expr {$x + 1000} } }
//! namespace eval ::foo {
//!     namespace eval tcl::mathfunc { proc f {x} { expr {$x * 100} } }
//!     proc use  {} { expr {f(2)} }   ;# 200  -> ::foo::tcl::mathfunc::f
//!     proc useg {} { expr {g(2)} }   ;# 1002 -> ::tcl::mathfunc::g
//! }
//! proc f {x} { return GLOBAL-PROC-f }   ;# never reached from expr
//! # a mathfunc proc is NOT an ordinary command
//! namespace eval tcl::mathfunc { proc li {l i args} { lindex $l $i } }
//! li {10 20 30} 1                        ;# invalid command name "li"
//! # single colons are ordinary name characters; only `::` separates
//! set a:b 42 ; proc p:q {x} {…} ; set arr(k:1) v ; dict get $d x:y   ;# all fine
//! # a brace-quoted word is emitted verbatim
//! set t {plain $level here}              ;# prints: plain $level here
//! ```

use tcl_compiler::analyser::{Analyser, AnalysisResult};
use tcl_registry::registry_for_dialect;

fn analyse(source: &str) -> AnalysisResult {
    Analyser::new().analyse(source, "tcl8.6").clone()
}

fn analyse_as(source: &str, dialect: &str) -> AnalysisResult {
    Analyser::new().analyse(source, dialect).clone()
}

/// Hover markdown at (`line`, `col`) under `dialect`.
fn hover_at(src: &str, dialect: &str, line: u32, col: u32) -> Option<String> {
    let analysis = analyse_as(src, dialect);
    tcl_lsp_core::hover::hover_with_profile(
        src,
        line,
        col,
        &analysis,
        Some(registry_for_dialect(dialect)),
        tcl_dialect::DialectProfile::by_name(dialect),
    )
    .map(|h| h.value)
}

/// Completion items at (`line`, `col`) under `dialect`.
fn completion_items(
    src: &str,
    dialect: &str,
    line: u32,
    col: u32,
) -> Vec<tcl_lsp_core::completion::CompletionItem> {
    let analysis = analyse_as(src, dialect);
    tcl_lsp_core::completion::completions(
        src,
        line,
        col,
        &analysis,
        Some(registry_for_dialect(dialect)),
        None,
        tcl_dialect::DialectProfile::by_name(dialect),
    )
}

/// Completion labels at (`line`, `col`) under `dialect`.
fn completion_labels(src: &str, dialect: &str, line: u32, col: u32) -> Vec<String> {
    completion_items(src, dialect, line, col)
        .into_iter()
        .map(|i| i.label)
        .collect()
}

/// The `(line, col)` of the first character of `needle` in `src`.
fn pos_of(src: &str, needle: &str) -> (u32, u32) {
    let off = src.find(needle).expect("needle present");
    let line = src[..off].matches('\n').count();
    let line_start = src[..off].rfind('\n').map_or(0, |nl| nl + 1);
    (
        u32::try_from(line).expect("fits"),
        u32::try_from(src[line_start..off].chars().count()).expect("fits"),
    )
}

// ---------------------------------------------------------------------------
// Issue #974 defect 1 — hover on a bare mathfunc call inside `expr`
// ---------------------------------------------------------------------------

/// TP (the reported FN): `set a [expr {sin(1.0)}]` drew nothing at any column
/// of `sin`; it must now render the same registry data the qualified spelling
/// already did.
#[test]
fn tp_hover_on_a_bare_mathfunc_call_renders_registry_docs() {
    let src = "set a [expr {sin(1.0)}]\n";
    let (line, col) = pos_of(src, "sin");
    for probe in [col, col + 1, col + 2] {
        let text = hover_at(src, "tcl8.6", line, probe)
            .unwrap_or_else(|| panic!("no hover at column {probe}"));
        assert!(text.contains("sin"), "{text}");
        assert!(text.contains("math function"), "{text}");
        assert!(
            text.contains("sine"),
            "summary from the registry spec: {text}"
        );
        assert!(
            text.contains("::tcl::mathfunc::sin"),
            "names the command it dispatches to: {text}"
        );
    }
}

/// TP: a variadic function, and one whose bare word could be mistaken for a
/// registry command tail.
#[test]
fn tp_hover_on_max_and_min_in_expr() {
    for (src, name) in [
        ("set a [expr {max(1,2)}]\n", "max"),
        ("set a [expr {min($x,$y)}]\n", "min"),
        ("if {abs($d) > 1} { puts hi }\n", "abs"),
    ] {
        let (line, col) = pos_of(src, name);
        let text = hover_at(src, "tcl8.6", line, col)
            .unwrap_or_else(|| panic!("no hover for {name} in {src:?}"));
        assert!(text.contains(name), "{text}");
        assert!(text.contains("math function"), "{text}");
    }
}

/// TP: a user override wins — `proc ::tcl::mathfunc::li` is a real command in
/// the table C Tcl dispatches through, so hover shows the proc, not the
/// built-in.
#[test]
fn tp_hover_on_a_user_overridden_mathfunc_shows_the_proc() {
    let src = "\
namespace eval tcl::mathfunc {
    proc li {list index args} {
        ::lindex $list $index {*}$args
    }
}
proc caller {} {
    if {li({0 1 0},1)} { return yes }
    return no
}
";
    let (line, col) = pos_of(src, "li({0 1 0}");
    let text = hover_at(src, "tcl8.6", line, col).expect("hover at the override call site");
    assert!(
        text.contains("proc ::tcl::mathfunc::li"),
        "the override, not the built-in: {text}"
    );
}

/// TN: the same word outside any expression draws nothing new — a variable
/// named `sin`, and a bare command word `sin` (which real Tcl cannot resolve:
/// `sin` is not an ordinary command).
#[test]
fn tn_no_mathfunc_hover_outside_an_expression() {
    let src = "set sin 1\nputs $sin\n";
    let (line, col) = pos_of(src, "$sin");
    let text = hover_at(src, "tcl8.6", line, col + 1).expect("the variable still hovers");
    assert!(text.contains("**Variable**"), "{text}");
    assert!(!text.contains("math function"), "{text}");

    // A command word `sin` is not a math function: nothing new appears.
    let cmd = "sin 1.0\n";
    let text = hover_at(cmd, "tcl8.6", 0, 1);
    assert!(
        text.as_deref().is_none_or(|t| !t.contains("math function")),
        "{text:?}"
    );
}

/// TN: a `[…]` substitution nested inside an expression re-enters *script*
/// context, so `sin` there is a command word, not a function call.
#[test]
fn tn_no_mathfunc_hover_for_a_command_word_inside_an_expression() {
    let src = "set a [expr {[sin 1.0]}]\n";
    let (line, col) = pos_of(src, "sin 1.0");
    let text = hover_at(src, "tcl8.6", line, col);
    assert!(
        text.as_deref().is_none_or(|t| !t.contains("math function")),
        "{text:?}"
    );
}

/// Version gates, from the registry's own data: `isnan` is TIP 521 (9.0+) and
/// `gamma` is TIP 745 (9.1+), while `abs` reaches back to the 8.4 `expr`
/// grammar even though the *command* `::tcl::mathfunc::abs` is 8.5+.
#[test]
fn mathfunc_hover_is_version_gated() {
    let isnan = "set a [expr {isnan(1.0)}]\n";
    let (line, col) = pos_of(isnan, "isnan");
    assert!(
        hover_at(isnan, "tcl9.0", line, col).is_some_and(|t| t.contains("math function")),
        "isnan is 9.0+"
    );
    assert!(
        hover_at(isnan, "tcl8.6", line, col)
            .as_deref()
            .is_none_or(|t| !t.contains("math function")),
        "isnan must not render under 8.6"
    );

    let gamma = "set a [expr {gamma(1.0)}]\n";
    let (line, col) = pos_of(gamma, "gamma");
    assert!(
        hover_at(gamma, "tcl9.1", line, col).is_some_and(|t| t.contains("math function")),
        "gamma is 9.1+"
    );
    assert!(
        hover_at(gamma, "tcl9.0", line, col)
            .as_deref()
            .is_none_or(|t| !t.contains("math function")),
        "gamma must not render under 9.0"
    );

    // The `expr`-grammar axis, not the command-table axis: `abs(…)` works in
    // 8.4 even though `::tcl::mathfunc::abs` does not exist there.
    let abs = "set a [expr {abs(-1)}]\n";
    let (line, col) = pos_of(abs, "abs");
    assert!(
        hover_at(abs, "tcl8.4", line, col).is_some_and(|t| t.contains("math function")),
        "abs(…) is an 8.4 expr function"
    );
    // `max` is TIP 232, so it is *not*.
    let max = "set a [expr {max(1,2)}]\n";
    let (line, col) = pos_of(max, "max");
    assert!(
        hover_at(max, "tcl8.4", line, col)
            .as_deref()
            .is_none_or(|t| !t.contains("math function")),
        "max is 8.5+"
    );
}

// ---------------------------------------------------------------------------
// Issue #974 defect 2 — completion inside `expr`
// ---------------------------------------------------------------------------

/// TP (the reported FN): `set a [expr {si` offered only same-prefixed procs
/// (`simulation::*` in the audited corpus) and no math functions at all.
#[test]
fn tp_completion_offers_math_functions_inside_expr() {
    let src = "proc sizzler {} {}\nset a [expr {si";
    let items = completion_items(src, "tcl8.6", 1, 15);
    let labels: Vec<&str> = items.iter().map(|i| i.label.as_str()).collect();
    assert!(labels.contains(&"sin"), "{labels:?}");
    assert!(labels.contains(&"sinh"), "{labels:?}");
    assert!(
        labels.contains(&"sizzler"),
        "the proc still surfaces: {labels:?}"
    );
    let sin = items
        .iter()
        .find(|i| i.label == "sin")
        .expect("sin present");
    assert_eq!(sin.detail.as_deref(), Some("expr math function"), "{sin:?}");
    // Ranked in the local-symbol tier, ahead of the `B…` built-ins.
    assert!(
        sin.sort_text
            .as_deref()
            .is_some_and(|s| s.starts_with("A0")),
        "{sin:?}"
    );
}

/// TP: an `if` / `while` condition is an EXPR-role argument too — the
/// registry decides, no command is named in the LSP.
#[test]
fn tp_completion_offers_math_functions_in_if_and_while_conditions() {
    for src in ["if {si", "while {si", "for {set i 0} {si"] {
        let col = u32::try_from(src.chars().count()).expect("fits");
        let labels = completion_labels(src, "tcl8.6", 0, col);
        assert!(labels.contains(&"sin".to_owned()), "{src:?} -> {labels:?}");
    }
}

/// TN: the same prefix outside an expression keeps its previous behaviour —
/// no math functions.
#[test]
fn tn_completion_outside_expr_offers_no_math_functions() {
    let src = "proc sizzler {} {}\nsi";
    let labels = completion_labels(src, "tcl8.6", 1, 2);
    assert!(!labels.contains(&"sin".to_owned()), "{labels:?}");
    assert!(labels.contains(&"sizzler".to_owned()), "{labels:?}");

    // A body argument is not an expression.
    let body = "if {1} {si";
    let labels = completion_labels(body, "tcl8.6", 0, 10);
    assert!(!labels.contains(&"sin".to_owned()), "{labels:?}");
}

/// Version gating is pinned for completion as well.
#[test]
fn completion_math_functions_are_version_gated() {
    let src = "set a [expr {ma";
    let labels86 = completion_labels(src, "tcl8.6", 0, 15);
    assert!(labels86.contains(&"max".to_owned()), "{labels86:?}");
    let labels84 = completion_labels(src, "tcl8.4", 0, 15);
    assert!(
        !labels84.contains(&"max".to_owned()),
        "max is 8.5+: {labels84:?}"
    );

    let g = "set a [expr {ga";
    assert!(completion_labels(g, "tcl9.1", 0, 15).contains(&"gamma".to_owned()));
    assert!(!completion_labels(g, "tcl9.0", 0, 15).contains(&"gamma".to_owned()));
}

// ---------------------------------------------------------------------------
// Issue #923 idx 30 — mathfunc call sites as first-class references
// ---------------------------------------------------------------------------

/// The audit's own six-consumer sweep shape: a namespace-local
/// `proc ::ns::tcl::mathfunc::f` override plus an *unrelated* same-named
/// global `proc f`.  tclsh proves `f(2)` inside `::ns` reaches the override
/// and never the global proc.
const SWEEP: &str = "\
namespace eval ns {
    namespace eval tcl::mathfunc {
        proc f {x} { return [expr {$x * 100}] }
    }
    proc use {} {
        return [expr {f(2)}]
    }
}
proc f {x} { return GLOBAL }
puts [f 1]
";

/// TP: go-to-definition from the `f(2)` call site reaches the override, not
/// the unrelated global proc.
#[test]
fn tp_definition_at_a_mathfunc_call_site_reaches_the_override() {
    let analysis = analyse(SWEEP);
    let (line, col) = pos_of(SWEEP, "f(2)");
    let locs = tcl_lsp_core::definition::definition(SWEEP, line, col, &analysis);
    assert_eq!(locs.len(), 1, "{locs:?}");
    // The override's own `proc f` declaration is on line 2.
    assert_eq!(locs[0].start_line, 2, "{:?}", locs[0]);
}

/// TP (the reported asymmetry): find-references from the *call site* now
/// returns the same set a query from the declaration does.
#[test]
fn tp_references_are_symmetric_at_a_mathfunc_call_site() {
    let analysis = analyse(SWEEP);
    let (decl_line, decl_col) = pos_of(SWEEP, "f {x} { return [expr {$x * 100}] }");
    let from_decl = tcl_lsp_core::references::references(
        SWEEP,
        tcl_dialect::DialectProfile::by_name("tcl8.6"),
        decl_line,
        decl_col,
        &analysis,
        true,
    );
    let (call_line, call_col) = pos_of(SWEEP, "f(2)");
    let from_call = tcl_lsp_core::references::references(
        SWEEP,
        tcl_dialect::DialectProfile::by_name("tcl8.6"),
        call_line,
        call_col,
        &analysis,
        true,
    );
    assert!(
        !from_decl.is_empty(),
        "declaration-anchored set is non-empty"
    );
    assert_eq!(
        from_call.len(),
        from_decl.len(),
        "call-site set must equal the declaration-anchored one:\n  call {from_call:?}\n  decl {from_decl:?}"
    );
    // Neither set may include the unrelated global `proc f` (line 8).
    assert!(
        from_call.iter().all(|r| r.start_line != 8),
        "the unrelated global proc must not appear: {from_call:?}"
    );
}

/// TP: the reference-count **lens** — the fourth surface fed by the same
/// `proc_reference_spans` substrate — agrees with the symmetric reference set
/// above: exactly one call site for the override (`f(2)`), and exactly one for
/// the unrelated global `proc f` (`puts [f 1]`).  The two must not be pooled:
/// tclsh 9.0.4 / 8.6.16 print `200` for `ns::use` and `GLOBAL` for `f 1`, so
/// they are two different commands that merely share a tail.
#[test]
fn tp_reference_lens_counts_the_mathfunc_override_and_the_global_proc_apart() {
    let analysis = analyse(SWEEP);
    let lenses = tcl_lsp_core::code_lens::code_lenses(
        SWEEP,
        tcl_dialect::DialectProfile::by_name("tcl8.6"),
        Some(&analysis),
        None,
        "",
    );
    let title_for = |qname: &str| {
        let Some(lens) = lenses.iter().find(|lens| lens.qname == qname) else {
            panic!(
                "no lens for {qname}; got {:?}",
                lenses.iter().map(|l| &l.qname).collect::<Vec<_>>()
            )
        };
        lens.command_title.clone()
    };
    assert_eq!(
        title_for("::ns::tcl::mathfunc::f"),
        "1 reference",
        "the `f(2)` application is the override's one call site",
    );
    assert_eq!(
        title_for("::f"),
        "1 reference",
        "`puts [f 1]` is the global proc's one call site, and the expr \
         application is not one of them",
    );
}

/// TP: call hierarchy and linked-editing — the other two consumers that ride
/// on `resolve_proc_target_at` — reach the override from the call site too.
#[test]
fn tp_call_hierarchy_and_linked_editing_reach_the_override() {
    let analysis = analyse(SWEEP);
    let (line, col) = pos_of(SWEEP, "f(2)");
    let items = tcl_lsp_core::call_hierarchy::prepare(SWEEP, line, col, &analysis);
    assert_eq!(items.len(), 1, "{items:?}");
    assert_eq!(items[0].name, "f", "{items:?}");
    assert_eq!(
        items[0].selection_range.start_line, 2,
        "the override's declaration: {items:?}"
    );

    // Linked editing anchors on the declaration whose body contains the
    // cursor; from the call site inside `use` it must not link the unrelated
    // global proc's name.
    let ranges =
        tcl_lsp_core::linked_editing_range::linked_editing_ranges(SWEEP, line, col, &analysis);
    if let Some(r) = ranges {
        assert!(
            r.ranges.iter().all(|x| x.start_line != 8),
            "must never link the unrelated global proc: {r:?}"
        );
    }
}

/// TP: hover and completion — the two consumers with their own e2e coverage —
/// agree with the rest of the sweep.
#[test]
fn tp_hover_and_completion_agree_with_the_sweep() {
    let (line, col) = pos_of(SWEEP, "f(2)");
    let text = hover_at(SWEEP, "tcl8.6", line, col).expect("hover at the override call site");
    assert!(text.contains("proc ::ns::tcl::mathfunc::f"), "{text}");

    // Inside the same expression, completion offers the built-ins under their
    // bare names — the override is a proc and comes from the proc list.
    let labels = completion_labels(SWEEP, "tcl8.6", line, col);
    assert!(labels.contains(&"fmod".to_owned()), "{labels:?}");
}

/// TP: minify and the call graph — the remaining two sweep consumers — keep
/// the override's dispatch intact / visible.
#[test]
fn tp_minify_and_call_graph_handle_the_override() {
    let registry = registry_for_dialect("tcl8.6");
    let minified = tcl_lsp_core::minify::minify_tcl(
        SWEEP,
        tcl_dialect::DialectProfile::by_name("tcl8.6"),
        registry,
    );
    assert!(
        minified.contains("f(2)"),
        "the expr function-call production must survive minification: {minified}"
    );
    let graph = tcl_lsp_core::graphs::call_graph(
        SWEEP,
        registry,
        tcl_dialect::DialectProfile::by_name("tcl8.6"),
    );
    let text = graph.to_string();
    assert!(
        text.contains("tcl::mathfunc::f") || text.contains("::ns::use"),
        "the call graph must see the namespace: {text}"
    );
}

/// TN (idx 30's secondary false positive): a **bare** call to a proc that
/// lives only in a `tcl::mathfunc` namespace resolves to nothing — real Tcl
/// raises `invalid command name "li"` for it, so claiming a definition /
/// hover / reference there is a false positive on guaranteed-to-crash code.
#[test]
fn tn_a_bare_call_never_reaches_a_mathfunc_namespace_proc() {
    let src = "\
namespace eval tcl::mathfunc {
    proc li {list index args} { ::lindex $list $index {*}$args }
}
puts [li {10 20 30} 1]
";
    let analysis = analyse(src);
    let (line, col) = pos_of(src, "li {10 20 30}");
    assert!(
        tcl_lsp_core::definition::definition(src, line, col, &analysis).is_empty(),
        "bare `li` is not a command"
    );
    assert!(
        hover_at(src, "tcl8.6", line, col).is_none(),
        "bare `li` is not a command"
    );
    assert!(
        tcl_lsp_core::references::references(
            src,
            tcl_dialect::DialectProfile::by_name("tcl8.6"),
            line,
            col,
            &analysis,
            true
        )
        .is_empty(),
        "bare `li` is not a command"
    );
}

/// FP guard: an ordinary global proc of the same name is still reached by a
/// bare call — the mathfunc exclusion must not swallow real resolutions.
#[test]
fn fp_guard_a_real_global_proc_still_resolves_from_a_bare_call() {
    let src = "\
namespace eval tcl::mathfunc {
    proc li {list index args} { ::lindex $list $index {*}$args }
}
proc li {a b} { return DECOY }
puts [li 10 20]
";
    let analysis = analyse(src);
    let (line, col) = pos_of(src, "li 10 20");
    let locs = tcl_lsp_core::definition::definition(src, line, col, &analysis);
    assert_eq!(locs.len(), 1, "{locs:?}");
    assert_eq!(locs[0].start_line, 3, "the global decoy: {:?}", locs[0]);
}

// ---------------------------------------------------------------------------
// Issue #923 idx 103 — mathfunc as a first-class command
// ---------------------------------------------------------------------------

/// The registry data the finding asked for is present and dialect-gated, so
/// the generic per-dispatch-site availability check sees it: the qualified
/// *command* spelling exists from 8.5 (TIP 232) and `isinf` only from 9.0.
#[test]
fn mathfunc_command_spellings_are_registered_and_gated() {
    let reg = registry_for_dialect("tcl9.0");
    for name in ["tcl::mathfunc::isinf", "::tcl::mathfunc::isinf"] {
        assert!(
            reg.known_in_any_dialect(name),
            "{name} must have registry data"
        );
    }
    assert!(
        reg.get_for_dialect("::tcl::mathfunc::isinf", tcl_dialect::DialectSet::TCL86)
            .is_none(),
        "isinf is 9.0+"
    );
    assert!(
        reg.get_for_dialect("::tcl::mathfunc::sin", tcl_dialect::DialectSet::TCL84)
            .is_none(),
        "the command table itself is 8.5+ (TIP 232)"
    );
}

// ---------------------------------------------------------------------------
// Audit C4 — sigil-free / colon word recognition (idx 24, 48, 54, 104)
// ---------------------------------------------------------------------------

/// idx 48 (TP, regression pin): a variable's bare **declaring** token —
/// `cmd` in `foreach cmd $list`, and a `set` left-hand side.
#[test]
fn tp_idx48_bare_declaring_token_resolves() {
    let src = "set tracecmds {a b}\nforeach cmd $tracecmds {\n    puts $cmd\n}\n";
    let analysis = analyse(src);
    let (line, col) = pos_of(src, "cmd $tracecmds");
    let text = hover_at(src, "tcl8.6", line, col).expect("hover on the foreach loop variable");
    assert!(text.contains("**Variable** `cmd`"), "{text}");
    let locs = tcl_lsp_core::definition::definition(src, line, col, &analysis);
    assert_eq!(locs.len(), 1, "{locs:?}");
    assert_eq!(locs[0].start_line, line);

    let simple = "set y 10\nputs $y\nputs $y\n";
    let analysis = analyse(simple);
    assert!(
        hover_at(simple, "tcl8.6", 0, 4).is_some_and(|t| t.contains("**Variable** `y`")),
        "bare `y` in `set y 10`"
    );
    assert_eq!(
        tcl_lsp_core::definition::definition(simple, 0, 4, &analysis).len(),
        1
    );
}

/// idx 104 (TP): a proc parameter's own name resolves to the parameter, and
/// its **default-value literal** — pure data — resolves to nothing at all,
/// rather than to a same-named command.
#[test]
fn tp_idx104_parameter_list_words_are_not_command_references() {
    let src = "\
proc destroy {w} { puts $w }
proc rfg {grab focus {destroy destroy}} {
    if {$destroy eq \"withdraw\"} { return }
    destroy $grab
}
";
    let analysis = analyse(src);
    // The parameter's own name token.
    let (line, col) = pos_of(src, "destroy destroy}}");
    let text = hover_at(src, "tcl8.6", line, col).expect("the parameter itself");
    assert!(
        text.contains("**Variable** `destroy`"),
        "the parameter, not the command: {text}"
    );

    // The default-value literal, two words later — pure data.
    let default_col = col + 8;
    assert!(
        hover_at(src, "tcl8.6", line, default_col).is_none(),
        "a default-value literal is data, not a command reference"
    );
    assert!(
        tcl_lsp_core::definition::definition(src, line, default_col, &analysis).is_empty(),
        "a default-value literal is data, not a command reference"
    );

    // FP guard: the real command call in the body still resolves.
    let (bline, bcol) = pos_of(src, "destroy $grab");
    let locs = tcl_lsp_core::definition::definition(src, bline, bcol, &analysis);
    assert_eq!(locs.len(), 1, "the real call still resolves: {locs:?}");
    assert_eq!(locs[0].start_line, 0, "{:?}", locs[0]);
}

/// idx 54 (TP): `${ns}::setopt` — the literal fragment after a substitution
/// is the *name* `setopt`, not an absolute `::setopt`, so it resolves.
#[test]
fn tp_idx54_residual_name_after_a_substitution_resolves() {
    let src = "\
proc mytoolkit::setopt {a b} { return 1 }
proc callit {} {
    set ns \"mytoolkit\"
    ${ns}::setopt options a
}
";
    let analysis = analyse(src);
    let (line, col) = pos_of(src, "setopt options");
    // The shared word-bounding rule reports the name, with the span narrowed
    // past the `::` so a rename cannot eat the namespace separator.
    let (word, start, end) =
        tcl_lsp_core::hover::find_word_span_at_position(src, line, col).expect("a word");
    assert_eq!(word, "setopt", "residual `::` stripped");
    assert_eq!(end - start, 6, "span covers exactly the name");
    let locs = tcl_lsp_core::definition::definition(src, line, col, &analysis);
    assert_eq!(locs.len(), 1, "{locs:?}");
    assert_eq!(locs[0].start_line, 0, "{:?}", locs[0]);
    assert!(
        hover_at(src, "tcl8.6", line, col).is_some_and(|t| t.contains("proc ::mytoolkit::setopt")),
        "hover resolves too"
    );
}

/// idx 54 residual (TP): find-references from a proc's own declaration must
/// reach a `${ns}::name` call **inside that proc's own namespace**.
///
/// The walk records the local-first candidate (`::mypkg::mypkg::setdef` for
/// a `${ns}::setdef` head folded to `mypkg::setdef` while inside `::mypkg`), and the
/// post-walk settle recovers the calling namespace by stripping `::{name}`
/// off it — which cannot work when `name` is the *written* head
/// (`${ns}::setdef`) rather than the folded one.  The call therefore stayed
/// pinned to a name that exists nowhere, and find-references missed it while
/// go-to-definition (which re-resolves from the cursor) found it.
///
/// C Tcl ground truth: a qualified name resolves in the current namespace
/// first, then global, so `mypkg::setdef` called from inside `::mypkg`
/// reaches `::mypkg::setdef` — `::mypkg::mypkg::setdef` does not exist.
#[test]
fn tp_idx54_folded_head_settles_through_the_global_fallback() {
    let src = "\
namespace eval mypkg {}
proc mypkg::setdef {opts key args} { return 1 }
proc mypkg::literal {} {
    mypkg::setdef opts a
}
proc mypkg::folded {} {
    set ns \"mypkg\"
    ${ns}::setdef opts a
}
";
    let analysis = analyse(src);
    let (line, col) = pos_of(src, "setdef {opts key args}");
    let refs = tcl_lsp_core::references::references(
        src,
        tcl_dialect::DialectProfile::by_name("tcl8.6"),
        line,
        col,
        &analysis,
        true,
    );
    let mut lines: Vec<u32> = refs.iter().map(|r| r.start_line).collect();
    lines.sort_unstable();
    assert_eq!(
        lines,
        vec![1, 3, 7],
        "declaration + the literal call + the folded-head call: {refs:?}"
    );
}

/// idx 54 residual FP guard: the settle must not invent a target.  A folded
/// head naming a proc that exists in **neither** candidate namespace stays
/// unresolved — no reference is attributed to the same-tailed proc in an
/// unrelated namespace.
#[test]
fn fp_idx54_folded_head_does_not_borrow_an_unrelated_same_tail_proc() {
    let src = "\
namespace eval other {}
proc other::setdef {a} { return 1 }
namespace eval mypkg {}
proc mypkg::folded {} {
    set ns \"nowhere\"
    ${ns}::setdef opts a
}
";
    let analysis = analyse(src);
    let (line, col) = pos_of(src, "setdef {a}");
    let refs = tcl_lsp_core::references::references(
        src,
        tcl_dialect::DialectProfile::by_name("tcl8.6"),
        line,
        col,
        &analysis,
        true,
    );
    let mut lines: Vec<u32> = refs.iter().map(|r| r.start_line).collect();
    lines.sort_unstable();
    assert_eq!(lines, vec![1], "declaration only: {refs:?}");
}

/// idx 54 TN — the colon grammar, pinned against C Tcl: a **single** colon is
/// an ordinary name character, so it must never split a word.  Verified on
/// tclsh 8.6 / 9.0 (`set a:b 42`, `proc p:q {x}`, `set arr(k:1) v`,
/// `dict get $d x:y` all work), which is exactly why the fix is *not* "add
/// `:` to the delimiter set".
#[test]
fn tn_single_colons_never_split_a_word() {
    let src = "proc p:q {x} { return $x }\nputs [p:q 1]\n";
    let analysis = analyse(src);
    let (line, col) = pos_of(src, "p:q 1");
    let (word, _, _) =
        tcl_lsp_core::hover::find_word_span_at_position(src, line, col).expect("a word");
    assert_eq!(word, "p:q", "a lone colon is a name character");
    let locs = tcl_lsp_core::definition::definition(src, line, col, &analysis);
    assert_eq!(locs.len(), 1, "the colon-named proc resolves: {locs:?}");

    // An array index and a dict key containing a colon are single words too.
    let arr = "set arr(k:1) hello\nputs $arr(k:1)\n";
    let (line, col) = pos_of(arr, "k:1) hello");
    let (word, _, _) =
        tcl_lsp_core::hover::find_word_span_at_position(arr, line, col).expect("a word");
    assert_eq!(word, "arr(k:1)", "the whole element word, unsplit");
}

/// idx 24 (TP): a `$var`-shaped substring in a place Tcl never substitutes —
/// an inert comment, and a brace-quoted data word — must not resolve.
#[test]
fn tp_idx24_inert_dollar_refs_do_not_resolve() {
    let src = "\
proc holder {} {
    set level 42
    # performs NO substitution ... so $level ...
    set literalText {plain text with $level inside}
    return $literalText
}
";
    let analysis = analyse(src);
    for needle in ["$level ...", "$level inside"] {
        let (line, col) = pos_of(src, needle);
        assert!(
            hover_at(src, "tcl8.6", line, col + 1).is_none(),
            "{needle:?} is inert"
        );
        assert!(
            tcl_lsp_core::definition::definition(src, line, col + 1, &analysis).is_empty(),
            "{needle:?} is inert"
        );
        assert!(
            tcl_lsp_core::references::references(
                src,
                tcl_dialect::DialectProfile::by_name("tcl8.6"),
                line,
                col + 1,
                &analysis,
                true
            )
            .is_empty(),
            "{needle:?} is inert"
        );
        assert!(
            tcl_lsp_core::rename::prepare_rename(src, line, col + 1, &analysis).is_none(),
            "{needle:?} is not renameable"
        );
    }
}

/// idx 24 FP guard: every shape that *does* substitute still resolves —
/// quoted words, `if` / `expr` conditions, nested bodies, `[…]`
/// substitutions, concatenations, `${name}`, and `$arr(k)`.
#[test]
fn fp_guard_idx24_live_dollar_refs_still_resolve() {
    let src = "\
proc p {} {
    set level 42
    set arr(k) 1
    puts \"quoted $level\"
    if {$level > 1} { puts nested }
    set x [expr {$level * 2}]
    foreach i {1 2} { puts $level }
    puts [string length $level]
    puts ${level}
    puts $arr(k)
    puts $x$level
}
";
    let analysis = analyse(src);
    for needle in [
        "$level\"",
        "$level > 1",
        "$level * 2",
        "$level }",
        "$level]",
        "${level}",
        "$arr(k)\n",
    ] {
        let (line, col) = pos_of(src, needle);
        assert!(
            hover_at(src, "tcl8.6", line, col + 1).is_some_and(|t| t.contains("**Variable**")),
            "{needle:?} must still resolve"
        );
        assert!(
            !tcl_lsp_core::definition::definition(src, line, col + 1, &analysis).is_empty(),
            "{needle:?} must still resolve"
        );
    }
}

/// idx 24 FP guard: a qualified read (`$::ns::v`) is not recorded as a
/// reference on the namespace variable, so the inertness proofs must be
/// positive ones rather than "the analyser saw no read here".
#[test]
fn fp_guard_idx24_qualified_reads_still_resolve() {
    let src = "namespace eval ::simple {\n    variable v \"hello\"\n}\nputs $::simple::v\n";
    let analysis = analyse(src);
    let locs = tcl_lsp_core::definition::definition(src, 3, 16, &analysis);
    assert_eq!(locs.len(), 1, "{locs:?}");
    assert_eq!(locs[0].start_line, 1, "{:?}", locs[0]);
}

// ---------------------------------------------------------------------------
// Codex review of PR #1073 — the two conservative proofs, sharpened
// ---------------------------------------------------------------------------

/// Finding 1 (TP): a `{` **inside a bare word** is an ordinary character, so a
/// `#` after it is not in command position and the `$v` following it is a
/// genuine read that hover / definition / references must still resolve.
///
/// Oracle (tclsh 9.0.4 and 8.6.16 agree — `a{#` is one bare word whose brace
/// is literal, and `$v` is substituted):
///
/// ```text
/// set v OK
/// puts "1: [list a{# $v]"        -> 1: a\{# OK
/// puts "2: [string cat a{# $v]"  -> 2: a{#OK
/// ```
#[test]
fn tp_mid_word_brace_does_not_suppress_a_real_variable_read() {
    let src = "set v OK\nset w [string cat a{# $v]\nputs $w\n";
    let analysis = analyse(src);
    let (line, col) = pos_of(src, "$v]");
    let text = hover_at(src, "tcl8.6", line, col + 1)
        .expect("the read after a mid-word brace still hovers");
    assert!(text.contains("**Variable** `v`"), "{text}");
    assert_eq!(
        tcl_lsp_core::definition::definition(src, line, col + 1, &analysis).len(),
        1,
        "go-to-definition must still resolve"
    );
    assert!(
        !tcl_lsp_core::references::references(
            src,
            tcl_dialect::DialectProfile::by_name("tcl8.6"),
            line,
            col + 1,
            &analysis,
            true
        )
        .is_empty(),
        "find-references must still resolve"
    );
    assert!(
        tcl_lsp_core::rename::prepare_rename(src, line, col + 1, &analysis).is_some(),
        "the read is renameable"
    );
}

/// Finding 1 (TN): the existing command-position cases stay suppressed — a
/// leading `#`, a `#` after `;`, and a `#` just inside a **word-start** brace
/// (which is structural, unlike a mid-word one).
#[test]
fn tn_real_comment_positions_stay_suppressed() {
    for (src, needle) in [
        ("proc p {} {\n    set v 1\n    # so $v ...\n}\n", "$v ..."),
        ("set v 1\nputs hi ;# note $v\n", "$v"),
        ("set v 1\nputs {# not a comment $v}\n", "$v"),
    ] {
        let analysis = analyse(src);
        let (line, col) = pos_of(src, needle);
        assert!(
            hover_at(src, "tcl8.6", line, col + 1).is_none(),
            "{src:?} / {needle:?} must stay suppressed"
        );
        assert!(
            tcl_lsp_core::definition::definition(src, line, col + 1, &analysis).is_empty(),
            "{src:?} / {needle:?} must stay suppressed"
        );
    }
}

/// Finding 2 (TP): `proc`'s parameter-list argument is an ordinary Tcl word, so
/// it may be **computed** — and then it holds live code that must stay
/// navigable.
///
/// Oracle (tclsh 9.0.4 and 8.6.16 agree — `[makeargs]` runs at definition
/// time):
///
/// ```text
/// proc makeargs {} { return {a b} }
/// proc p [makeargs] { return "p got $a $b" }
/// puts [p 1 2]        -> p got 1 2
/// puts [info args p]  -> a b
/// ```
#[test]
fn tp_computed_parameter_list_stays_navigable() {
    let src = "\
proc makeargs {} { return {a b} }
proc p [makeargs] { return \"p got $a $b\" }
puts [p 1 2]
";
    let analysis = analyse(src);
    let (line, col) = pos_of(src, "makeargs] {");
    let locs = tcl_lsp_core::definition::definition(src, line, col, &analysis);
    assert_eq!(locs.len(), 1, "go-to-definition on `[makeargs]`: {locs:?}");
    assert_eq!(locs[0].start_line, 0, "{:?}", locs[0]);
    let text = hover_at(src, "tcl8.6", line, col).expect("hover on `[makeargs]`");
    assert!(text.contains("proc ::makeargs"), "{text}");
}

/// Finding 2 (TP): the `$var` and quoted forms of a computed parameter list are
/// live too — oracle: `set params {x y}; proc q $params {…}` and
/// `proc r "m n" {…}` both work on 9.0.4 and 8.6.16.
#[test]
fn tp_variable_parameter_list_stays_navigable() {
    let src = "\
set params {x y}
proc q $params { return \"q got $x $y\" }
";
    let analysis = analyse(src);
    let (line, col) = pos_of(src, "$params {");
    let text = hover_at(src, "tcl8.6", line, col + 1).expect("hover on `$params`");
    assert!(text.contains("**Variable** `params`"), "{text}");
    assert_eq!(
        tcl_lsp_core::definition::definition(src, line, col + 1, &analysis).len(),
        1,
        "go-to-definition on the parameter-list variable"
    );
}

/// Finding 2 (TN): a **braced literal** parameter list stays inert, so a
/// default-value word inside it draws no bogus command resolution — including
/// a nested `{b 1}` default pair whose `b` shares a name with a real command.
#[test]
fn tn_literal_parameter_list_stays_inert() {
    let src = "\
proc b {x} { return $x }
proc s {a {b 1} args} { return \"s got $a $b $args\" }
";
    // The parameter name `b` inside `{b 1}` resolves to the parameter, never
    // to the same-named global proc on line 0.
    let (line, col) = pos_of(src, "b 1} args");
    let text = hover_at(src, "tcl8.6", line, col).expect("the parameter itself");
    assert!(
        text.contains("**Variable** `b`"),
        "the parameter, not the proc: {text}"
    );
    // The default-value literal `1` is data.
    assert!(
        hover_at(src, "tcl8.6", line, col + 2).is_none(),
        "the default value is data"
    );
    // And a bareword default that names a real command still draws nothing.
    let shadow = "proc destroy {w} { puts $w }\nproc rfg {g {destroy destroy}} { destroy $g }\n";
    let shadow_analysis = analyse(shadow);
    let (line, col) = pos_of(shadow, "destroy}} {");
    assert!(
        hover_at(shadow, "tcl8.6", line, col).is_none(),
        "a default-value literal is not a command reference"
    );
    assert!(
        tcl_lsp_core::definition::definition(shadow, line, col, &shadow_analysis).is_empty(),
        "a default-value literal is not a command reference"
    );
}

// ---------------------------------------------------------------------------
// Issue #1079 — a computed parameter list declares nothing the analyser can
// model, so it registers no per-parameter `VarDef` and no arity
// ---------------------------------------------------------------------------
//
// Oracle (tclsh 9.0.4 and 8.6.16, identical):
//
//   proc makeargs {} { return {a b} }
//   proc p [makeargs] { return "p got $a $b" }
//   puts [info args p]  -> a b
//   puts [p 1 2]        -> p got 1 2
//
//   set params {x y}
//   proc q $params { return "q got $x $y" }
//   puts [q 1 2]        -> q got 1 2
//
// The analyser used to read the unresolved word as a *one-parameter literal*
// and register a `VarDef` whose name is the source text (`"[makeargs]"` /
// `"$params"`) spanning that word, plus a one-argument arity that made both
// two-argument calls above draw a false `E003 Too many arguments`.

/// TN: no `VarDef` is invented for a computed parameter-list word, in either
/// of its two spellings.
#[test]
fn tn_computed_parameter_list_registers_no_variable() {
    for (src, stub) in [
        (
            "proc makeargs {} { return {a b} }\nproc p [makeargs] { return \"$a$b\" }\n",
            "[makeargs]",
        ),
        (
            "set params {x y}\nproc q $params { return \"$x$y\" }\n",
            "$params",
        ),
    ] {
        let analysis = analyse(src);
        let names: Vec<&String> = analysis.all_variables.keys().collect();
        assert!(
            !names.iter().any(|n| n.contains(stub)),
            "no `VarDef` may be named after the computed word {stub:?}; got {names:?}"
        );
        // …and the proc itself carries no modelled parameters.
        let proc = analysis
            .all_procs
            .values()
            .find(|p| p.qualified_name == "::p" || p.qualified_name == "::q")
            .expect("the proc is still registered");
        assert!(
            proc.params.is_empty() && proc.params_computed,
            "the formals are unmodelled, not empty: {:?}",
            proc.params
        );
    }
}

/// TN: with the formals unknown, the call-site arity checker abstains — the
/// two-argument calls the oracle runs must draw no `E002`/`E003`/`E005`.
#[test]
fn tn_computed_parameter_list_abstains_from_arity_checking() {
    for src in [
        "proc makeargs {} { return {a b} }\nproc p [makeargs] { return \"$a$b\" }\nputs [p 1 2]\n",
        "set params {x y}\nproc q $params { return \"$x$y\" }\nq 1 2\n",
    ] {
        let codes: Vec<String> = analyse(src)
            .diagnostics
            .iter()
            .map(|d| d.code.to_string())
            .collect();
        assert!(
            !codes
                .iter()
                .any(|c| c == "E002" || c == "E003" || c == "E005"),
            "arity is unknowable for a computed parameter list; {src:?} emitted {codes:?}"
        );
    }
}

/// TP control: a *literal* parameter list — braced, bareword, or a quoted word
/// with no substitution — still models its formals and still checks arity.
/// tclsh 9.0.4 / 8.6.16: `proc s {a b} {…}; s 1 2 3` → `wrong # args`.
#[test]
fn tp_literal_parameter_lists_still_model_formals_and_arity() {
    let src = "proc s {a b} { return \"$a$b\" }\ns 1 2 3\n";
    let analysis = analyse(src);
    let proc = &analysis.all_procs["::s"];
    assert!(!proc.params_computed, "a braced list is literal");
    assert_eq!(proc.params.len(), 2, "{:?}", proc.params);
    let codes: Vec<String> = analysis
        .diagnostics
        .iter()
        .map(|d| d.code.to_string())
        .collect();
    assert!(codes.iter().any(|c| c == "E003"), "{codes:?}");
    // A quoted word with no substitution really does declare `m` and `n`
    // (tclsh: `proc r "m n" {…}; info args r` → `m n`).
    let quoted = analyse("proc r \"m n\" { return \"$m$n\" }\n");
    let r = &quoted.all_procs["::r"];
    assert!(
        !r.params_computed,
        "a substitution-free quoted list is literal"
    );
    assert_eq!(r.params.len(), 2, "{:?}", r.params);
}

// ---------------------------------------------------------------------------
// Issue #1054 — hover type inference must be built for the document's dialect
// ---------------------------------------------------------------------------

/// TP: an f5-iRules word operator (`contains`) inside an `if` condition is a
/// dialect-specific construct.  Under the iRules dialect the hover's inferred
/// intrep for a variable in the same script must still be produced — before
/// the fix the unit was lowered with the plain-Tcl lexer config and the
/// annotation skewed or vanished.
#[test]
fn tp_hover_type_inference_uses_the_documents_dialect() {
    let src = "\
when HTTP_REQUEST {
    set uri [HTTP::uri]
    set n 42
    if { $uri contains \"/api\" } {
        log local0. \"api $n\"
    }
}
";
    let (line, col) = pos_of(src, "$n\"");
    let text =
        hover_at(src, "f5-irules", line, col + 1).expect("hover on $n under the iRules dialect");
    assert!(text.contains("**Variable** `n`"), "{text}");
    assert!(
        text.contains("**Inferred intrep**: int"),
        "the dialect-lowered unit still infers the intrep: {text}"
    );
}

/// TN: plain-Tcl hover is unchanged by the threading.
#[test]
fn tn_plain_tcl_hover_type_inference_unchanged() {
    let src = "set n 42\nputs $n\n";
    let text = hover_at(src, "tcl8.6", 1, 6).expect("hover on $n");
    assert!(text.contains("**Variable** `n`"), "{text}");
    assert!(text.contains("**Inferred intrep**: int"), "{text}");
}
