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

//! Diagnostics raised from the scope helpers: W215 unreachable names, W116 and W117 stub shadows and W127 closed-set values.

use super::*;

fn diag_codes(source: &str, dialect: &str) -> Vec<String> {
    let mut a = Analyser::new();
    a.analyse(source, dialect)
        .diagnostics
        .iter()
        .map(|d| d.code.to_string())
        .collect()
}

#[test]
fn w215_fires_for_unreachable_quoted_name() {
    assert!(diag_codes("set \"a}b\" 1", "tcl").contains(&"W215".to_string()));
}

#[test]
fn w215_quiet_for_braced_escaped_name() {
    // `{a\}b}` is braced, so `\}` is preserved verbatim and the
    // runtime name `a\}b` IS brace-substitutable.
    assert!(!diag_codes("set {a\\}b} 1", "tcl").contains(&"W215".to_string()));
}

#[test]
fn w215_fires_for_array_index_with_paren() {
    assert!(diag_codes("set arr(a)b) 1", "tcl").contains(&"W215".to_string()));
}

#[test]
fn w215_quiet_for_normal_name() {
    assert!(!diag_codes("set normal 1", "tcl").contains(&"W215".to_string()));
}

// A namespace-qualified `set` target with
// a dynamic trailing segment (`set ns::$k 1`) must not fire W215 —
// `word_piece`'s reconstruction re-braces an unbraced `$k` piece to
// `${k}` when rendering the word's display text (so an adjacent literal
// suffix can't run into it), and that reconstruction — not the source,
// not the runtime name — was what the reachability check inspected.
// Confirmed against tclsh 8.6.14: `set k client_addr; set
// ::ns::$k hello` writes the perfectly ordinary, `$`-reachable variable
// `::ns::client_addr`; nothing about it is unreachable.

#[test]
fn w215_quiet_for_namespace_qualified_dynamic_suffix() {
    // FP: the exact corpus shape (`rust/tcl-irule-test/tcl/runner.tcl`,
    // `foreach {k v} $values { set ::state::connection::$k $v }`).
    assert!(!diag_codes("set ::state::connection::$k $v", "tcl").contains(&"W215".to_string()));
}

#[test]
fn w215_quiet_for_dynamic_suffix_without_namespace() {
    // FP: the bug is not namespace-specific — any unbraced word with an
    // embedded, non-leading `$var` piece is affected.
    assert!(!diag_codes("set prefix_$k 1", "tcl").contains(&"W215".to_string()));
}

#[test]
fn w215_quiet_for_dynamic_suffix_with_command_substitution() {
    // FP: `[cmd]` pieces hit the same reconstruction path as `$var`.
    assert!(!diag_codes("set prefix_[cmd] 1", "tcl").contains(&"W215".to_string()));
}

#[test]
fn w215_still_fires_for_static_typo_after_the_fix() {
    // TN: the reconstruction path must not blunt the check for the genuine
    // case it exists for — a fully static (no `$`/`[`) stray-delimiter typo.
    assert!(diag_codes("set \"a}b\" 1", "tcl").contains(&"W215".to_string()));
}

#[test]
fn w215_suggests_close_in_scope_variable() {
    // TP: `"a}b"` is a stray-brace typo of the already-defined `ab`
    // (edit distance 1) — the message names it.
    let mut a = Analyser::new();
    let r = a.analyse("set ab 1\nset \"a}b\" 2\n", "tcl");
    let d = r
        .diagnostics
        .iter()
        .find(|d| d.code == DiagCode::W215)
        .expect("W215");
    assert!(
        d.message.ends_with("; did you mean 'ab'?"),
        "expected an 'ab' suggestion: {:?}",
        d.message
    );
}

#[test]
fn w215_no_suggestion_when_nothing_is_close() {
    // FP guard: no defined variable sits within the edit budget —
    // the message keeps its plain form.
    let mut a = Analyser::new();
    let r = a.analyse("set totally 1\nset \"qq}zz\" 2\n", "tcl");
    let d = r
        .diagnostics
        .iter()
        .find(|d| d.code == DiagCode::W215)
        .expect("W215");
    assert!(
        !d.message.contains("did you mean"),
        "no suggestion expected: {:?}",
        d.message
    );
}

#[test]
fn w215_quiet_for_backslash_continued_param_list() {
    // A parameter list wrapped across lines with `\`. Tcl
    // list-parses the braced param list, so `ddrtol\<newline>ddatol` is two
    // parameters (`ddrtol`, `ddatol`), not a single `ddrtol\` name — no
    // W215 unreachable-name warning should fire.
    let src = "proc p {a ddrtol\\\n        ddatol} { list $a $ddrtol $ddatol }";
    assert!(!diag_codes(src, "tcl").contains(&"W215".to_string()));

    // A TclOO `method` with a wrapped parameter list.
    let method_src = "oo::class create C {\n  method Fdjac2 {funct ifree ddrtol\\\n      ddatol} { list $ddrtol $ddatol }\n}\n";
    assert!(!diag_codes(method_src, "tcl").contains(&"W215".to_string()));
}

#[test]
fn w116_fires_for_stub_shadowing_builtin() {
    let src = "# tcl-lsp: stubs-begin\n# tcl-lsp: stub set {a:var b}\n# tcl-lsp: stubs-end\n";
    assert!(diag_codes(src, "tcl").contains(&"W116".to_string()));
}

#[test]
fn w117_fires_for_stub_expr_shadowing_builtin() {
    let src = "# tcl-lsp: stubs-begin\n# tcl-lsp: stub expr-func sin 1\n# tcl-lsp: stubs-end\n";
    assert!(diag_codes(src, "tcl").contains(&"W117".to_string()));
}

#[test]
fn sidecar_stub_shadows_never_point_into_the_source_document() {
    let root = std::env::temp_dir().join(format!(
        "tcl-lsp-sidecar-shadow-{}-{}",
        std::process::id(),
        std::thread::current().name().unwrap_or("test")
    ));
    std::fs::create_dir_all(&root).unwrap();
    std::fs::write(
        root.join("tcl.tcl.stubs"),
        "stub set {value}\nstub expr-func sin 1\n",
    )
    .unwrap();
    let source = root.join("main.tcl");
    let result = Analyser::new()
        .with_file_path(Some(source.display().to_string()))
        .analyse("set value 1\n", "tcl");
    assert!(
        !result
            .diagnostics
            .iter()
            .any(|d| matches!(d.code, DiagCode::W116 | DiagCode::W117)),
        "sidecar shadow diagnostics must not be published on source spans: {:?}",
        result.diagnostics
    );
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn w127_fires_for_value_outside_closed_set() {
    let src = "when HTTP_REQUEST { HTTP::version \"2.0\" }";
    assert!(diag_codes(src, "f5-irules").contains(&"W127".to_string()));
}

#[test]
fn w127_quiet_for_allowed_value() {
    let src = "when HTTP_REQUEST { HTTP::version \"1.1\" }";
    assert!(!diag_codes(src, "f5-irules").contains(&"W127".to_string()));
}
