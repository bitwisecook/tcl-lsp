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

//! End-to-end TP/FP/TN/FN matrix for preview issues #954-#958, driven over
//! real JSON-RPC against the packaged server:
//!
//! * #954 — commands inside an `apply` lambda body highlight as a script, and
//!   a bare arg-list name is a `parameter` (semantic tokens).
//! * #955 — a `$dir` read in a `pkgIndex.tcl` is not W210 read-before-set
//!   (publishDiagnostics), but the suppression is filename-scoped.
//! * #956 — a `$obj method` dispatch is a reference and is counted by the
//!   member lens.
//! * #957 — a `my method` dispatch (including nested in `[ … ]`) is a reference
//!   and is counted by the member lens.
//! * #958 — a `::tcl::mathfunc::<fn>` expr-function application is a reference
//!   to the backing proc.

#![allow(clippy::cast_possible_truncation)]

use crate::common::helpers::*;
use crate::common::{Lsp, unique_uri};
use serde_json::Value;

/// A unique `pkgIndex.tcl` document URI (unique directory, fixed basename so
/// the server's filename gate fires).
fn pkgindex_uri() -> String {
    format!("{}/pkgIndex.tcl", unique_uri("d"))
}

/// The `tokenTypes` legend advertised at `initialize`.
fn legend(lsp: &Lsp) -> Vec<String> {
    lsp.initialize_result()["capabilities"]["semanticTokensProvider"]["legend"]["tokenTypes"]
        .as_array()
        .expect("tokenTypes legend")
        .iter()
        .map(|v| v.as_str().unwrap_or_default().to_owned())
        .collect()
}

/// Word-bounded 0-based (line, col) of the first standalone `needle` in `src`.
fn pos_of(src: &str, needle: &str) -> (u32, u32) {
    let is_word = |b: u8| b.is_ascii_alphanumeric() || b == b'_' || b == b':';
    let bytes = src.as_bytes();
    let byte = (0..=src.len().saturating_sub(needle.len()))
        .find(|&p| {
            src[p..].starts_with(needle)
                && (p == 0 || !is_word(bytes[p - 1]))
                && (p + needle.len() == src.len() || !is_word(bytes[p + needle.len()]))
        })
        .unwrap_or_else(|| panic!("needle {needle:?} not found in source"));
    let before = &src[..byte];
    let line = before.matches('\n').count() as u32;
    let col = (byte - before.rfind('\n').map_or(0, |n| n + 1)) as u32;
    (line, col)
}

/// The semantic-token type name covering the first standalone `needle`, or
/// `None`.
fn token_type_of(lsp: &mut Lsp, uri: &str, src: &str, needle: &str) -> Option<String> {
    let legend = legend(lsp);
    let (line, col) = pos_of(src, needle);
    let toks = decode_semantic_tokens(&lsp.semantic_tokens(uri));
    toks.iter()
        .find(|t| {
            t.line == i64::from(line)
                && t.char <= i64::from(col)
                && i64::from(col) < t.char + t.length
        })
        .map(|t| legend[usize::try_from(t.ttype).unwrap()].clone())
}

/// The reference-count lens title anchored on `line` (member lenses carry an
/// eager `command.title`).
fn member_lens_title(lsp: &mut Lsp, uri: &str, line: i64) -> Option<String> {
    let ls = match lsp.code_lens(uri) {
        Value::Array(a) => a,
        _ => Vec::new(),
    };
    ls.iter()
        .find(|l| l["range"]["start"]["line"].as_i64() == Some(line))
        .and_then(|l| l["command"]["title"].as_str())
        .map(str::to_owned)
}

/// Whether any diagnostic carries the string `code` (W-codes are strings).
fn has_diag_code(diags: &[Value], code: &str) -> bool {
    diags
        .iter()
        .any(|d| d.get("code").and_then(Value::as_str) == Some(code))
}

// ───────────────────────── #954 — apply lambda tokens ─────────────────────

mod apply_lambda {
    use super::*;

    #[test]
    fn tp_apply_body_and_bare_param_highlight() {
        let mut lsp = Lsp::tcl();
        let uri = unique_uri("tcl");
        let src = "apply {dir {\n    puts $dir\n}} /tmp\n";
        lsp.open_ready(&uri, src);
        assert_eq!(
            token_type_of(&mut lsp, &uri, src, "puts").as_deref(),
            Some("function"),
            "the apply body command must highlight as a script"
        );
        assert_eq!(
            token_type_of(&mut lsp, &uri, src, "dir").as_deref(),
            Some("parameter"),
            "the bare arg-list name is a parameter declaration"
        );
    }

    #[test]
    fn fp_variable_lambda_stays_a_variable() {
        let mut lsp = Lsp::tcl();
        let uri = unique_uri("tcl");
        let src = "apply $lambda a b\n";
        lsp.open_ready(&uri, src);
        assert_eq!(
            token_type_of(&mut lsp, &uri, src, "lambda").as_deref(),
            Some("variable"),
            "`$lambda` is a runtime value, never split into a lambda literal"
        );
    }
}

// ─────────────────────── #955 — pkgIndex `$dir` W210 ──────────────────────

mod pkgindex_dir {
    use super::*;

    const READ_DIR: &str = "set f [file join $dir pkg.tcl]\nsource $f\n";

    #[test]
    fn tp_dir_in_pkgindex_no_w210() {
        let mut lsp = Lsp::tcl();
        let uri = pkgindex_uri();
        let diags = lsp.open_ready(&uri, READ_DIR);
        assert!(
            !has_diag_code(&diags, "W210"),
            "$dir in pkgIndex.tcl must not draw W210: {diags:?}"
        );
    }

    #[test]
    fn fp_dir_in_ordinary_file_fires_w210() {
        let mut lsp = Lsp::tcl();
        let uri = unique_uri("tcl");
        let diags = lsp.open_ready(&uri, READ_DIR);
        assert!(
            has_diag_code(&diags, "W210"),
            "$dir outside pkgIndex.tcl is still read-before-set: {diags:?}"
        );
    }

    #[test]
    fn fp_other_undefined_in_pkgindex_fires_w210() {
        let mut lsp = Lsp::tcl();
        let uri = pkgindex_uri();
        let diags = lsp.open_ready(&uri, "set f [file join $dir $missing]\nsource $f\n");
        assert!(
            has_diag_code(&diags, "W210"),
            "only `dir` is implicit; `$missing` is still read-before-set: {diags:?}"
        );
    }
}

// ───────────────────────── #956 — `$obj method` refs ──────────────────────

mod obj_method_dispatch {
    use super::*;

    const SRC: &str = "oo::class create Bar956 {\n    method get {key} { return $key }\n}\nset b [Bar956 new]\nputs [$b get foo]\n";

    #[test]
    fn tp_obj_dispatch_reference_and_lens() {
        let mut lsp = Lsp::tcl();
        let uri = unique_uri("tcl");
        lsp.open_ready(&uri, SRC);
        // References on the `get` decl (line 1) include the `$b get` site (4).
        let lines = start_lines(&lsp.references(&uri, 1, 11, true));
        assert!(lines.contains(&1) && lines.contains(&4), "{lines:?}");
        // The member lens counts the one external dispatch.
        assert_eq!(
            member_lens_title(&mut lsp, &uri, 1).as_deref(),
            Some("1 reference")
        );
    }

    #[test]
    fn tn_uncalled_method_zero_references() {
        let mut lsp = Lsp::tcl();
        let uri = unique_uri("tcl");
        lsp.open_ready(
            &uri,
            "oo::class create Solo {\n    method lonely {} {}\n}\nSolo new\n",
        );
        assert_eq!(
            member_lens_title(&mut lsp, &uri, 1).as_deref(),
            Some("0 references")
        );
    }
}

// ───────────────────────── #957 — `my method` refs ────────────────────────

mod my_method_dispatch {
    use super::*;

    const SRC: &str = "oo::class create Bar957 {\n    method getOptions {key} { return $key }\n    method get {key} { return [my getOptions $key] }\n}\n";

    #[test]
    fn tp_my_dispatch_reference_and_lens() {
        let mut lsp = Lsp::tcl();
        let uri = unique_uri("tcl");
        lsp.open_ready(&uri, SRC);
        // References on `getOptions` (line 1) include the nested `my` site (2).
        let lines = start_lines(&lsp.references(&uri, 1, 11, true));
        assert!(lines.contains(&1) && lines.contains(&2), "{lines:?}");
        // The lens above `method getOptions` counts the intra-class dispatch.
        assert_eq!(
            member_lens_title(&mut lsp, &uri, 1).as_deref(),
            Some("1 reference")
        );
    }

    #[test]
    fn fp_my_other_method_not_counted() {
        let mut lsp = Lsp::tcl();
        let uri = unique_uri("tcl");
        lsp.open_ready(
            &uri,
            "oo::class create C {\n    method getOptions {k} { return $k }\n    method other {k} { return $k }\n    method run {} { my other 1 }\n}\n",
        );
        // `getOptions` has no dispatch — the lens reads zero.
        assert_eq!(
            member_lens_title(&mut lsp, &uri, 1).as_deref(),
            Some("0 references")
        );
    }
}

// ─────────────────── #958 — `::tcl::mathfunc` expr functions ──────────────

mod mathfunc_expr {
    use super::*;

    const SRC: &str = "namespace eval ::tcl::mathfunc {\n    proc Pi958 {} { return 3.14 }\n}\nset x [expr {Pi958() / 2.0}]\n";

    #[test]
    fn tp_mathfunc_application_is_a_reference() {
        let mut lsp = Lsp::tcl();
        let uri = unique_uri("tcl");
        lsp.open_ready(&uri, SRC);
        // References on the `Pi958` proc (line 1) include the `expr` call (3).
        let lines = start_lines(&lsp.references(&uri, 1, 9, true));
        assert!(lines.contains(&1) && lines.contains(&3), "{lines:?}");
    }

    #[test]
    fn fp_other_mathfunc_not_counted() {
        let mut lsp = Lsp::tcl();
        let uri = unique_uri("tcl");
        lsp.open_ready(
            &uri,
            "namespace eval ::tcl::mathfunc {\n    proc Pi958 {} { return 3.14 }\n    proc Tau958 {} { return 6.28 }\n}\nset x [expr {Tau958()}]\n",
        );
        // References on `Pi958` are just its declaration — `Tau958()` is not one.
        let lines = start_lines(&lsp.references(&uri, 1, 9, true));
        assert!(lines.contains(&1) && !lines.contains(&4), "{lines:?}");
    }
}
