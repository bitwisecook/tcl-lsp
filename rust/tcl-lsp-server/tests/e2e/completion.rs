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

//! Native port of `tests/lsp_e2e/test_completion_e2e.py`.
//!
//! Completion, end-to-end against the packaged server. Full-parity port of the
//! request/response cases: the text-edit assertions matter on the live surface,
//! since an editor applies `textEdit` verbatim.

use crate::common::helpers::*;
use crate::common::{Lsp, unique_uri};

use serde_json::Value;

/// The completion labels at a position.
fn labels(lsp: &mut Lsp, uri: &str, line: u32, ch: u32) -> Vec<String> {
    completion_labels(&lsp.completion(uri, line, ch))
}

/// Completion items keyed by label at a position.
fn by_label(
    lsp: &mut Lsp,
    uri: &str,
    line: u32,
    ch: u32,
) -> std::collections::BTreeMap<String, Value> {
    let mut out = std::collections::BTreeMap::new();
    for item in completion_items(&lsp.completion(uri, line, ch)) {
        let label = item
            .get("label")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_owned();
        out.insert(label, item);
    }
    out
}

/// The `documentation` text of a completion item (string or `MarkupContent`).
fn doc_text(item: &Value) -> String {
    match item.get("documentation") {
        Some(Value::String(s)) => s.clone(),
        Some(Value::Object(o)) => o
            .get("value")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_owned(),
        _ => String::new(),
    }
}

// -- TestCommandCompletion -----------------------------------------------

#[test]
fn empty_line_returns_commands() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    lsp.open_ready(&uri, "");
    let ls: std::collections::BTreeSet<String> = labels(&mut lsp, &uri, 0, 0).into_iter().collect();
    for expected in ["set", "proc", "puts"] {
        assert!(ls.contains(expected), "missing {expected:?} in {ls:?}");
    }
}

#[test]
fn partial_command() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    lsp.open_ready(&uri, "pu");
    let ls = labels(&mut lsp, &uri, 0, 2);
    assert!(ls.contains(&"puts".to_owned()));
    assert!(!ls.contains(&"set".to_owned()));
}

#[test]
fn no_math_operators() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    lsp.open_ready(&uri, "");
    let ls = labels(&mut lsp, &uri, 0, 0);
    assert!(!ls.contains(&"+".to_owned()));
    assert!(!ls.contains(&"-".to_owned()));
}

#[test]
fn user_proc_in_completions() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    lsp.open_ready(&uri, "proc myHelper {x} { return $x }\nmy");
    assert!(labels(&mut lsp, &uri, 1, 2).contains(&"myHelper".to_owned()));
}

#[test]
fn builtin_command_has_documentation() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    lsp.open_ready(&uri, "se");
    let item = by_label(&mut lsp, &uri, 0, 2)["set"].clone();
    let dt = doc_text(&item).to_lowercase();
    assert!(
        dt.contains("variable") || dt.contains("value"),
        "doc: {dt:?}"
    );
}

// -- TestVariableCompletion ----------------------------------------------

#[test]
fn dollar_triggers_vars() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    lsp.open_ready(&uri, "set greeting hello\nputs $");
    assert!(labels(&mut lsp, &uri, 1, 6).contains(&"$greeting".to_owned()));
}

#[test]
fn partial_var_name() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    lsp.open_ready(&uri, "set greeting hello\nset goodbye bye\nputs $gre");
    let ls = labels(&mut lsp, &uri, 2, 9);
    assert!(ls.contains(&"$greeting".to_owned()));
    assert!(!ls.contains(&"$goodbye".to_owned()));
}

#[test]
fn var_in_proc_scope() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let src = "proc foo {x} {\n    set local 1\n    puts $\n}\n";
    lsp.open_ready(&uri, src);
    let ls = labels(&mut lsp, &uri, 2, 10);
    assert!(ls.contains(&"$x".to_owned()));
    assert!(ls.contains(&"$local".to_owned()));
}

#[test]
fn namespace_var_completion() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let src = "namespace eval myns {\n    variable nsVar 1\n    puts $\n}\n";
    lsp.open_ready(&uri, src);
    assert!(labels(&mut lsp, &uri, 2, 10).contains(&"$nsVar".to_owned()));
}

#[test]
fn dollar_text_edit_replaces_dollar_sign() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    lsp.open_ready(&uri, "set testvar Test\nputs $");
    let edit = by_label(&mut lsp, &uri, 1, 6)["$testvar"]["textEdit"].clone();
    assert_eq!(edit["range"]["start"]["character"], 5);
    assert_eq!(edit["range"]["end"]["character"], 6);
    assert_eq!(edit["newText"], "$testvar");
}

#[test]
fn dollar_text_edit_replaces_partial_var_name() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    lsp.open_ready(&uri, "set greeting hello\nputs $gre");
    let edit = by_label(&mut lsp, &uri, 1, 9)["$greeting"]["textEdit"].clone();
    assert_eq!(edit["range"]["start"]["character"], 5);
    assert_eq!(edit["range"]["end"]["character"], 9);
    assert_eq!(edit["newText"], "$greeting");
}

#[test]
fn dollar_text_edit_brace_form() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    lsp.open_ready(&uri, "set greeting hello\nputs ${gre");
    let edit = by_label(&mut lsp, &uri, 1, 10)["$greeting"]["textEdit"].clone();
    assert_eq!(edit["range"]["start"]["character"], 5);
    assert_eq!(edit["range"]["end"]["character"], 10);
    assert_eq!(edit["newText"], "${greeting}");
}

#[test]
fn dollar_text_edit_brace_form_consumes_existing_close() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    lsp.open_ready(&uri, "set greeting hello\nputs ${gre}");
    let edit = by_label(&mut lsp, &uri, 1, 10)["$greeting"]["textEdit"].clone();
    assert_eq!(edit["range"]["start"]["character"], 5);
    assert_eq!(edit["range"]["end"]["character"], 11);
    assert_eq!(edit["newText"], "${greeting}");
}

#[test]
fn dollar_text_edit_brace_form_empty_with_close() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    lsp.open_ready(&uri, "set greeting hello\nputs ${}");
    let edit = by_label(&mut lsp, &uri, 1, 7)["$greeting"]["textEdit"].clone();
    assert_eq!(edit["range"]["start"]["character"], 5);
    assert_eq!(edit["range"]["end"]["character"], 8);
    assert_eq!(edit["newText"], "${greeting}");
}

#[test]
fn dollar_text_edit_midword_replaces_full_token() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    lsp.open_ready(&uri, "set greeting hello\nputs $greeting");
    let edit = by_label(&mut lsp, &uri, 1, 9)["$greeting"]["textEdit"].clone();
    assert_eq!(edit["range"]["start"]["character"], 5);
    assert_eq!(edit["range"]["end"]["character"], 14);
    assert_eq!(edit["newText"], "$greeting");
}

#[test]
fn dollar_text_edit_midword_brace_form_replaces_full_token() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    lsp.open_ready(&uri, "set greeting hello\nputs ${greeting}");
    let edit = by_label(&mut lsp, &uri, 1, 10)["$greeting"]["textEdit"].clone();
    assert_eq!(edit["range"]["start"]["character"], 5);
    assert_eq!(edit["range"]["end"]["character"], 16);
    assert_eq!(edit["newText"], "${greeting}");
}

#[test]
fn dollar_completion_omits_unsubstitutable_brace_names() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    lsp.open_ready(&uri, "set a_\\}_closebrace 1\nputs $");
    for label in labels(&mut lsp, &uri, 1, 6) {
        assert!(!label.contains('}'), "label had brace: {label:?}");
    }
}

#[test]
fn dollar_text_edit_brace_midword_with_hyphenated_name() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    lsp.open_ready(&uri, "set \"foo-bar\" 1\nputs ${foo-bar}");
    let by = by_label(&mut lsp, &uri, 1, 10);
    assert!(by.contains_key("$foo-bar"));
    let edit = by["$foo-bar"]["textEdit"].clone();
    assert_eq!(edit["range"]["start"]["character"], 5);
    assert_eq!(edit["range"]["end"]["character"], 15);
    assert_eq!(edit["newText"], "${foo-bar}");
}

#[test]
fn dollar_auto_braces_var_name_with_hyphen() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    lsp.open_ready(&uri, "set \"foo-bar\" 1\nputs $");
    let by = by_label(&mut lsp, &uri, 1, 6);
    assert!(by.contains_key("$foo-bar"));
    assert_eq!(by["$foo-bar"]["textEdit"]["newText"], "${foo-bar}");
}

#[test]
fn dollar_cross_namespace_offers_qualified_name() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let src =
        "namespace eval ::other {\n    variable baz 3\n}\nnamespace eval ::myns {\n    puts $\n}\n";
    lsp.open_ready(&uri, src);
    let by = by_label(&mut lsp, &uri, 4, 10);
    assert!(by.contains_key("$::other::baz"));
    assert_eq!(by["$::other::baz"]["textEdit"]["newText"], "$::other::baz");
}

#[test]
fn dollar_same_namespace_uses_bare_minimal_form() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let src = "namespace eval ::myns {\n    variable foo 1\n    puts $\n}\n";
    lsp.open_ready(&uri, src);
    let ls = labels(&mut lsp, &uri, 2, 10);
    assert!(ls.contains(&"$foo".to_owned()));
    assert!(!ls.contains(&"$::myns::foo".to_owned()));
}

#[test]
fn dollar_partial_filters_cross_namespace() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let src = "namespace eval ::other {\n    variable baz 3\n}\nnamespace eval ::myns {\n    variable foo 1\n    puts $::ot\n}\n";
    lsp.open_ready(&uri, src);
    let ls = labels(&mut lsp, &uri, 5, 14);
    assert!(ls.contains(&"$::other::baz".to_owned()));
    assert!(!ls.contains(&"$foo".to_owned()));
}

#[test]
fn dollar_completion_offers_dict_for_loop_vars() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let src = "proc p {} {\n    dict for {k v} {a 1 b 2} {\n        puts $\n    }\n}\n";
    lsp.open_ready(&uri, src);
    let ls = labels(&mut lsp, &uri, 2, 14);
    assert!(ls.contains(&"$k".to_owned()));
    assert!(ls.contains(&"$v".to_owned()));
}

#[test]
fn dollar_completion_offers_dict_with_keys_from_const_literal() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let src =
        "proc p {} {\n    set d {name alice age 30}\n    dict with d {\n        puts $\n    }\n}\n";
    lsp.open_ready(&uri, src);
    let ls = labels(&mut lsp, &uri, 3, 14);
    assert!(ls.contains(&"$name".to_owned()));
    assert!(ls.contains(&"$age".to_owned()));
}

#[test]
fn dollar_completion_tolerates_cursor_past_eol() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    lsp.open_ready(&uri, "set greeting hello\nputs $");
    assert!(labels(&mut lsp, &uri, 1, 100).contains(&"$greeting".to_owned()));
}

// -- TestArrayElementCompletion ------------------------------------------

#[test]
fn array_element_completion_offers_known_indices() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let src = "proc p {} {\n    set arr(name) hello\n    set arr(age) 42\n    puts $arr(\n}\n";
    lsp.open_ready(&uri, src);
    let ls = labels(&mut lsp, &uri, 3, 14);
    assert!(ls.contains(&"$arr(name)".to_owned()));
    assert!(ls.contains(&"$arr(age)".to_owned()));
}

#[test]
fn array_element_completion_filters_by_partial() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let src = "proc p {} {\n    set arr(name) hello\n    set arr(age) 42\n    puts $arr(na\n}\n";
    lsp.open_ready(&uri, src);
    let mut ls = labels(&mut lsp, &uri, 3, 16);
    ls.sort();
    assert_eq!(ls, vec!["$arr(name)".to_owned()]);
}

#[test]
fn array_element_completion_consumes_existing_close_paren() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let src = "proc p {} {\n    set arr(name) hello\n    puts $arr()\n}\n";
    lsp.open_ready(&uri, src);
    let edit = by_label(&mut lsp, &uri, 2, 14)["$arr(name)"]["textEdit"].clone();
    assert_eq!(edit["range"]["end"]["character"], 15);
    assert_eq!(edit["newText"], "$arr(name)");
}

// -- TestSubcommandCompletion --------------------------------------------

#[test]
fn string_subcommands() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    lsp.open_ready(&uri, "string ");
    let ls: std::collections::BTreeSet<String> = labels(&mut lsp, &uri, 0, 7).into_iter().collect();
    for expected in ["length", "match", "tolower"] {
        assert!(ls.contains(expected), "missing {expected:?} in {ls:?}");
    }
}

#[test]
fn partial_subcommand() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    lsp.open_ready(&uri, "string to");
    let ls: std::collections::BTreeSet<String> = labels(&mut lsp, &uri, 0, 9).into_iter().collect();
    for expected in ["tolower", "toupper", "totitle"] {
        assert!(ls.contains(expected), "missing {expected:?} in {ls:?}");
    }
    assert!(!ls.contains("length"));
}

#[test]
fn history_bare_subcommands() {
    // `history` is a `WithSubcommands` registry command like `string`, but a
    // bare call is itself valid Tcl (defaults to `history info`); completion
    // at the bare position must still offer its subcommand list.
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    lsp.open_ready(&uri, "history ");
    let ls: std::collections::BTreeSet<String> = labels(&mut lsp, &uri, 0, 8).into_iter().collect();
    for expected in ["add", "clear", "info", "keep"] {
        assert!(ls.contains(expected), "missing {expected:?} in {ls:?}");
    }
}

#[test]
fn namespace_subcommands() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    lsp.open_ready(&uri, "namespace ");
    let ls: std::collections::BTreeSet<String> =
        labels(&mut lsp, &uri, 0, 10).into_iter().collect();
    for expected in ["eval", "export"] {
        assert!(ls.contains(expected), "missing {expected:?} in {ls:?}");
    }
}

// -- TestSwitchCompletion ------------------------------------------------

#[test]
fn regexp_switches() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    lsp.open_ready(&uri, "regexp -");
    let ls: std::collections::BTreeSet<String> = labels(&mut lsp, &uri, 0, 8).into_iter().collect();
    for expected in ["-nocase", "-all"] {
        assert!(ls.contains(expected), "missing {expected:?} in {ls:?}");
    }
}

#[test]
fn partial_switch() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    lsp.open_ready(&uri, "lsort -no");
    assert!(labels(&mut lsp, &uri, 0, 9).contains(&"-nocase".to_owned()));
}

#[test]
fn socket_switches() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    lsp.open_ready(&uri, "socket -");
    let ls: std::collections::BTreeSet<String> = labels(&mut lsp, &uri, 0, 8).into_iter().collect();
    for expected in ["-server", "-myaddr"] {
        assert!(ls.contains(expected), "missing {expected:?} in {ls:?}");
    }
}

#[test]
fn switch_completion_ignores_semicolon_in_quoted_arg() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let source = "socket \"a;b\" -";
    lsp.open_ready(&uri, source);
    assert!(
        labels(&mut lsp, &uri, 0, u32::try_from(source.len()).unwrap())
            .contains(&"-server".to_owned())
    );
}

#[test]
fn switch_text_edit_replaces_partial_dash_prefix() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let source = "lsort -no";
    lsp.open_ready(&uri, source);
    let edit =
        by_label(&mut lsp, &uri, 0, u32::try_from(source.len()).unwrap())["-nocase"]["textEdit"]
            .clone();
    assert_eq!(edit["range"]["start"]["character"], 6);
    assert_eq!(edit["range"]["end"]["character"], 9);
    assert_eq!(edit["newText"], "-nocase");
}

#[test]
fn switch_text_edit_bare_dash() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let source = "regexp -";
    lsp.open_ready(&uri, source);
    let edit =
        by_label(&mut lsp, &uri, 0, u32::try_from(source.len()).unwrap())["-nocase"]["textEdit"]
            .clone();
    assert_eq!(edit["range"]["start"]["character"], 7);
    assert_eq!(edit["range"]["end"]["character"], 8);
    assert_eq!(edit["newText"], "-nocase");
}

#[test]
fn switch_has_documentation() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    lsp.open_ready(&uri, "socket -");
    let item = by_label(&mut lsp, &uri, 0, 8)["-server"].clone();
    let ds = doc_text(&item);
    assert!(!ds.is_empty(), "doc empty");
}

#[test]
fn switch_text_edit_with_single_char_partial() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let source = "regexp -n";
    lsp.open_ready(&uri, source);
    let edit =
        by_label(&mut lsp, &uri, 0, u32::try_from(source.len()).unwrap())["-nocase"]["textEdit"]
            .clone();
    assert_eq!(edit["range"]["start"]["character"], 7);
    assert_eq!(edit["range"]["end"]["character"], 9);
    assert_eq!(edit["newText"], "-nocase");
}

#[test]
fn switch_text_edit_with_longer_partial() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let source = "lsort -noc";
    lsp.open_ready(&uri, source);
    let edit =
        by_label(&mut lsp, &uri, 0, u32::try_from(source.len()).unwrap())["-nocase"]["textEdit"]
            .clone();
    assert_eq!(edit["range"]["start"]["character"], 6);
    assert_eq!(edit["range"]["end"]["character"], 10);
    assert_eq!(edit["newText"], "-nocase");
}

// -- TestScopeBindingCompletion ------------------------------------------

#[test]
fn dollar_global_var_from_namespace_offers_qualified() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let src = "set ::globalvar 9\nnamespace eval ::myns {\n    puts $\n}\n";
    lsp.open_ready(&uri, src);
    assert!(labels(&mut lsp, &uri, 2, 10).contains(&"$::globalvar".to_owned()));
}

#[test]
fn dollar_completion_offers_global_local_alias() {
    for decl in ["global myglobal", "global ::myglobal"] {
        let mut lsp = Lsp::tcl();
        let uri = unique_uri("tcl");
        let src = format!("set ::myglobal 9\nproc p {{}} {{\n    {decl}\n    puts $\n}}\n");
        lsp.open_ready(&uri, &src);
        let ls: std::collections::BTreeSet<String> =
            labels(&mut lsp, &uri, 3, 10).into_iter().collect();
        assert!(ls.contains("$myglobal"), "decl={decl:?} labels={ls:?}");
        assert!(ls.contains("$::myglobal"), "decl={decl:?} labels={ls:?}");
    }
}

#[test]
fn dollar_completion_offers_upvar_alias() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let src = "proc inner {var} {\n    upvar 1 $var alias\n    puts $\n}\n";
    lsp.open_ready(&uri, src);
    assert!(labels(&mut lsp, &uri, 2, 10).contains(&"$alias".to_owned()));
}

#[test]
fn dollar_completion_offers_namespace_upvar_alias() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let src = "namespace eval ::ns { variable foo 1 }\nproc p {} {\n    namespace upvar ::ns foo localfoo\n    puts $\n}\n";
    lsp.open_ready(&uri, src);
    assert!(labels(&mut lsp, &uri, 3, 10).contains(&"$localfoo".to_owned()));
}

#[test]
fn dollar_completion_offers_try_on_error_bindings() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let src = "proc p {} {\n    try {\n        error boom\n    } on error {try_msg try_opts} {\n        puts $\n    }\n}\n";
    lsp.open_ready(&uri, src);
    let ls = labels(&mut lsp, &uri, 4, 18);
    assert!(ls.contains(&"$try_msg".to_owned()));
    assert!(ls.contains(&"$try_opts".to_owned()));
}

#[test]
fn dollar_completion_offers_dict_update_aliases() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let src = "proc p {} {\n    set d {a 1 b 2}\n    dict update d a varA b varB {\n        puts $\n    }\n}\n";
    lsp.open_ready(&uri, src);
    let ls = labels(&mut lsp, &uri, 3, 14);
    assert!(ls.contains(&"$varA".to_owned()));
    assert!(ls.contains(&"$varB".to_owned()));
}

#[test]
fn dollar_completion_uplevel_zero_uses_global_scope() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let src = "set ::globalvar 9\nproc p {} {\n    set local_var 1\n    uplevel #0 {\n        set inside_var 99\n        puts $\n    }\n}\n";
    lsp.open_ready(&uri, src);
    let ls = labels(&mut lsp, &uri, 5, 18);
    assert!(ls.contains(&"$::globalvar".to_owned()));
    assert!(ls.contains(&"$inside_var".to_owned()));
    assert!(!ls.contains(&"$local_var".to_owned()));
}

#[test]
fn dollar_completion_uplevel_one_abstains_from_proc_scope() {
    // A non-`#0` `uplevel` body runs in the *caller's* frame (statically
    // unknown), where the enclosing proc's locals are not accessible.  Unlike
    // `uplevel #0` (which resolves to the global frame), completion must
    // abstain on the proc's locals here — offering `$local_var` would suggest a
    // variable that is not in scope at runtime — while still offering the
    // body's own declarations.
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let src = "proc p {} {\n    set local_var 1\n    uplevel 1 {\n        set body_var 5\n        puts $\n    }\n}\n";
    lsp.open_ready(&uri, src);
    let ls = labels(&mut lsp, &uri, 4, 14);
    assert!(
        !ls.contains(&"$local_var".to_owned()),
        "proc-local must not be offered inside an `uplevel 1` body: {ls:?}"
    );
    assert!(
        ls.contains(&"$body_var".to_owned()),
        "the uplevel body's own variable should be offered: {ls:?}"
    );
}

// -- TestArrayReadOnlyIndices --------------------------------------------

#[test]
fn array_element_completion_picks_up_read_only_indices() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let src = "proc p {} {\n    set arr(name) hello\n    puts $arr(role)\n    puts $arr(\n}\n";
    lsp.open_ready(&uri, src);
    let ls = labels(&mut lsp, &uri, 3, 14);
    assert!(ls.contains(&"$arr(name)".to_owned()));
    assert!(ls.contains(&"$arr(role)".to_owned()));
}

// -- TestFuzzyFallback -----------------------------------------------------
// The fuzzy fallback only runs when the prefix filter yields nothing, so
// every test here pairs a typo case with the byte-identity guarantee that
// prefix responses never change (see `string_to_prefix_list_is_unchanged`).

/// A typo'd command fragment falls back to fuzzy matching: the item keeps the
/// typed fragment as `filterText` (so client-side subsequence filters don't
/// hide it) and carries a rank-encoding `sortText`.
#[test]
fn fuzzy_command_fallback_offers_lsearch() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    lsp.open_ready(&uri, "lsaerch");
    let items = by_label(&mut lsp, &uri, 0, 7);
    let item = items.get("lsearch").expect("lsearch offered for lsaerch");
    assert_eq!(
        item.get("filterText").and_then(Value::as_str),
        Some("lsaerch")
    );
    assert_eq!(
        item.get("sortText").and_then(Value::as_str),
        Some("F00_lsearch")
    );
}

/// One-character fragments never fall back to fuzzy matching.
#[test]
fn fuzzy_fallback_skips_one_char_fragment() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    lsp.open_ready(&uri, "q");
    assert!(labels(&mut lsp, &uri, 0, 1).is_empty());
}

/// A typo'd switch word offers the real switch with a replace edit covering
/// the typed fragment.
#[test]
fn fuzzy_switch_fallback_offers_nocase() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    lsp.open_ready(&uri, "lsort -ncoase");
    let items = by_label(&mut lsp, &uri, 0, 13);
    let item = items.get("-nocase").expect("-nocase offered for -ncoase");
    assert_eq!(
        item.get("filterText").and_then(Value::as_str),
        Some("-ncoase")
    );
    let edit = item.get("textEdit").expect("replace edit present");
    let range = edit.get("range").expect("range");
    assert_eq!(range["start"]["character"].as_u64(), Some(6));
    assert_eq!(range["end"]["character"].as_u64(), Some(13));
    assert_eq!(edit.get("newText").and_then(Value::as_str), Some("-nocase"));
}

/// A typo'd subcommand offers exactly the intended subcommand.
#[test]
fn fuzzy_subcommand_fallback_offers_length() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    lsp.open_ready(&uri, "string lenght");
    let items = by_label(&mut lsp, &uri, 0, 13);
    assert_eq!(items.len(), 1, "exactly one item; got {:?}", items.keys());
    let item = items.get("length").expect("length offered for lenght");
    assert_eq!(
        item.get("filterText").and_then(Value::as_str),
        Some("lenght")
    );
}

/// Byte-identity guard: a prefix that matches today keeps its exact response —
/// no fuzzy padding, no filterText decoration.
#[test]
fn string_to_prefix_list_is_unchanged() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    lsp.open_ready(&uri, "string to");
    let items = by_label(&mut lsp, &uri, 0, 9);
    assert_eq!(
        items.keys().cloned().collect::<Vec<_>>(),
        vec!["tolower", "totitle", "toupper"],
        "prefix response must be exactly the to* subcommands"
    );
    for (label, item) in &items {
        assert!(
            item.get("filterText").is_none(),
            "{label}: prefix items must not carry filterText"
        );
    }
}

/// A typo'd variable fragment offers the real variable, replacing the whole
/// `$…` token.
#[test]
fn fuzzy_variable_fallback_offers_banana() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    lsp.open_ready(&uri, "set banana 2\nputs $bnaana");
    let items = by_label(&mut lsp, &uri, 1, 12);
    let item = items.get("$banana").expect("$banana offered for $bnaana");
    assert_eq!(
        item.get("filterText").and_then(Value::as_str),
        Some("$bnaana")
    );
    let edit = item.get("textEdit").expect("replace edit present");
    assert_eq!(edit["range"]["start"]["character"].as_u64(), Some(5));
    assert_eq!(edit["range"]["end"]["character"].as_u64(), Some(12));
    assert_eq!(edit.get("newText").and_then(Value::as_str), Some("$banana"));
}

/// A typo'd method word on a known instance offers the method via the
/// response-level fallback…
#[test]
fn fuzzy_method_fallback_offers_bark() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let src =
        "oo::class create Dog {\n    method bark {} { return woof }\n}\nset d [Dog new]\n$d brk";
    lsp.open_ready(&uri, src);
    let items = by_label(&mut lsp, &uri, 4, 6);
    assert!(
        items.contains_key("bark"),
        "bark offered for brk; got {:?}",
        items.keys()
    );
}

/// …but never hijacks a fragment that already prefix-matches something in the
/// assembled response: `xy` still reaches the proc `xyz` fall-through, with no
/// fuzzy method items and no filterText decoration.
#[test]
fn fuzzy_method_fallback_does_not_hijack_prefix_match() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let src = "proc xyz {} { return 1 }\noo::class create Dog {\n    method bark {} { return woof }\n}\nset d [Dog new]\n$d xy";
    lsp.open_ready(&uri, src);
    let items = by_label(&mut lsp, &uri, 5, 5);
    assert!(
        items.contains_key("xyz"),
        "prefix fall-through must survive; got {:?}",
        items.keys()
    );
    assert!(!items.contains_key("bark"), "no fuzzy method items");
    for (label, item) in &items {
        assert!(
            item.get("filterText").is_none(),
            "{label}: prefix items must not carry filterText"
        );
    }
}
