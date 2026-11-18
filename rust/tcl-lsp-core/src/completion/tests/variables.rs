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

//! Variable-name and array-element candidates.

use super::*;

#[test]
fn array_element_completion_does_not_eat_trailing_text_when_unclosed() {
    // `$arr(k more stuff` with no `)` — accepting `$arr(key)` must replace
    // only up to the cursor, not to end-of-line, so ` more stuff` survives
    // (issue 181).
    let src = "set arr(key) 1\nset v $arr(k more stuff\n";
    let analysis = analyse(src);
    let registry = CommandRegistry::build_default();
    // Line 1, cursor just after `k`: `set v $arr(k` → char col 12.
    let items = completions(
        src,
        1,
        12,
        &analysis,
        Some(&registry),
        None,
        tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile(),
    );
    let key = items
        .iter()
        .find(|i| i.label == "$arr(key)")
        .expect("array-element completion offered");
    let edit = key.text_edit.as_ref().expect("has a replace edit");
    assert_eq!(
        edit.end_char, 12,
        "replace range must stop at the cursor, not run to EOL"
    );
}

#[test]
fn array_element_completion_swallows_existing_close_paren() {
    // With a closing `)`, the replace range extends through it so the old
    // index is fully replaced.
    let src = "set arr(key) 1\nset v $arr(k)\n";
    let analysis = analyse(src);
    let registry = CommandRegistry::build_default();
    let items = completions(
        src,
        1,
        12,
        &analysis,
        Some(&registry),
        None,
        tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile(),
    );
    let key = items
        .iter()
        .find(|i| i.label == "$arr(key)")
        .expect("array-element completion offered");
    let edit = key.text_edit.as_ref().expect("has a replace edit");
    assert_eq!(edit.end_char, 13, "replace range must cover the `)`");
}

#[test]
fn variable_completion_lists_globals_alphabetically() {
    let src = "set apple 1\nset banana 2\nset $\n";
    let analysis = analyse(src);
    // Cursor after `$` on the third line.
    let items = completions(
        src,
        2,
        5,
        &analysis,
        None,
        None,
        tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile(),
    );
    assert!(!items.is_empty(), "expected variable completions");
    assert_eq!(items[0].kind, CompletionKind::Variable);
    let labels: Vec<&str> = items.iter().map(|i| i.label.as_str()).collect();
    // Alphabetical order.
    let mut sorted = labels.clone();
    sorted.sort_unstable();
    assert_eq!(labels, sorted);
    assert!(labels.contains(&"$apple"));
    assert!(labels.contains(&"$banana"));
}

#[test]
fn variable_completion_filters_by_partial() {
    let src = "set apple 1\nset banana 2\nset $b\n";
    let analysis = analyse(src);
    let items = completions(
        src,
        2,
        6,
        &analysis,
        None,
        None,
        tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile(),
    );
    let labels: Vec<&str> = items.iter().map(|i| i.label.as_str()).collect();
    assert_eq!(labels, vec!["$banana"]);
}

#[test]
fn fuzzy_variable_offers_banana_for_bnaana() {
    let src = "set apple 1\nset banana 2\nputs $bnaana\n";
    let analysis = analyse(src);
    let items = completions(
        src,
        2,
        12,
        &analysis,
        None,
        None,
        tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile(),
    );
    let labels: Vec<&str> = items.iter().map(|i| i.label.as_str()).collect();
    assert_eq!(labels, vec!["$banana"], "typo'd variable fragment");
    // Sigil included, matching the replaced `$…` range.
    assert_eq!(items[0].filter_text.as_deref(), Some("$bnaana"));
    let edit = items[0].text_edit.as_ref().expect("variable replace edit");
    assert_eq!(edit.new_text, "$banana");
}

#[test]
fn fuzzy_array_element_offers_key_for_kye() {
    let src = "set arr(key) 1\nset v $arr(kye\n";
    let analysis = analyse(src);
    let items = completions(
        src,
        1,
        14,
        &analysis,
        None,
        None,
        tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile(),
    );
    let labels: Vec<&str> = items.iter().map(|i| i.label.as_str()).collect();
    assert_eq!(labels, vec!["$arr(key)"], "typo'd array index");
    assert_eq!(items[0].filter_text.as_deref(), Some("$arr(kye"));
}

/// Labels of `$`-completion at `(line, character)` under Tcl 8.6.
fn dollar_labels(src: &str, line: u32, character: u32) -> Vec<String> {
    let analysis = analyse_for(src, "tcl8.6");
    let registry = CommandRegistry::build_default();
    completions(
        src,
        line,
        character,
        &analysis,
        Some(&registry),
        None,
        tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile(),
    )
    .into_iter()
    .map(|item| item.label)
    .collect()
}

// tclsh 8.4.20, 8.5.19, 8.6.18, 9.0.4 and 9.1.0: `proc q {} {set local 1;
// uplevel 1 {set local}}; q` raises `can't read "local"` (the body runs in
// the caller's frame), while `uplevel 0 {set local}` reads the proc's own.

#[test]
fn uplevel_hash_zero_body_completes_global_variables_only() {
    let src = "set ::globalvar 9\nproc p {} {\n    set local_var 1\n    uplevel #0 {\n        set inside_var 99\n        puts $\n    }\n}\n";
    let labels = dollar_labels(src, 5, 14);
    assert!(labels.contains(&"$::globalvar".to_owned()), "{labels:?}");
    assert!(labels.contains(&"$inside_var".to_owned()), "{labels:?}");
    assert!(!labels.contains(&"$local_var".to_owned()), "{labels:?}");
}

#[test]
fn uplevel_one_body_offers_its_own_variables_not_the_procs_locals() {
    let src = "proc p {} {\n    set local_var 1\n    uplevel 1 {\n        set body_var 5\n        puts $\n    }\n}\n";
    let labels = dollar_labels(src, 4, 14);
    assert!(!labels.contains(&"$local_var".to_owned()), "{labels:?}");
    assert!(labels.contains(&"$body_var".to_owned()), "{labels:?}");
}

#[test]
fn try_on_error_bindings_complete_inside_the_handler() {
    let src = "proc p {} {\n    try {\n        error boom\n    } on error {try_msg try_opts} {\n        puts $\n    }\n}\n";
    let labels = dollar_labels(src, 4, 14);
    assert!(labels.contains(&"$try_msg".to_owned()), "{labels:?}");
    assert!(labels.contains(&"$try_opts".to_owned()), "{labels:?}");
}

#[test]
fn an_upvar_alias_completes_after_its_declaration() {
    let src = "proc inner {var} {\n    upvar 1 $var alias\n    puts $\n}\n";
    assert!(dollar_labels(src, 2, 10).contains(&"$alias".to_owned()));
}

#[test]
fn dict_with_keys_of_a_literal_complete_in_its_body() {
    let src =
        "proc p {} {\n    set d {name alice age 30}\n    dict with d {\n        puts $\n    }\n}\n";
    let labels = dollar_labels(src, 3, 14);
    assert!(labels.contains(&"$name".to_owned()), "{labels:?}");
    assert!(labels.contains(&"$age".to_owned()), "{labels:?}");
}
