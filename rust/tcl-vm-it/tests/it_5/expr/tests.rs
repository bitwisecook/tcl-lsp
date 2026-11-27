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

//! `expr` results per release, compiled and evaluated from a variable.

use tcl_runtime_api::Code;

fn native_case(profile: &'static tcl_dialect::DialectProfile, expression: &str, expected: &str) {
    for dynamic in [false, true] {
        let mut vm = crate::fixture::interpreter(profile);
        let script = if dynamic {
            format!("set native_expression {{{expression}}}; expr $native_expression")
        } else {
            format!("expr {{{expression}}}")
        };
        let result = vm.eval_source(&script).unwrap();
        assert_eq!(
            result.code,
            Code::Ok,
            "{}: {script}: {}",
            profile.display_name,
            result.result.to_str()
        );
        assert_eq!(
            &*result.result.to_str(),
            expected,
            "{}: {script}",
            profile.display_name
        );
    }
}

fn arithmetic_profiles() -> Vec<&'static tcl_dialect::DialectProfile> {
    let mut profiles: Vec<_> = tcl_dialect::TclVersion::ALL
        .iter()
        .map(|version| tcl_dialect::DialectProfile::find(version.dialect_profile_name()).unwrap())
        .collect();
    profiles.push(tcl_registry::model::ingress::resolve_environment("jim").analyser_profile());
    profiles
}

#[test]
fn native_integer_arithmetic_uses_the_selected_engine_on_both_paths() {
    let cases = [
        (
            "9223372036854775807 * 2",
            "-2",
            "18446744073709551614",
            "-2",
        ),
        ("(9223372036854775807 + 1) < 0", "1", "0", "1"),
        ("-(-9223372036854775807 - 1) < 0", "1", "0", "1"),
        ("((-9223372036854775807 - 1) / -1) < 0", "1", "0", "1"),
        (
            "18446744073709551615 + 0",
            "-1",
            "18446744073709551615",
            "-1",
        ),
        ("1 << 64", "0", "18446744073709551616", "1"),
        ("8 >> 64", "0", "0", "8"),
        ("-1 >> 64", "-1", "-1", "-1"),
        ("-7 / 3", "-3", "-3", "-3"),
        ("7 / -3", "-3", "-3", "-3"),
        ("-7 % 3", "2", "2", "2"),
        ("7 % -3", "-2", "-2", "-2"),
    ];
    for profile in arithmetic_profiles() {
        for (expression, tcl84, modern, jim) in cases {
            let expected = match tcl_native_model::InvocationDialect::of_profile(profile)
                .arithmetic()
                .unwrap()
            {
                tcl_dialect::NativeArithmetic::Tcl84Wide => tcl84,
                tcl_dialect::NativeArithmetic::TclBignum => modern,
                tcl_dialect::NativeArithmetic::JimWide => jim,
            };
            native_case(profile, expression, expected);
        }
    }
}

#[test]
fn native_literal_and_final_string_conversion_are_distinct() {
    for profile in arithmetic_profiles() {
        let policy = tcl_native_model::InvocationDialect::of_profile(profile)
            .arithmetic()
            .unwrap();
        native_case(
            profile,
            "\"001\"",
            if policy.normalizes_expression_result() {
                "1"
            } else {
                "001"
            },
        );
        native_case(profile, "1e3", "1000.0");
        native_case(profile, "\"18446744073709551616\"", "18446744073709551616");
        if policy == tcl_dialect::NativeArithmetic::Tcl84Wide {
            let mut vm = crate::fixture::interpreter(profile);
            let result = vm.eval_source("expr {18446744073709551616 + 0}").unwrap();
            assert_eq!(result.code, Code::Error);
            assert_eq!(
                &*result.result.to_str(),
                "integer value too large to represent"
            );
        } else {
            native_case(
                profile,
                "18446744073709551616 + 0",
                if policy == tcl_dialect::NativeArithmetic::JimWide {
                    "-1"
                } else {
                    "18446744073709551616"
                },
            );
        }
    }
}
