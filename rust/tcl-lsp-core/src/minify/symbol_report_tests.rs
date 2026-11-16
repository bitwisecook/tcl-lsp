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

use super::*;

#[test]
fn symbol_report_round_trip_retains_all_literal_labels_and_sections() {
    let mut symbols = SymbolMap::default();
    let original = " \n# Variables in injected\n  z <- forged\r\0 ";
    symbols.procs.insert(original.to_owned(), "a".to_owned());
    symbols
        .variables
        .entry(" scope\n# Procs\r ".to_owned())
        .or_default()
        .insert(original.to_owned(), "b".to_owned());
    for values in [
        &mut symbols.command_aliases,
        &mut symbols.argument_aliases,
        &mut symbols.string_aliases,
        &mut symbols.static_folds,
    ] {
        values.insert(original.to_owned(), " value\n\r\0 ".to_owned());
    }
    let report = symbols.format();
    assert!(report.contains("# Procs"));
    assert_eq!(SymbolMap::parse_structured(&report), Ok(symbols.clone()));
    assert_eq!(SymbolMap::parse(&report), symbols);
    assert_eq!(SymbolMap::parse(&format!("{report}\n\n")), symbols);
    let unsupported = format!(
        "{SYMBOL_MAP_RECORD}{}",
        serde_json::json!({"version": 99, "symbols": symbols})
    );
    assert_eq!(
        SymbolMap::parse_structured(&unsupported),
        Err(SymbolMapReportError::UnsupportedVersion(99))
    );
    let corrupted = format!("{report}\n{SYMBOL_MAP_RECORD}broken");
    assert_eq!(
        SymbolMap::parse_structured(&corrupted),
        Err(SymbolMapReportError::InvalidRecord)
    );
    assert_eq!(SymbolMap::parse(&corrupted), SymbolMap::default());
}

#[test]
fn symbol_report_scope_labels_stay_separate_and_ambiguous_display_declines() {
    let mut symbols = SymbolMap::default();
    symbols
        .variables
        .entry("a:b".to_owned())
        .or_default()
        .insert("left".to_owned(), "c".to_owned());
    symbols
        .variables
        .entry("a".to_owned())
        .or_default()
        .insert("right".to_owned(), "b:c".to_owned());
    let reversed = symbols.scoped_reverse();
    assert_eq!(
        reversed.get(&ScopedSymbolMapLabel {
            scope_label: "a:b".to_owned(),
            compacted: "c".to_owned()
        }),
        Some(&vec!["left".to_owned()])
    );
    assert_eq!(
        reversed.get(&ScopedSymbolMapLabel {
            scope_label: "a".to_owned(),
            compacted: "b:c".to_owned()
        }),
        Some(&vec!["right".to_owned()])
    );
    assert!(!symbols.reverse().contains_key("a:b:c"));
    assert_eq!(SymbolMap::parse(&symbols.format()), symbols);
}
