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

//! Tests for the LSP inlay-hints provider.
//! Verifies `inlay_hints` emits inferred-type annotations (`: int`) and
//! call-site parameter-name labels (`a:`), gated/positioned correctly.
//!
//! C-Tcl proof: the inferred type is Tcl's actual value type — `set x 42` is
//! an integer (tclsh `string is integer 42` → 1); and `add 1 2` binds the
//! params a=1, b=2 (tclsh `proc add {a b} {...}; add 1 2` → "1,2"), so the
//! call-site labels `a:`/`b:` land on the right argument columns.

use tcl_compiler::analyser::Analyser;
use tcl_lsp_core::definition::LspRange;
use tcl_lsp_core::inlay_hints::{InlayHint, InlayHintKind, inlay_hints};
use tcl_registry::registry_for_dialect;

const FULL: LspRange = LspRange {
    start_line: 0,
    start_character: 0,
    end_line: 999,
    end_character: 0,
};

fn hints_range(source: &str, range: LspRange) -> Vec<InlayHint> {
    let registry = registry_for_dialect("tcl8.6");
    let mut analyser = Analyser::new();
    let analysis = analyser.analyse(source, "tcl8.6");
    inlay_hints(
        source,
        tcl_dialect::DialectProfile::by_name("tcl8.6"),
        range,
        Some(&analysis),
        Some(registry),
        true,
        true,
    )
}

fn hints(source: &str) -> Vec<InlayHint> {
    hints_range(source, FULL)
}

#[test]
fn integer_type_hint() {
    // `set x 42` → a `: int` Type hint just after `set x` (col 5).
    let h = hints("set x 42\n");
    assert!(
        h.iter().any(|x| x.label == ": int"
            && x.position_character == 5
            && x.kind == InlayHintKind::Type),
        "expected `: int` at col 5; got {:?}",
        h.iter()
            .map(|x| (&x.label, x.position_character))
            .collect::<Vec<_>>()
    );
}

#[test]
fn unknown_type_does_not_crash() {
    // Unknown command return type may or may not produce hints — just no panic.
    let _ = hints("set x [some_command]\n");
}

#[test]
fn empty_file_has_no_hints() {
    assert!(hints("").is_empty());
}

#[test]
fn hint_kinds_are_type_or_parameter() {
    let h = hints("set x 42\n");
    for x in &h {
        assert!(matches!(
            x.kind,
            InlayHintKind::Type | InlayHintKind::Parameter
        ));
    }
    assert!(
        h.iter()
            .any(|x| x.kind == InlayHintKind::Type && x.label.contains("int"))
    );
}

#[test]
fn user_proc_param_name_hints_at_call_site() {
    // tclsh: `add 1 2` binds a=1, b=2 → labels `a:`@col4, `b:`@col6 on line 1.
    let h = hints("proc add {a b} { expr {$a + $b} }\nadd 1 2\n");
    let call: Vec<(String, u32)> = h
        .into_iter()
        .filter(|x| x.kind == InlayHintKind::Parameter && x.position_line == 1)
        .map(|x| (x.label, x.position_character))
        .collect();
    assert!(call.contains(&("a:".to_string(), 4)), "got {call:?}");
    assert!(call.contains(&("b:".to_string(), 6)), "got {call:?}");
}

#[test]
fn negative_number_is_positional_not_flag() {
    // `-1` is no command's option, so it gets a positional parameter hint
    // rather than being skipped as a flag.
    let source = "set s hello\nstring index $s -1\n";
    let neg_one_col = u32::try_from(source.lines().nth(1).unwrap().find("-1").unwrap()).unwrap();
    let on_neg = hints(source)
        .iter()
        .filter(|x| x.kind == InlayHintKind::Parameter && x.position_line == 1)
        .any(|x| x.position_character == neg_one_col);
    assert!(on_neg, "expected a positional hint on the -1 token");
}

#[test]
fn declared_flag_is_not_mislabelled() {
    // `lsort -integer $l` — `-integer` is a declared flag; no hint label should
    // itself start with `-`.
    let h = hints("set l {3 1 2}\nlsort -integer $l\n");
    for x in &h {
        if x.kind == InlayHintKind::Parameter {
            assert!(!x.label.starts_with('-'), "flag mislabelled: {}", x.label);
        }
    }
}

#[test]
fn range_filtering_limits_to_requested_lines() {
    let source = "set x 42\nset y 99\n";
    let narrow = LspRange {
        start_line: 0,
        start_character: 0,
        end_line: 0,
        end_character: 100,
    };
    for x in hints_range(source, narrow) {
        assert_eq!(x.position_line, 0, "hint outside requested range");
    }
}

#[test]
fn foreach_multi_var_hints_are_not_clumped() {
    // Type hints for `foreach {a b c}` appear after each var, at distinct cols.
    let h = hints("foreach {a b c} {1 2 3} { puts $a$b$c }\n");
    let type_cols: Vec<u32> = h
        .iter()
        .filter(|x| x.kind == InlayHintKind::Type)
        .map(|x| x.position_character)
        .collect();
    // Distinct columns (not all clumped at one position) when present.
    if type_cols.len() > 1 {
        let mut uniq = type_cols.clone();
        uniq.sort_unstable();
        uniq.dedup();
        assert_eq!(
            uniq.len(),
            type_cols.len(),
            "multi-var hints clumped: {type_cols:?}"
        );
    }
}
