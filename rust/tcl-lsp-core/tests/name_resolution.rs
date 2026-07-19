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

//! TP/FP/TN/FN matrix for issues #954-#958 (Rust LSP), unit level.
//!
//! Each issue is exercised across the confusion matrix:
//!   * **TP** — the feature fires where it should (the call *is* a reference,
//!     the token *is* a parameter, the read *is* suppressed).
//!   * **FP guard** — a look-alike that must *not* fire (a different class /
//!     method / function, a non-`pkgIndex` file, a `$lambda` variable).
//!   * **TN** — nothing relevant present, nothing emitted.
//!   * **FN** (the regression) — the exact shape each issue reported, which
//!     used to be missed and now resolves.
//!
//! `references`/`semantic_tokens` come from `tcl-lsp-core`; W210 diagnostics
//! from the `tcl-compiler` analyser directly.

#![allow(clippy::cast_possible_truncation)]

use tcl_compiler::analyser::{Analyser, AnalysisResult};
use tcl_compiler::compiler_checks::DiagCode;
use tcl_lsp_core::definition::LspRange;
use tcl_lsp_core::references::references;
use tcl_lsp_core::semantic_tokens::{full, legend_token_types};
use tcl_registry::registry_for_dialect;

fn analyse(source: &str) -> AnalysisResult {
    Analyser::new().analyse(source, "tcl8.6").clone()
}

/// Sorted, de-duplicated 0-based line numbers of a reference result.
fn ref_lines(ranges: &[LspRange]) -> Vec<u32> {
    let mut v: Vec<u32> = ranges.iter().map(|r| r.start_line).collect();
    v.sort_unstable();
    v.dedup();
    v
}

/// Reference lines for the cursor at (`line`, `col`), declaration included.
fn refs_at(src: &str, line: u32, col: u32) -> Vec<u32> {
    let analysis = analyse(src);
    ref_lines(&references(src, "tcl", line, col, &analysis, true))
}

/// W210 (read-before-set) diagnostic codes for `src` analysed as `path`.
fn has_w210(src: &str, path: Option<&str>) -> bool {
    let mut a = Analyser::new().with_file_path(path.map(str::to_string));
    a.analyse(src, "tcl8.6")
        .diagnostics
        .iter()
        .any(|d| d.code == DiagCode::W210)
}

/// The semantic-token *type name* covering the first byte of the first
/// **whole-word** occurrence of `needle` in `src` (via the public legend),
/// or `None`.  Word-bounded so a needle like `a` matches the standalone
/// parameter, not the `a` inside `apply`.
fn kind_of(src: &str, dialect: &str, needle: &str) -> Option<String> {
    let is_word = |b: u8| b.is_ascii_alphanumeric() || b == b'_' || b == b':';
    let bytes = src.as_bytes();
    let byte = (0..=src.len().saturating_sub(needle.len())).find(|&p| {
        src[p..].starts_with(needle)
            && (p == 0 || !is_word(bytes[p - 1]))
            && (p + needle.len() == src.len() || !is_word(bytes[p + needle.len()]))
    })?;
    let before = &src[..byte];
    let line = before.matches('\n').count() as u32;
    let col = (byte - before.rfind('\n').map_or(0, |n| n + 1)) as u32;
    let registry = registry_for_dialect(dialect);
    let st = full(src, dialect, registry);
    let legend = legend_token_types();
    let (mut l, mut c) = (0u32, 0u32);
    for chunk in st.data.chunks(5) {
        let (dl, dc, len, ty) = (chunk[0], chunk[1], chunk[2], chunk[3]);
        if dl > 0 {
            l += dl;
            c = dc;
        } else {
            c += dc;
        }
        if l == line && c <= col && col < c + len {
            return Some(legend[ty as usize].to_string());
        }
    }
    None
}

// ───────────────────────── #956 — `$obj method` references ────────────────

mod obj_method_dispatch {
    use super::*;

    /// FN→TP: the reported shape — `$b get` dispatches to `Bar956::get`, so the
    /// call site is a reference to the method declaration.
    #[test]
    fn tp_obj_dispatch_is_a_reference() {
        let src = "oo::class create Bar956 {\n    method get {key} { return $key }\n}\nset b [Bar956 new]\nputs [$b get foo]\n";
        // cursor on the `get` declaration name (line 1, col 11).
        assert_eq!(refs_at(src, 1, 11), vec![1, 4], "decl + $b dispatch");
    }

    /// TP: `CLASS create NAME` binds an object *command*; the bare `NAME method`
    /// dispatch is a reference too.
    #[test]
    fn tp_object_command_dispatch_is_a_reference() {
        let src = "oo::class create Dog {\n    method bark {} { return woof }\n}\nDog create rex\nrex bark\n";
        assert_eq!(refs_at(src, 1, 11), vec![1, 4], "decl + `rex bark`");
    }

    /// TP: `$b get` embedded in a quoted / compound word
    /// (`"result: [$b get foo]"`) is still a reference — the merged word's
    /// substitution is recovered by re-lexing its slice.
    #[test]
    fn tp_obj_dispatch_in_quoted_word() {
        let src = "oo::class create Bar {\n    method get {key} { return $key }\n}\nset b [Bar new]\nputs \"result: [$b get foo]\"\n";
        assert_eq!(
            refs_at(src, 1, 11),
            vec![1, 4],
            "decl + `$b` dispatch in a quote"
        );
    }

    /// FP guard: a same-named method on an *unrelated* class must not be pulled
    /// into `Bar`'s reference set — `$b get` resolves to `Bar`, never `Foo`.
    #[test]
    fn fp_unrelated_class_same_method_excluded() {
        let src = "oo::class create Bar {\n    method get {} { return 1 }\n}\noo::class create Foo {\n    method get {} { return 2 }\n}\nset b [Bar new]\nputs [$b get]\n";
        // cursor on Bar::get (line 1). References must be Bar's decl + the `$b
        // get` site (line 7), never Foo's decl (line 4).
        let got = refs_at(src, 1, 11);
        assert!(
            got.contains(&1) && got.contains(&7),
            "Bar decl + dispatch: {got:?}"
        );
        assert!(!got.contains(&4), "Foo::get must be excluded: {got:?}");
    }

    /// TN: a declared-but-never-dispatched method has only its declaration.
    #[test]
    fn tn_no_dispatch_only_declaration() {
        let src = "oo::class create Solo {\n    method ping {} { return pong }\n}\n";
        assert_eq!(refs_at(src, 1, 11), vec![1], "declaration only");
    }
}

// ───────────────────────── #957 — `my method` references ──────────────────

mod my_method_dispatch {
    use super::*;

    /// FN→TP: the reported shape — `my getOptions` nested in `return [ … ]`.
    #[test]
    fn tp_my_dispatch_in_command_substitution() {
        let src = "oo::class create Bar957 {\n    method getOptions {key} { return $key }\n    method get {key} { return [my getOptions $key] }\n}\n";
        // cursor on `getOptions` declaration (line 1, col 11).
        assert_eq!(refs_at(src, 1, 11), vec![1, 2], "decl + `my` in [ … ]");
    }

    /// TP: a bare top-level `my getOptions` (not inside `[ … ]`) is also a ref.
    #[test]
    fn tp_my_dispatch_top_level() {
        let src = "oo::class create C {\n    method getOptions {k} { return $k }\n    method run {} { my getOptions x }\n}\n";
        assert_eq!(refs_at(src, 1, 11), vec![1, 2], "decl + bare `my`");
    }

    /// TP: `my getOptions` embedded in a quoted / compound word
    /// (`"opts: [my getOptions $k]"`) is still a reference — the segmenter
    /// merges the whole word into one token, so the substitution is recovered
    /// by re-lexing the word slice.
    #[test]
    fn tp_my_dispatch_in_quoted_word() {
        let src = "oo::class create C {\n    method getOptions {k} { return $k }\n    method get {k} { return \"opts: [my getOptions $k]\" }\n}\n";
        assert_eq!(
            refs_at(src, 1, 11),
            vec![1, 2],
            "decl + `my` inside a quote"
        );
    }

    /// TP: `my getOptions` in a bareword concatenation (`[my getOptions]x`).
    #[test]
    fn tp_my_dispatch_in_compound_word() {
        let src = "oo::class create C {\n    method getOptions {k} { return $k }\n    method get {k} { return [my getOptions $k]-tail }\n}\n";
        assert_eq!(
            refs_at(src, 1, 11),
            vec![1, 2],
            "decl + `my` in a compound word"
        );
    }

    /// FP guard: `my other` must not count as a reference to `getOptions`.
    #[test]
    fn fp_my_other_method_excluded() {
        let src = "oo::class create C {\n    method getOptions {k} { return $k }\n    method other {k} { return $k }\n    method run {} { my other 1 }\n}\n";
        // references to `getOptions` (line 1) must be its declaration only.
        assert_eq!(refs_at(src, 1, 11), vec![1], "no `my other` leakage");
    }

    /// FP guard: a bare `getOptions` head (no `my`, no object) is *not* a call —
    /// a `TclOO` method is not a command in the body namespace.
    #[test]
    fn fp_bare_head_is_not_a_call() {
        let src = "oo::class create C {\n    method getOptions {k} { return $k }\n    method run {} { getOptions 1 }\n}\n";
        assert_eq!(refs_at(src, 1, 11), vec![1], "bare head is not a dispatch");
    }
}

// ─────────────────── #958 — `::tcl::mathfunc` expr functions ──────────────

mod mathfunc_expr {
    use super::*;

    /// FN→TP: the reported shape — `Pi958()` inside `[expr { … }]` dispatches to
    /// the command `::tcl::mathfunc::Pi958`.
    #[test]
    fn tp_mathfunc_in_nested_expr_subst() {
        let src = "namespace eval ::tcl::mathfunc {\n    proc Pi958 {} { return 3.14 }\n}\nset x [expr {Pi958() / 2.0}]\n";
        // cursor on the `Pi958` proc name (line 1, col 9).
        assert_eq!(refs_at(src, 1, 9), vec![1, 3], "decl + expr-fn call");
    }

    /// TP: the same via a top-level `expr` command (not nested in `[ … ]`).
    #[test]
    fn tp_mathfunc_in_top_level_expr() {
        let src = "namespace eval ::tcl::mathfunc {\n    proc Half {x} { return [expr {$x / 2.0}] }\n}\nexpr {Half(4)}\n";
        let got = refs_at(src, 1, 9);
        assert!(got.contains(&1) && got.contains(&3), "decl + call: {got:?}");
    }

    /// FP guard: a *different* math function must not be pulled in — `Tau()`
    /// is not a reference to `Pi958`.
    #[test]
    fn fp_other_mathfunc_excluded() {
        let src = "namespace eval ::tcl::mathfunc {\n    proc Pi958 {} { return 3.14 }\n    proc Tau958 {} { return 6.28 }\n}\nset x [expr {Tau958()}]\n";
        assert_eq!(refs_at(src, 1, 9), vec![1], "only Pi958's declaration");
    }

    /// TN: a math-func proc that is never applied has only its declaration.
    #[test]
    fn tn_unused_mathfunc_only_declaration() {
        let src = "namespace eval ::tcl::mathfunc {\n    proc Unused {} { return 0 }\n}\n";
        assert_eq!(refs_at(src, 1, 9), vec![1], "declaration only");
    }
}

// ─────────────────────── #955 — pkgIndex `$dir` W210 ──────────────────────

mod pkgindex_dir {
    use super::*;

    const READ_DIR: &str = "set f [file join $dir pkg.tcl]\nsource $f\n";

    /// FN→(no diagnostic): reading `$dir` in a `pkgIndex.tcl` is not
    /// read-before-set — the loader sets it before the script runs.
    #[test]
    fn tp_dir_read_in_pkgindex_suppressed() {
        assert!(
            !has_w210(READ_DIR, Some("/proj/pkgIndex.tcl")),
            "$dir in pkgIndex.tcl must not draw W210"
        );
    }

    /// FP guard: the suppression is filename-scoped — the same `$dir` read in
    /// an ordinary file is a genuine read-before-set.
    #[test]
    fn fp_dir_read_in_ordinary_file_still_fires() {
        assert!(
            has_w210(READ_DIR, Some("/proj/other.tcl")),
            "$dir outside pkgIndex.tcl is still read-before-set"
        );
    }

    /// FP guard: only `dir` is implicit — an unrelated undefined read in a
    /// `pkgIndex.tcl` still draws W210.
    #[test]
    fn fp_other_undefined_var_in_pkgindex_still_fires() {
        let src = "set f [file join $dir $missing]\nsource $f\n";
        assert!(
            has_w210(src, Some("/proj/pkgIndex.tcl")),
            "an undefined `$missing` is still read-before-set in pkgIndex.tcl"
        );
    }

    /// FP guard: the gate matches the *basename* exactly, so a look-alike
    /// filename that merely ends in `pkgIndex.tcl` (`notpkgIndex.tcl`) is not
    /// a package index and its `$dir` read still fires.
    #[test]
    fn fp_lookalike_filename_still_fires() {
        assert!(
            has_w210(READ_DIR, Some("/proj/notpkgIndex.tcl")),
            "a file merely ending in `pkgIndex.tcl` is not a package index"
        );
    }

    /// TN: a `dir` the script sets itself reads cleanly everywhere.
    #[test]
    fn tn_locally_set_dir_is_clean() {
        let src = "set dir /tmp\nset f [file join $dir pkg.tcl]\n";
        assert!(
            !has_w210(src, Some("/proj/other.tcl")),
            "a locally-set var is defined before read"
        );
    }
}

// ──────────────────────── #954 — apply lambda tokens ──────────────────────

mod apply_lambda {
    use super::*;

    /// FN→TP: commands inside an `apply` lambda body are highlighted as a
    /// script (the reported bug), and the bare arg-list name is a parameter.
    #[test]
    fn tp_bare_arglist_param_and_body() {
        let src = "apply {dir {\n    puts $dir\n}} /tmp\n";
        assert_eq!(
            kind_of(src, "tcl", "dir").as_deref(),
            Some("parameter"),
            "bare arg-list `dir` is a parameter declaration"
        );
        assert_eq!(
            kind_of(src, "tcl", "puts").as_deref(),
            Some("function"),
            "the body command `puts` is highlighted as a function"
        );
    }

    /// TP: a braced arg-list keeps its parameter names.
    #[test]
    fn tp_braced_arglist_params() {
        let src = "apply {{a b} { expr {$a + $b} }} 1 2\n";
        assert_eq!(kind_of(src, "tcl", "a").as_deref(), Some("parameter"));
        assert_eq!(kind_of(src, "tcl", "b").as_deref(), Some("parameter"));
    }

    /// FP guard: `apply $lambda` is a runtime value, not a literal — the
    /// `$lambda` word stays a variable and is never split into a lambda.
    #[test]
    fn fp_variable_lambda_not_recursed() {
        let src = "apply $lambda a b\n";
        assert_eq!(
            kind_of(src, "tcl", "lambda").as_deref(),
            Some("variable"),
            "`$lambda` is a variable, not a lambda literal"
        );
    }
}
