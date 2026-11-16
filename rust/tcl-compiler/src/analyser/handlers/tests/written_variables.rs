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

//! `set`, `incr`, loop variables and the other variables a command writes.

use super::*;

#[test]
fn a_whole_word_loop_variable_records_its_declaration_occurrence() {
    // TP — `foreach name …` declares `name` at the word itself, as
    // `catch {…} res` declares `res`; the read alone left navigation
    // from `$name` with no declaration to answer.
    let source = "foreach name {a b} {puts $name}\nproc p {} {foreach x {1 2} {puts $x}}\n";
    for dialect in ["tcl8.4", "tcl8.6", "tcl9.0", "tcl"] {
        let result = Analyser::new().analyse(source, dialect);
        for name in ["name", "x"] {
            let at = u32::try_from(source.find(&format!("foreach {name}")).unwrap() + 8).unwrap();
            let span = Span::new(at, at + u32::try_from(name.len()).unwrap());
            assert!(
                result
                    .original_variable_symbols
                    .iter()
                    .any(|occurrence| occurrence.span() == span),
                "{dialect}: {name}"
            );
        }
    }
}

#[test]
fn handle_set_defines_variable() {
    let a = analysed("set x 1\n");
    assert!(a.result.global_scope.variables.contains_key("x"));
    assert!(a.result.global_scope.variables["x"].warn_if_unused);
}

#[test]
fn handle_set_tracks_single_token_literal_value() {
    let a = analysed("set x hello\n");
    assert_eq!(
        a.const_strings
            .get(&Vec::new())
            .and_then(|values| values.get("x"))
            .map(|(value, _)| value.as_str()),
        Some("hello")
    );
}

#[test]
fn handle_set_tracks_braced_string_value() {
    let a = analysed("set x {hello world}\n");
    assert_eq!(
        a.const_strings
            .get(&Vec::new())
            .and_then(|values| values.get("x"))
            .map(|(value, _)| value.as_str()),
        Some("hello world")
    );
}

#[test]
fn handle_set_clears_const_string_for_interpolated_value() {
    // Re-assigning a value no literal spells clears the constant.
    let a = analysed("set x old\nset x [gets stdin]\n");
    assert_eq!(
        a.const_strings
            .get(&Vec::new())
            .and_then(|values| values.get("x"))
            .map(|(value, _)| value.as_str()),
        None
    );
}

#[test]
fn handle_set_no_value_records_read_not_definition() {
    // ``set x`` (one-arg form) is a *read*, not a definition — Tcl
    // returns the current value of ``x`` — recorded by the walk's
    // `VarRead`-role reference pass, with no const-string tracking.
    let src = "set x 1\nset x\n";
    let mut a = Analyser::new();
    let r = a.analyse(src, "tcl8.6");
    let read = u32::try_from(src.rfind('x').expect("the read")).expect("an offset");
    assert_eq!(
        r.global_scope.variables["x"].references,
        vec![span(read, read + 1)],
    );
    let mut read_only = Analyser::new();
    dispatch_tokens(
        &mut read_only,
        &[
            ("set", esc_tok(span(0, 3)), true),
            ("x", esc_tok(span(4, 5)), true),
        ],
    );
    assert_eq!(
        read_only
            .const_strings
            .get(&Vec::new())
            .and_then(|values| values.get("x"))
            .map(|(value, _)| value.as_str()),
        None
    );
}

/// A literal value word is recorded as Tcl reads it: the
/// `CellWrite` evaluation cooks a bare or quoted token, so `set p
/// "a\\d"` holds `a\d` — tclsh 8.4.20 to 9.1b0 print `a\d`.
#[test]
fn a_literal_value_word_is_recorded_as_tcl_reads_it() {
    let a = analysed("set p \"a\\\\d\"\n");
    assert_eq!(
        a.const_strings
            .get(&Vec::new())
            .and_then(|values| values.get("p"))
            .map(|(value, _)| value.as_str()),
        Some("a\\d")
    );
}

/// A computed target defines no variable: `set $n 1` writes
/// the variable `n` names — tclsh 8.4.20 to 9.1b0 leave `n` as it was
/// — so the role binding's static-name rule binds nothing.
#[test]
fn a_computed_set_target_defines_no_variable() {
    let r = Analyser::new().analyse("set $n 1\n", "tcl8.6");
    assert!(!r.global_scope.variables.contains_key("n"));
}

#[test]
fn handle_set_no_value_undefined_var_is_silent() {
    // ``set x`` on an undefined variable is still a read; the read
    // recording silently no-ops when the name isn't in scope, so no
    // spurious binding lands.
    let r = Analyser::new().analyse("set x\n", "tcl8.6");
    assert!(!r.global_scope.variables.contains_key("x"));
}

// `incr` / `append` / `lappend` / `dict` — bound by role

#[test]
fn incr_binds_its_target_by_role() {
    let a = analysed("incr counter\n");
    assert!(a.result.global_scope.variables.contains_key("counter"));
    // A read-modify-write target is read before it is written, so the
    // binding is a side effect, never a "set but unused" write.
    assert!(!a.result.global_scope.variables["counter"].warn_if_unused);
}

#[test]
fn handle_incr_with_amount() {
    let a = analysed("incr counter 5\n");
    assert!(a.result.global_scope.variables.contains_key("counter"));
    assert!(!a.result.global_scope.variables.contains_key("5"));
}

#[test]
fn handle_incr_no_args_no_op() {
    let mut a = Analyser::new();
    dispatch_words(&mut a, &["incr"]);
    assert!(a.result.global_scope.variables.is_empty());
}

#[test]
fn a_written_array_element_binds_its_array_and_a_computed_name_binds_nothing() {
    // `incr hits($word)` writes an element of `hits` whatever the key;
    // `append $name x` and `incr $count` write a variable no static name
    // spells.
    let a = analysed(
        "set word k\nincr hits($word)\nset name v\nappend $name x\n\
             set count n\nset n 1\nincr $count\nlappend items x\n",
    );
    assert!(a.result.global_scope.variables.contains_key("hits"));
    assert!(a.result.global_scope.variables.contains_key("items"));
    assert!(!a.result.global_scope.variables.contains_key("v"));
    assert!(!a.result.global_scope.variables.contains_key("$name"));
    assert!(!a.result.global_scope.variables.contains_key("$count"));
}

#[test]
fn dict_for_and_dict_update_bind_their_loop_variables_by_role() {
    let a = analysed("set d {}\ndict for {k v} $d {}\ndict update d key w {}\n");
    for name in ["k", "v", "w"] {
        assert!(
            a.result.global_scope.variables.contains_key(name),
            "{name}: {:?}",
            a.result.global_scope.variables.keys().collect::<Vec<_>>()
        );
    }
    // Neither the key word nor the dictionary variable is a loop variable.
    assert!(!a.result.global_scope.variables.contains_key("key"));
}

// stray-close-bracket recovery

#[test]
fn analyse_top_level_repairs_stray_close_bracket() {
    // ``set x string]`` is a typo for ``set x [string ...]``.
    // The recovery should rewrite the third argv entry into
    // a virtual ``CMD`` token before dispatch so the var
    // record is registered with the recovered shape.
    let mut a = crate::analyser::Analyser::new();
    let r = a.analyse("set x string]", "tcl");
    // ``x`` ends up in scope as a single-arg ``set`` (a var
    // read), not as a two-arg ``set`` with the broken text
    // — recovery yields the synthetic ``[string]`` command
    // word so dispatch sees the intended shape.
    assert!(r.global_scope.variables.contains_key("x"));
}
