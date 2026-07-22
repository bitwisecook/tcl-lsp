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

//! End-to-end coverage for the issue #954 follow-up, against the real,
//! packaged native server.
//!
//! The original #954 fix (merged as part of v2.1.11, `cb10e7c90`) corrected
//! a bare-unbraced-parameter-list highlighting bug in `apply {dir {…}}`, but
//! the issue was reopened: the reporter's actual screenshot was a
//! pkgIndex.tcl-style `package ifneeded name ver [list apply {dir {…}} $dir]`
//! entry — `apply` reached *indirectly* through the `[list …]`
//! command-quoting idiom, a case the first fix never touched. This file
//! exercises the real reported repro end-to-end, plus the closely related
//! shapes the general (registry-driven) fix now also covers.

use crate::common::helpers::*;
use crate::common::{Lsp, unique_uri};

fn legend(lsp: &Lsp) -> Vec<String> {
    lsp.initialize_result()["capabilities"]["semanticTokensProvider"]["legend"]["tokenTypes"]
        .as_array()
        .expect("tokenTypes legend")
        .iter()
        .map(|v| v.as_str().unwrap_or("").to_owned())
        .collect()
}

struct TypedToken {
    line: i64,
    char: i64,
    length: i64,
    ttype: String,
}

fn typed(lsp: &mut Lsp, legend: &[String], uri: &str) -> Vec<TypedToken> {
    let raw = lsp.semantic_tokens(uri);
    decode_semantic_tokens(&raw)
        .into_iter()
        .map(|t| TypedToken {
            line: t.line,
            char: t.char,
            length: t.length,
            ttype: legend[usize::try_from(t.ttype).unwrap()].clone(),
        })
        .collect()
}

fn open_doc(lsp: &mut Lsp, source: &str) -> String {
    let uri = unique_uri("tcl");
    lsp.open_ready(&uri, source);
    uri
}

/// The source text a decoded token covers (single-line tokens only).
fn covered<'a>(source: &'a str, tok: &TypedToken) -> &'a str {
    let line = source
        .split('\n')
        .nth(usize::try_from(tok.line).unwrap())
        .unwrap_or("");
    let start = usize::try_from(tok.char).unwrap();
    let end = start + usize::try_from(tok.length).unwrap();
    line.get(start..end).unwrap_or("")
}

fn has_typed(toks: &[TypedToken], source: &str, text: &str, ttype: &str) -> bool {
    toks.iter()
        .any(|t| t.ttype == ttype && covered(source, t) == text)
}

/// The exact reported repro: a pkgIndex.tcl-style `package ifneeded` entry
/// whose deferred script is built with `[list apply {dir {…}} $dir]` to
/// capture the install directory safely. Every command inside the real
/// apply body must tokenise — not sit inside one opaque string, which is
/// what the reporter's screenshot showed.
#[test]
fn test_pkgindex_list_quoted_apply_body_highlights() {
    let mut lsp = Lsp::tcl();
    let legend_names = legend(&lsp);
    let source = "package ifneeded myPackage 1.0.0 [list apply {dir {\n\
                  \n\
                  \x20   source [file join $dir src font.tcl]\n\
                  \x20   source [file join $dir src utils.tcl]\n\
                  \n\
                  }} $dir]\n";
    let uri = open_doc(&mut lsp, source);
    let toks = typed(&mut lsp, &legend_names, &uri);
    assert!(
        has_typed(&toks, source, "source", "keyword"),
        "expected `source` to tokenise inside the list-quoted apply body; got {:?}",
        toks.iter()
            .map(|t| (t.line, t.char, covered(source, t), t.ttype.as_str()))
            .collect::<Vec<_>>()
    );
    assert!(
        has_typed(&toks, source, "file", "function"),
        "expected `file` to tokenise inside the list-quoted apply body"
    );
    assert!(
        has_typed(&toks, source, "dir", "parameter"),
        "expected the bare `dir` param to be a Parameter declaration"
    );
}

/// A `::`-qualified `::apply` inside the `[list …]` wrapper resolves the
/// same way (registry `get` strips a leading `::`).
#[test]
fn test_qualified_apply_list_quoted_body_highlights() {
    let mut lsp = Lsp::tcl();
    let legend_names = legend(&lsp);
    let source = "package ifneeded p 1.0 [list ::apply {dir {puts $dir}} $dir]\n";
    let uri = open_doc(&mut lsp, source);
    let toks = typed(&mut lsp, &legend_names, &uri);
    assert!(
        has_typed(&toks, source, "puts", "function"),
        "expected `puts` to tokenise inside the qualified `::apply` body; got {:?}",
        toks.iter()
            .map(|t| (t.line, t.char, covered(source, t), t.ttype.as_str()))
            .collect::<Vec<_>>()
    );
}

/// The same `[list apply {…} $x]` idiom under a different Body-role
/// enclosing command (`after idle`) — proves the fix is registry-driven
/// (any Body-role position), not special-cased to `package ifneeded`.
#[test]
fn test_after_idle_list_quoted_apply_body_highlights() {
    let mut lsp = Lsp::tcl();
    let legend_names = legend(&lsp);
    let source = "after idle [list apply {{x} {puts $x}} 5]\n";
    let uri = open_doc(&mut lsp, source);
    let toks = typed(&mut lsp, &legend_names, &uri);
    assert!(
        has_typed(&toks, source, "puts", "function"),
        "expected `puts` to tokenise inside the after-idle list-quoted apply body"
    );
    assert!(
        has_typed(&toks, source, "x", "parameter"),
        "expected the `x` param to be a Parameter declaration"
    );
}

/// `package ifneeded`'s script argument, as a *literal* braced script (no
/// `[list …]` wrapper), must also recurse — the sibling half of the fix
/// (the argument previously carried no `ArgRole` at all).
#[test]
fn test_pkgindex_literal_script_highlights() {
    let mut lsp = Lsp::tcl();
    let legend_names = legend(&lsp);
    let source = "package ifneeded myPackage 1.0.0 {\n\x20   source [file join $dir x.tcl]\n}\n";
    let uri = open_doc(&mut lsp, source);
    let toks = typed(&mut lsp, &legend_names, &uri);
    assert!(
        has_typed(&toks, source, "source", "keyword"),
        "expected `source` to tokenise inside package ifneeded's literal script body"
    );
}

/// FP guard: an ordinary data list must not be split as if it were a
/// deferred `apply` command construction.
#[test]
fn test_plain_data_list_is_not_split_as_lambda() {
    let mut lsp = Lsp::tcl();
    let legend_names = legend(&lsp);
    let source = "set data [list puts hello world]\n";
    let uri = open_doc(&mut lsp, source);
    let toks = typed(&mut lsp, &legend_names, &uri);
    assert!(
        !has_typed(&toks, source, "hello", "parameter"),
        "a plain data list must never be split as a lambda literal; got {:?}",
        toks.iter()
            .map(|t| (t.line, t.char, covered(source, t), t.ttype.as_str()))
            .collect::<Vec<_>>()
    );
}

/// Direct (non-list-quoted) `apply` regression check — the original #954
/// fix (bare unbraced param list) must still hold.
#[test]
fn test_direct_apply_bare_param_still_highlights() {
    let mut lsp = Lsp::tcl();
    let legend_names = legend(&lsp);
    let source = "apply {dir {\n\x20   puts $dir\n}} /tmp\n";
    let uri = open_doc(&mut lsp, source);
    let toks = typed(&mut lsp, &legend_names, &uri);
    assert!(has_typed(&toks, source, "dir", "parameter"));
    assert!(has_typed(&toks, source, "puts", "function"));
}
