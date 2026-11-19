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
use tcl_dialect::{DoubleFormat, DoubleStringPolicy, TclVersion};

#[test]
fn native_double_strings_match_all_six_engines_and_precision_states() {
    let vectors = include_str!("../../../tcl-syntax/tests/data/double_string_vectors.txt");
    let mut count = 0;
    for row in vectors
        .lines()
        .filter(|row| !row.starts_with('#') && !row.is_empty())
    {
        let parts: Vec<_> = row.split('\t').collect();
        let policy = match parts[0] {
            "8.4" => DoubleStringPolicy::for_tcl_version(TclVersion::V8_4),
            "8.5" => DoubleStringPolicy::for_tcl_version(TclVersion::V8_5),
            "8.6" => DoubleStringPolicy::for_tcl_version(TclVersion::V8_6),
            "9.0" => DoubleStringPolicy::for_tcl_version(TclVersion::V9_0),
            "9.1" => DoubleStringPolicy::for_tcl_version(TclVersion::V9_1),
            "jim" => DoubleStringPolicy::JimTwelve,
            engine => panic!("unmeasured engine {engine}"),
        };
        let precision: u8 = parts[1].parse().unwrap();
        let precision = if policy == DoubleStringPolicy::Tcl84Precision && precision == 0 {
            12 // Tcl 8.4 rejects zero and retains its initial precision.
        } else {
            precision
        };
        let value: f64 = parts[2].parse::<f64>().unwrap() + 0.0;
        assert_eq!(
            format_double_selected(value, policy.format(precision).unwrap()),
            parts[3],
            "{row}"
        );
        count += 1;
    }
    assert_eq!(count, 384);
}

#[test]
fn precision_variable_parsing_preserves_native_release_grammar() {
    let old = DoubleStringPolicy::Tcl84Precision;
    let new = DoubleStringPolicy::TclPrecision;
    for (value, legacy, modern) in [
        ("0", None, Some(0)),
        ("17", Some(17), Some(17)),
        ("18", None, None),
        ("-1", None, None),
        ("1.5", None, None),
        ("+4", Some(4), Some(4)),
        (" 4", Some(4), Some(4)),
        ("4 ", None, Some(4)),
        ("010", Some(10), Some(8)),
        ("017", Some(17), Some(15)),
        ("0x10", None, Some(16)),
        ("4294967300", Some(4), None),
        ("-4294967292", Some(4), Some(4)),
        ("18446744073709551620", None, None),
    ] {
        assert_eq!(parse_double_precision(old, value), legacy, "8.4 {value}");
        assert_eq!(
            parse_double_precision(new, value),
            modern,
            "8.5/8.6 {value}"
        );
    }
    assert_eq!(
        DoubleStringPolicy::JimTwelve.constant_format(),
        Some(DoubleFormat::General(12))
    );
    assert_eq!(old.constant_format(), None);
    assert_eq!(new.constant_format(), None);
}
