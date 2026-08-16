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

//! Behaviour-driven coverage of several `tcl-lsp-core` editing / workspace
//! providers and utilities:
//!
//!   * `workspace_index::WorkspaceIndex::symbols_matching` — workspace
//!     symbol search over the indexed documents.
//!   * `minify::{minify_tcl, minify_tcl_compact, minify_tcl_aggressive, …}`
//!     — the Tcl minifier tiers.
//!   * `snippets::snippet_completions` — context-aware snippet templates.
//!   * `source_style::{check_*, style_diagnostics, …}` — the source-text
//!     style checks (W111/W112/W115/W118).
//!   * `linked_editing_range::linked_editing_ranges` — a small set of
//!     scenarios NOT already exercised by `lsp_providers.rs`
//!     (namespace-relative self-call linking; two mutually-recursive
//!     procs staying unlinked).
//!
//! These tests are derived from each provider's public API (see the matching
//! `src/*.rs`) and from Tcl semantics, with the Tcl-semantic facts pinned to
//! real C-Tcl (tclsh8.6 + tclsh9.0 via `scripts/dev/tclsh_check.sh`).
//!
//! C-Tcl proof model
//! -----------------
//! The load-bearing Tcl-semantic claim in this file is that **minification
//! preserves the program's result**. Every minify round-trip test below
//! takes a COMPLETE runnable Tcl script that `puts` a value, asserts the
//! minified form against the real minifier output, and the cited `// tclsh:`
//! line records that the original and the minified script compute the SAME
//! value under both tclsh8.6 and tclsh9.0. A workspace symbol that is
//! asserted to be a proc / class / method is likewise a genuinely-declared
//! symbol — the snippets that declare them are runnable and the
//! `info procs` / `info class …` facts are cited.
//!
//! Symbol / range / snippet STRUCTURE (which name, which kind, which line,
//! the snippet tabstop text, the style-diagnostic column) is
//! editor-presentation: `tclsh` has no opinion on it, so it is asserted
//! structurally. Those assertions are marked as structural where it matters.
//!
//! Verified C-Tcl facts (tclsh8.6 + tclsh9.0, identical output):
//!   * `proc fact {n} {…}; puts [fact 5]` -> 120, and its minified form
//!     `proc fact {n} {if {$n<=1} {return 1};return [expr …]};puts [fact 5]`
//!     -> 120 (comment stripped, whitespace collapsed, same value).
//!   * `set total 0; foreach x {1 2 3 4 5} {incr total $x}; puts $total`
//!     -> 15, minified `set total 0;foreach x {1 2 3 4 5} {incr total $x};
//!     puts $total` -> 15.
//!   * `set s "hello world"; puts [string toupper $s]; puts [expr {3+4*2}]`
//!     -> "HELLO WORLD" / 11, identical after minification.
//!   * compact-tier rename `fact`->`a` everywhere:
//!     `proc a {n} {…[a [expr {$n-1}]]…};puts [a 5]` -> 120 (the recursive
//!     self-call and the top-level call both bind to the one renamed proc).
//!   * clean proc-local compaction `proc add {a b} {set sum …;return $sum}`
//!     -> 5; renamed `proc a {a b} {set c …;return $c};puts [a 2 3]` -> 5.

use std::collections::{HashMap, HashSet};

use tcl_compiler::analyser::{Analyser, AnalysisResult};
use tcl_lsp_core::minify::{minify_tcl, minify_tcl_aggressive, minify_tcl_compact};
use tcl_lsp_core::snippets::SnippetContext;
use tcl_lsp_core::snippets::snippet_completions;
use tcl_lsp_core::source_style::{
    DEFAULT_LINE_ENDING, DEFAULT_LINE_LENGTH, StyleSeverity, check_comment_continuation,
    check_line_endings, check_line_length, check_trailing_whitespace, style_diagnostics,
};
use tcl_lsp_core::workspace_index::WorkspaceIndex;
use tcl_lsp_core::workspace_symbols::{
    IndexedWorkspaceSymbol, MAX_WORKSPACE_SYMBOL_RESULTS, WorkspaceSymbolKind,
};
use tcl_registry::{CommandRegistry, registry_for_dialect};

// ---------------------------------------------------------------------------
// shared helpers
// ---------------------------------------------------------------------------

fn analyse(source: &str) -> AnalysisResult {
    let mut a = Analyser::new();
    a.analyse(source, "tcl8.6").clone()
}

fn registry() -> &'static CommandRegistry {
    registry_for_dialect("tcl8.6")
}

/// Map symbol name -> symbol, for order-independent lookups.
fn by_name(syms: &[IndexedWorkspaceSymbol]) -> HashMap<&str, &IndexedWorkspaceSymbol> {
    syms.iter().map(|s| (s.name.as_str(), s)).collect()
}

/// `source` indexed as the workspace's only document, queried for `query`.
fn symbols(source: &str, query: &str) -> Vec<IndexedWorkspaceSymbol> {
    let analysis = analyse(source);
    let mut index = WorkspaceIndex::new();
    index.add_document("file:///doc.tcl", &analysis);
    index.symbols_matching(query, MAX_WORKSPACE_SYMBOL_RESULTS)
}

/// The zero-based line a symbol's name token starts on.  The index stores a
/// byte span; the server resolves it against the target document's source, and
/// so does this.
fn start_line(source: &str, sym: &IndexedWorkspaceSymbol) -> u32 {
    tcl_lexer::LineIndex::new(source)
        .position_at_utf16(sym.name_span.start(), source)
        .line
}

/// Empty disabled-set / suppression-map for the `style_diagnostics`
/// orchestrator (the generic `BuildHasher` slots take the std defaults).
fn no_disable() -> HashSet<String> {
    HashSet::new()
}
fn no_suppress() -> HashMap<i32, HashSet<String>> {
    HashMap::new()
}

// ===========================================================================
// workspace_symbols
//
// The provider lists procs / classes / methods / constructors from the
// analysed document, filtered by a case-insensitive SUBSTRING query
// (`matches_query` => `name.to_lowercase().contains(query)`).  Which symbols
// are genuine declarations is a Tcl fact (cited); kind / container / line is
// editor-presentation, asserted structurally.
// ===========================================================================

/// A document with a top-level proc, a namespaced proc, and a `TclOO` class
/// (with a method + a constructor). Reused by several symbol tests.
const SYMBOL_DOC: &str = "proc alpha {} { return 1 }\n\
                          namespace eval ::ns {\n\
                          proc beta {} { return 2 }\n\
                          }\n\
                          oo::class create Widget {\n\
                          method draw {} {}\n\
                          constructor {arg} {}\n\
                          }\n";

#[test]
fn workspace_symbols_empty_query_returns_every_declaration() {
    // tclsh: every name here is a real, separately-declared symbol —
    // `proc alpha …; namespace eval ::ns {proc beta …}` makes `info procs`
    // report `alpha` and `info procs ::ns::*` report `::ns::beta`;
    // `oo::class create Widget {method draw …}` makes
    // `info class methods Widget` report `draw`. An empty query matches all.
    let syms = symbols(SYMBOL_DOC, "");
    let map = by_name(&syms);
    // Structural: the full declared set surfaces.
    assert!(map.contains_key("alpha"), "{syms:?}");
    assert!(map.contains_key("beta"), "{syms:?}");
    assert!(map.contains_key("Widget"), "{syms:?}");
    assert!(map.contains_key("draw"), "{syms:?}");
    assert!(map.contains_key("constructor"), "{syms:?}");
}

#[test]
fn workspace_symbols_kinds_and_containers_are_structural() {
    let syms = symbols(SYMBOL_DOC, "");
    let map = by_name(&syms);
    // Kinds (editor-presentation, asserted structurally).
    assert_eq!(map["alpha"].kind, WorkspaceSymbolKind::Function);
    assert_eq!(map["beta"].kind, WorkspaceSymbolKind::Function);
    assert_eq!(map["Widget"].kind, WorkspaceSymbolKind::Class);
    assert_eq!(map["draw"].kind, WorkspaceSymbolKind::Method);
    assert_eq!(map["constructor"].kind, WorkspaceSymbolKind::Constructor);
    // Containers: the namespaced proc carries its namespace; the class
    // members carry the class's qualified name.
    assert_eq!(map["beta"].container_name.as_deref(), Some("::ns"));
    assert_eq!(map["draw"].container_name.as_deref(), Some("::Widget"));
    assert_eq!(
        map["constructor"].container_name.as_deref(),
        Some("::Widget")
    );
    // Every symbol names the document it was indexed from (structural).
    assert!(syms.iter().all(|s| s.uri == "file:///doc.tcl"), "{syms:?}");
    // The definition span points at the declaring line (structural).
    assert_eq!(start_line(SYMBOL_DOC, map["alpha"]), 0);
    assert_eq!(start_line(SYMBOL_DOC, map["draw"]), 5);
}

#[test]
fn workspace_symbols_query_is_case_insensitive_substring() {
    // Mixed-case substring of `alpha` matches exactly that proc.
    let syms = symbols(SYMBOL_DOC, "LPH");
    let names: Vec<&str> = syms.iter().map(|s| s.name.as_str()).collect();
    assert_eq!(names, vec!["alpha"], "{syms:?}");
}

#[test]
fn workspace_symbols_query_matches_class_member_by_substring() {
    // `draw` is reachable by a substring of its own name.
    let syms = symbols(SYMBOL_DOC, "dra");
    let names: Vec<&str> = syms.iter().map(|s| s.name.as_str()).collect();
    assert!(names.contains(&"draw"), "{syms:?}");
    // The unrelated top-level proc `alpha` must not match `dra`.
    assert!(!names.contains(&"alpha"), "{syms:?}");
}

#[test]
fn workspace_symbols_no_match_returns_empty() {
    assert!(
        symbols(SYMBOL_DOC, "zzz_nonexistent").is_empty(),
        "a query matching nothing yields no symbols",
    );
}

#[test]
fn workspace_symbols_empty_document_is_empty_and_does_not_panic() {
    assert!(symbols("", "").is_empty());
    assert!(symbols("", "anything").is_empty());
}

#[test]
fn workspace_symbols_classmethod_surfaces_as_method() {
    // tclsh: `oo::class create C { self method make {} {} }` (a class/`self`
    // method) is a real member — `info class methods C -all` lists it. The
    // provider surfaces a classmethod with kind Method (structural).
    let src = "oo::class create C {\n    classmethod make {} {}\n}\n";
    let syms = symbols(src, "make");
    assert_eq!(syms.len(), 1, "{syms:?}");
    assert_eq!(syms[0].name, "make");
    assert_eq!(syms[0].kind, WorkspaceSymbolKind::Method);
}

#[test]
fn workspace_symbols_span_every_indexed_document() {
    // Issue #1156: the answer is the whole workspace, not the caller's
    // document — two indexed documents both contribute, each naming its own
    // URI.
    let a = analyse("proc from_a {} {}\n");
    let b = analyse("proc from_b {} {}\n");
    let mut index = WorkspaceIndex::new();
    index.add_document("file:///a.tcl", &a);
    index.add_document("file:///b.tcl", &b);
    let syms = index.symbols_matching("from_", MAX_WORKSPACE_SYMBOL_RESULTS);
    let by_uri: HashMap<&str, &str> = syms
        .iter()
        .map(|s| (s.uri.as_str(), s.name.as_str()))
        .collect();
    assert_eq!(by_uri.get("file:///a.tcl"), Some(&"from_a"), "{syms:?}");
    assert_eq!(by_uri.get("file:///b.tcl"), Some(&"from_b"), "{syms:?}");
}

// ===========================================================================
// minify — default tier (`minify_tcl`)
//
// The default tier strips comments and collapses whitespace WITHOUT renaming
// anything, so the minified program is byte-for-byte semantically identical.
// Each test pins the exact minifier output AND records the tclsh proof that
// original and minified compute the same value.
// ===========================================================================

#[test]
fn minify_strips_comments_and_collapses_whitespace_factorial() {
    // tclsh (8.6 + 9.0): the ORIGINAL `proc fact {n} { # base case \n
    // if {$n<=1} {return 1}; return [expr {$n*[fact [expr {$n-1}]]}] };
    // puts [fact 5]` -> 120, and the MINIFIED form below -> 120 (same value).
    let src = "proc fact {n} {\n\
               # base case\n\
               if {$n <= 1} { return 1 }\n\
               return [expr {$n * [fact [expr {$n - 1}]]}]\n\
               }\n\
               puts [fact 5]\n";
    let out = minify_tcl(src, "tcl8.6", registry());
    // Exact minified form (the # comment line is gone; spaces collapsed;
    // inter-command newlines became `;`).
    assert_eq!(
        out,
        "proc fact {n} {if {$n<=1} {return 1};return [expr {$n*[fact [expr {$n-1}]]}]};puts [fact 5]",
    );
    // Semantics-preserving structural invariants of a successful minify:
    assert!(!out.contains('#'), "comment must be stripped: {out:?}");
    assert!(!out.contains('\n'), "newlines collapsed: {out:?}");
    assert!(out.len() < src.len(), "minified is shorter");
}

#[test]
fn minify_preserves_foreach_accumulator_result() {
    // tclsh (8.6 + 9.0): original -> 15, minified -> 15.
    let src = "set total 0\n\
               # accumulate\n\
               foreach x {1 2 3 4 5} {\n\
               incr total $x\n\
               }\n\
               puts $total\n";
    let out = minify_tcl(src, "tcl8.6", registry());
    assert_eq!(
        out,
        "set total 0;foreach x {1 2 3 4 5} {incr total $x};puts $total"
    );
}

#[test]
fn minify_preserves_string_and_expr_results() {
    // tclsh (8.6 + 9.0): original -> "HELLO WORLD" / 11; minified -> same.
    let src = "set s \"hello world\"\n\
               puts [string toupper $s]\n\
               puts [expr {3 + 4 * 2}]\n";
    let out = minify_tcl(src, "tcl8.6", registry());
    assert_eq!(
        out,
        "set s \"hello world\";puts [string toupper $s];puts [expr {3+4*2}]"
    );
    // The quoted literal is preserved verbatim (no semantic change).
    assert!(out.contains("\"hello world\""), "{out:?}");
}

#[test]
fn minify_comment_only_and_empty_collapse_to_empty() {
    // A file of only comments / whitespace minifies to nothing — there is no
    // command to run, so the (empty) program is trivially equivalent.
    assert_eq!(minify_tcl("", "tcl8.6", registry()), "");
    assert_eq!(
        minify_tcl("# just a comment\n# another\n", "tcl8.6", registry()),
        "",
    );
}

#[test]
fn minify_is_idempotent_on_already_minified_source() {
    // Minifying twice changes nothing further — the result is a fixpoint, so
    // re-running the minifier cannot drift the program's meaning.
    let src = "proc fact {n} {\n    if {$n <= 1} { return 1 }\n    return [expr {$n * [fact [expr {$n - 1}]]}]\n}\nputs [fact 5]\n";
    let once = minify_tcl(src, "tcl8.6", registry());
    let twice = minify_tcl(&once, "tcl8.6", registry());
    assert_eq!(once, twice, "minify must be idempotent");
}

// ===========================================================================
// minify — compact tier (`minify_tcl_compact`) — the clean rename case
//
// The compact tier renames proc-local vars / params / proc names. The
// rename is correct exactly when the renamed script still resolves the same
// way Tcl does, which is the cited tclsh proof.
// ===========================================================================

#[test]
fn minify_compact_isolated_renames_proc_everywhere_and_preserves_result() {
    // tclsh (8.6 + 9.0): original `proc fact {n} {…}; puts [fact 5]` -> 120.
    // Under `isolated` (proc names are public identities in the non-isolated
    // tier — issue #1193) the compact tier renames `fact`->`a` at the
    // DECLARATION, the RECURSIVE self-call, and the top-level call. The
    // renamed program
    // `proc a {n} {if {$n<=1} {return 1};return [expr {$n*[a [expr {$n-1}]]}]};
    // puts [a 5]` -> 120 (verified identical) — all three sites bind to the
    // one renamed proc, so the value is unchanged.
    let src = "proc fact {n} {\n    if {$n <= 1} { return 1 }\n    return [expr {$n * [fact [expr {$n - 1}]]}]\n}\nputs [fact 5]\n";
    let (out, map) = minify_tcl_compact(src, "tcl8.6", true, registry());
    assert_eq!(
        out,
        "proc a {n} {if {$n<=1} {return 1};return [expr {$n*[a [expr {$n-1}]]}]};puts [a 5]",
    );
    // The symbol map records the rename so an error referring to `a` can be
    // un-minified back to `fact`.
    assert_eq!(map.procs.get("fact").map(String::as_str), Some("a"));
    // `fact` no longer appears anywhere (decl + both call sites rewritten).
    assert!(!out.contains("fact"), "all `fact` sites renamed: {out:?}");
}

#[test]
fn minify_compact_non_isolated_keeps_proc_names() {
    // Issue #1193: without `isolated`, a proc name is a PUBLIC command
    // identity — external callers, `info procs`, `rename`, and `unknown`
    // can observe or invoke it — so it must survive verbatim.
    let src = "proc fact {n} {\n    if {$n <= 1} { return 1 }\n    return [expr {$n * [fact [expr {$n - 1}]]}]\n}\nputs [fact 5]\n";
    let (out, map) = minify_tcl_compact(src, "tcl8.6", false, registry());
    assert!(out.contains("proc fact"), "{out:?}");
    assert!(out.contains("[fact 5]"), "{out:?}");
    assert!(map.procs.is_empty(), "{:?}", map.procs);
}

#[test]
fn minify_compact_renames_local_var_at_def_and_read() {
    // tclsh (8.6 + 9.0): `proc add {a b} {set sum [expr {$a+$b}];return $sum};
    // puts [add 2 3]` -> 5; the compact rename
    // `proc a {a b} {set c [expr {$a+$b}];return $c};puts [a 2 3]` -> 5
    // (verified identical). `sum`->`c` is rewritten at BOTH the `set`
    // definition and the `$sum` read, so the value is preserved. (The
    // params `a`/`b` are already 1 char, so they are left alone.)
    let src =
        "proc add {a b} {\n    set sum [expr {$a + $b}]\n    return $sum\n}\nputs [add 2 3]\n";
    let (out, map) = minify_tcl_compact(src, "tcl8.6", true, registry());
    assert_eq!(
        out,
        "proc a {a b} {set c [expr {$a+$b}];return $c};puts [a 2 3]",
    );
    // proc renamed `add`->`a`, local `sum`->`c` recorded under the proc scope.
    assert_eq!(map.procs.get("add").map(String::as_str), Some("a"));
    assert!(
        map.variables
            .values()
            .any(|m| m.get("sum").map(String::as_str) == Some("c")),
        "sum->c must be in the symbol map: {map:?}",
    );
    // The `$sum` read became `$c`; no stray `sum` survives.
    assert!(out.contains("$c"), "{out:?}");
    assert!(!out.contains("sum"), "{out:?}");
}

#[test]
fn minify_compact_short_names_are_left_alone() {
    // Single-char names (`x`) are never compacted (no shorter name exists),
    // so a script with only short locals minifies but renames nothing — the
    // symbol map's variable section is empty.
    let src = "proc f {} {\n    set x 1\n    return $x\n}\n";
    let (_out, map) = minify_tcl_compact(src, "tcl8.6", false, registry());
    assert!(
        map.variables.values().all(|m| !m.contains_key("x")),
        "single-char `x` must not be renamed: {map:?}",
    );
}

// ===========================================================================
// minify — aggressive tier (`minify_tcl_aggressive`) — result metadata only
//
// We assert the MinifyResult bookkeeping (length / savings) rather than the
// exact byte output, because the aggressive tier's varname-compaction has a
// known correctness bug (see the `// BUG:` block below) that we must not
// depend on. The savings arithmetic is pure and unaffected.
// ===========================================================================

#[test]
fn minify_aggressive_reports_consistent_length_bookkeeping() {
    let src = "set total 0\nputs $total\nputs $total\nputs $total\n";
    let res = minify_tcl_aggressive(src, "tcl8.6", true, registry());
    // original_length is the input byte length; minified_length matches the
    // produced source; savings is the derived percentage. These are pure and
    // independent of the rename bug.
    assert_eq!(res.original_length, src.len());
    assert_eq!(res.minified_length(), res.source.len());
    // Lengths are small known values; convert losslessly via u32.
    let min_len = f64::from(u32::try_from(res.source.len()).unwrap());
    let orig_len = f64::from(u32::try_from(src.len()).unwrap());
    let expected_pct = (1.0 - min_len / orig_len) * 100.0;
    assert!(
        (res.savings_pct() - expected_pct).abs() < 1e-9,
        "savings_pct must equal 1 - min/orig: got {} want {expected_pct}",
        res.savings_pct(),
    );
}

#[test]
fn minify_aggressive_empty_source_has_zero_savings() {
    let res = minify_tcl_aggressive("", "tcl8.6", true, registry());
    assert_eq!(res.original_length, 0);
    // The 0/0 guard returns an exact 0.0 (not NaN); compare bit patterns.
    assert_eq!(
        res.savings_pct().to_bits(),
        0.0_f64.to_bits(),
        "0/0 guard returns 0.0, not NaN"
    );
}

// --- BUG: compact/aggressive minify corrupts `incr`/`append`/`lappend` -----
//
// The compact-name tier renames a variable at its `set` definition and at
// every `$var` READ, but NOT when the variable name appears as the bare
// (un-`$`-prefixed) WRITE-TARGET argument of `incr` / `append` / `lappend`.
// The renamed program then mutates a *different* variable than the one it
// reads, changing the result.
//
// Minimal repro (isolated aggressive, but the plain `minify_tcl_compact`
// tier produces the identical broken output):
//
//   input:    set total 0\nincr total 5\nputs $total\n
//   minified: set a 0;incr total 5;puts $a
//
//   tclsh8.6 + tclsh9.0:  original `set total 0;incr total 5;puts $total` -> 5
//                         minified `set a 0;incr total 5;puts $a`         -> 0
//
// (Cross-checked: `append` corrupts the same way — original
// `proc build {} {set acc "";append acc x;append acc y;return $acc};
// puts [build]` -> "xy"; minified `proc a {} {set a "";append acc x;
// append acc y;return $a};puts [a]` -> "" on both tclsh versions.)
//
// FIXED: name compaction now excludes any variable that is the bare
// write-target of a mutating command (incr/append/lappend), since the
// analyser records those as definitions (not reads) and so VarDef.references
// misses them. The variable keeps its original name everywhere — correct
// (semantics-preserving) at the cost of not compacting that one name.
#[test]
fn minify_compact_preserves_incr_accumulator() {
    let src = "set total 0\nincr total 5\nputs $total\n";
    let (out, _map) = minify_tcl_compact(src, "tcl8.6", true, registry());
    // The `incr` target and the `set`/`$` sites must agree: either all renamed
    // to the same short name, or all left as `total`. With the exclude fix
    // `total` is kept, so the minified script still prints 5 in tclsh.
    assert!(
        !out.contains("incr total ") || out.contains("set total "),
        "incr write-target left un-renamed: {out:?}",
    );
}

#[test]
fn minify_compact_preserves_append_accumulator() {
    // append cross-check: `acc` is an append target, so it stays `acc`
    // everywhere; the minified proc still returns "xy" in tclsh.
    let src = "proc build {} { set acc \"\"\nappend acc x\nappend acc y\nreturn $acc }\n";
    let (out, _map) = minify_tcl_compact(src, "tcl8.6", true, registry());
    assert!(
        !out.contains("append acc ") || out.contains("set acc "),
        "append write-target left un-renamed: {out:?}",
    );
}

// ===========================================================================
// snippets — context-aware completion templates (structural)
//
// Snippet bodies / labels / filtering are editor-presentation; tclsh has no
// opinion, so these are asserted structurally.
// ===========================================================================

fn tcl_ctx<'a>(partial: &'a str, vars: &'a [String]) -> SnippetContext<'a> {
    SnippetContext {
        profile: tcl_dialect::DialectProfile::by_name("tcl8.6"),
        indent_unit: "    ",
        scope_vars: vars,
        partial,
        current_event: None,
        file_events: &[],
    }
}

#[test]
fn snippets_emit_only_snippet_kind_items() {
    let items = snippet_completions(&tcl_ctx("", &[]));
    assert!(!items.is_empty(), "the tcl dialect offers core templates");
    assert!(
        items
            .iter()
            .all(|i| i.is_snippet && i.kind == tcl_lsp_core::completion::CompletionKind::Snippet),
        "every item is a Snippet-kind snippet",
    );
    // Each carries a `tcl-…` filter prefix and a non-empty body.
    assert!(items.iter().all(|i| {
        i.filter_text
            .as_deref()
            .is_some_and(|f| f.starts_with("tcl-"))
    }));
    assert!(items.iter().all(|i| !i.insert_text.is_empty()));
}

#[test]
fn snippets_prefix_filters_to_matching_templates() {
    // `tcl-fo` matches exactly the foreach + for templates (prefix match).
    let items = snippet_completions(&tcl_ctx("tcl-fo", &[]));
    let labels: Vec<&str> = items.iter().map(|i| i.label.as_str()).collect();
    assert_eq!(labels, vec!["Foreach", "For Loop"], "{labels:?}");
}

#[test]
fn snippets_proc_body_uses_indent_unit_and_tabstops() {
    let items = snippet_completions(&tcl_ctx("tcl-proc", &[]));
    assert_eq!(items.len(), 1, "{items:?}");
    // Structural: the body is the canonical VS Code snippet with the
    // four-space indent unit threaded in.
    assert_eq!(
        items[0].insert_text,
        "proc ${1:name} {${2:args}} {\n    $0\n}",
    );
    assert_eq!(items[0].filter_text.as_deref(), Some("tcl-proc"));
}

#[test]
fn snippets_foreach_offers_in_scope_vars_as_choices() {
    let vars = vec!["items".to_string(), "list".to_string()];
    let items = snippet_completions(&tcl_ctx("tcl-foreach", &vars));
    assert_eq!(items.len(), 1, "{items:?}");
    // The list placeholder becomes a `${2|...|}` choice of the in-scope vars.
    assert!(
        items[0].insert_text.contains("${2|\\$items,\\$list|}"),
        "expected a choice placeholder of scope vars: {:?}",
        items[0].insert_text,
    );
}

#[test]
fn snippets_irules_templates_hidden_in_plain_tcl_dialect() {
    // The `irule-…` event templates are `f5-irules`-only; an `irule` prefix
    // in a tcl8.6 context yields nothing.
    let items = snippet_completions(&tcl_ctx("irule", &[]));
    assert!(items.is_empty(), "{items:?}");
}

#[test]
fn snippets_irules_event_templates_offered_in_irules_dialect() {
    let events: Vec<String> = Vec::new();
    let ctx = SnippetContext {
        profile: tcl_dialect::DialectProfile::irules(),
        indent_unit: "    ",
        scope_vars: &[],
        partial: "irule",
        current_event: None,
        file_events: &events,
    };
    let items = snippet_completions(&ctx);
    let labels: Vec<&str> = items.iter().map(|i| i.label.as_str()).collect();
    // Structural: every iRules event template is offered at the top level
    // when none are yet declared.
    assert!(labels.contains(&"iRule RULE_INIT"), "{labels:?}");
    assert!(labels.contains(&"iRule HTTP_REQUEST"), "{labels:?}");
}

#[test]
fn snippets_irules_event_template_declines_when_event_present() {
    // A `when RULE_INIT` already declared in the file => the RULE_INIT
    // template declines (offers nothing for that prefix).
    let events = vec!["RULE_INIT".to_string()];
    let ctx = SnippetContext {
        profile: tcl_dialect::DialectProfile::irules(),
        indent_unit: "    ",
        scope_vars: &[],
        partial: "irule-rule-init",
        current_event: None,
        file_events: &events,
    };
    assert!(
        snippet_completions(&ctx).is_empty(),
        "RULE_INIT template must decline when its event is already declared",
    );
}

// ===========================================================================
// source_style — the source-text style checks (W111/W112/W115/W118)
//
// These read raw text columns; structural assertions throughout.
// ===========================================================================

#[test]
fn style_w111_flags_overlong_line_only_past_the_limit() {
    // 121 chars > 120 fires; the boundary case 120 stays clean.
    let long = "x".repeat(121);
    let diags = check_line_length(&long, DEFAULT_LINE_LENGTH);
    assert_eq!(diags.len(), 1, "{diags:?}");
    assert_eq!(diags[0].code, "W111");
    assert_eq!(diags[0].severity, StyleSeverity::Warning);
    // end_character is end-exclusive (one past the last covered column): a
    // 121-char line covers columns 0..=120, so end is 121 (issue 186).
    assert_eq!(diags[0].range.end_character, 121);
    assert!(diags[0].fix.is_none(), "W111 has no quick-fix");

    let boundary = "z".repeat(DEFAULT_LINE_LENGTH);
    assert!(
        check_line_length(&boundary, DEFAULT_LINE_LENGTH).is_empty(),
        "exactly max length is clean",
    );
}

#[test]
fn style_w112_flags_trailing_whitespace_with_remove_fix() {
    let src = "set x 1   \nset y 2";
    let diags = check_trailing_whitespace(src);
    assert_eq!(diags.len(), 1, "{diags:?}");
    let d = &diags[0];
    assert_eq!(d.code, "W112");
    assert_eq!(d.severity, StyleSeverity::Hint);
    // The flagged span covers exactly the trailing run: columns 7, 8, 9
    // as an end-exclusive range 7..10 (issue 186).
    assert_eq!(d.range.start_character, 7);
    assert_eq!(d.range.end_character, 10);
    let fix = d
        .fix
        .as_ref()
        .expect("W112 carries a remove-whitespace fix");
    assert_eq!(fix.new_text, "");
    assert_eq!(fix.description, "Remove trailing whitespace");
}

#[test]
fn style_w112_does_not_flag_clean_lines_or_crlf() {
    assert!(check_trailing_whitespace("set x 1\nset y 2\n").is_empty());
    // A bare CRLF terminator is not itself trailing whitespace.
    assert!(check_trailing_whitespace("set x 1\r\nset y 2\r\n").is_empty());
}

#[test]
fn style_w118_flags_unexpected_line_endings() {
    let diags = check_line_endings("a\r\nb\r\n", "\n");
    assert_eq!(diags.len(), 1, "{diags:?}");
    assert_eq!(diags[0].code, "W118");
    assert_eq!(
        diags[0].message,
        "File uses CRLF line endings (2); expected LF"
    );
    // A file that already uses the expected ending is clean.
    assert!(check_line_endings("a\nb\nc\n", "\n").is_empty());
}

#[test]
fn style_w115_flags_comment_continuation_with_perline_fix() {
    // tclsh-relevant hazard: a `\` at the end of a `#` comment line swallows
    // the NEXT line into the comment, hiding live code. (Structural here —
    // we assert the diagnostic + the convert-to-per-line fix text.)
    let src = "# first line \\\nsecond line\nputs ok";
    let diags = check_comment_continuation(src);
    assert_eq!(diags.len(), 1, "{diags:?}");
    let d = &diags[0];
    assert_eq!(d.code, "W115");
    assert_eq!(d.range.start_line, 0);
    assert_eq!(d.range.end_line, 1);
    let fix = d.fix.as_ref().expect("W115 carries a per-line-comment fix");
    assert_eq!(fix.new_text, "# first line\n# second line");
    assert_eq!(fix.description, "Convert to per-line comments");

    // A plain comment with no backslash continuation is clean.
    assert!(check_comment_continuation("# plain\nputs hi").is_empty());
}

#[test]
fn style_orchestrator_merges_checks_and_respects_disabled_set() {
    // A document with a long line AND trailing whitespace produces both
    // codes; disabling W112 drops only that one.
    let src = format!("{}   \nset y 2", "q".repeat(125));
    let all = style_diagnostics(
        &src,
        DEFAULT_LINE_LENGTH,
        DEFAULT_LINE_ENDING,
        &no_disable(),
        &no_suppress(),
        None,
        "tcl9.0",
    );
    let codes: HashSet<&str> = all.iter().map(|d| d.code).collect();
    assert!(codes.contains("W111"), "{all:?}");
    assert!(codes.contains("W112"), "{all:?}");

    let mut disabled = no_disable();
    disabled.insert("W112".to_string());
    let filtered = style_diagnostics(
        &src,
        DEFAULT_LINE_LENGTH,
        DEFAULT_LINE_ENDING,
        &disabled,
        &no_suppress(),
        None,
        "tcl9.0",
    );
    assert!(filtered.iter().all(|d| d.code != "W112"), "{filtered:?}");
    assert!(
        filtered.iter().any(|d| d.code == "W111"),
        "W111 still fires"
    );
}

#[test]
fn style_orchestrator_honours_line_suppression_for_line_codes() {
    // A `*` suppression recorded against line 0 hides the line-0 W112, but a
    // file-level W118 (not line-suppressible) still fires.
    let mut suppressed = no_suppress();
    suppressed.insert(0, std::iter::once("*".to_string()).collect());
    let diags = style_diagnostics(
        "set x 1   \r\n",
        DEFAULT_LINE_LENGTH,
        DEFAULT_LINE_ENDING,
        &no_disable(),
        &suppressed,
        None,
        "tcl9.0",
    );
    assert!(
        diags.iter().all(|d| d.code != "W112"),
        "line W112 suppressed: {diags:?}"
    );
    assert!(
        diags.iter().any(|d| d.code == "W118"),
        "file-level W118 not line-suppressed: {diags:?}"
    );
}

// ===========================================================================
// linked_editing_range — only scenarios NOT covered by lsp_providers.rs
//
// (That file already covers: recursive self-call linking, cursor-inside-body,
// non-recursive => None, builtin => None, degenerate inputs => None.)
// Here we add: a namespace-relative self-call resolved via
// resolved_qualified_name, and two mutually-recursive procs staying UNLINKED.
//
// The linked-range SET is editor-presentation (asserted structurally); the
// proof it's RIGHT is that the linked sites are real self-invocations Tcl
// binds to the one proc, and the sites it keeps apart are calls Tcl resolves
// to a *different* proc.
// ===========================================================================

use tcl_lsp_core::linked_editing_range::{WORD_PATTERN, linked_editing_ranges};

#[test]
fn linked_editing_links_namespace_relative_self_call() {
    // tclsh (8.6 + 9.0): a proc in `::ns` that calls itself by its SHORT name
    // from inside the namespace is a real recursion —
    // `namespace eval ::ns { proc countdown {n} { if {$n<=0} {return done};
    //  return [countdown [expr {$n-1}]] } }; puts [::ns::countdown 2]`
    // -> done. The short `countdown` self-call resolves to `::ns::countdown`,
    // so it links to the declaration.
    let src = "namespace eval ::ns {\n\
               proc countdown {n} {\n\
               if {$n <= 0} { return done }\n\
               return [countdown [expr {$n - 1}]]\n\
               }\n\
               }\n";
    let an = analyse(src);
    // Cursor on the `countdown` declaration name (line 1).
    let line1 = "proc countdown {n} {";
    let col = u32::try_from(line1.find("countdown").expect("name present")).unwrap();
    let result = linked_editing_ranges(src, 1, col, &an)
        .expect("namespace-relative self-call should link to the declaration");
    // Structural: at least the declaration + the relative self-call.
    assert!(result.ranges.len() >= 2, "{result:?}");
    assert_eq!(result.word_pattern, WORD_PATTERN);
}

#[test]
fn linked_editing_does_not_link_across_two_mutually_recursive_procs() {
    // tclsh (8.6 + 9.0): `is_even`/`is_odd` are two DISTINCT procs that call
    // each other — `proc is_even {n} {…[is_odd …]…}; proc is_odd {n}
    // {…[is_even …]…}; puts [is_even 4]` -> 1. Cursor on `is_even`'s
    // declaration must link ONLY `is_even`'s own sites (it has no self-call,
    // only a call TO is_odd), never `is_odd`'s — so the result is `None`
    // (a lone declaration range can't be linked-edited).
    let src = "proc is_even {n} {\n\
               if {$n == 0} { return 1 }\n\
               return [is_odd [expr {$n - 1}]]\n\
               }\n\
               proc is_odd {n} {\n\
               if {$n == 0} { return 0 }\n\
               return [is_even [expr {$n - 1}]]\n\
               }\n";
    let an = analyse(src);
    // Cursor on `is_even`'s declaration name (line 0).
    let col = u32::try_from("proc ".len()).unwrap();
    // `is_even` is only *called* from is_odd's body (a cross-proc call, not a
    // self-call), so from its own declaration there is no second linkable
    // range -> None. This proves the provider does not mistake a mutual call
    // for a self-call.
    assert!(
        linked_editing_ranges(src, 0, col, &an).is_none(),
        "mutual recursion must not yield linked ranges from is_even's decl",
    );
}
