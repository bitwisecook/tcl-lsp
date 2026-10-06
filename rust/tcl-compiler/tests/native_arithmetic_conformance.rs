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

//! Constant folds compared with independent native integer towers.

use tcl_compiler::tcl_expr_eval::{Env, FoldPolicy, eval_tcl_expr_with_policy, format_tcl_value};
use tcl_dialect::DialectProfile;
use tcl_registry::InvocationDialect;
use tcl_test_support::{available_tclshs, locate_jimsh, require_jimsh, run_script};

// Boolean boundaries test the numeric result without relying on Tcl 8.4's
// build-dependent signed-minimum formatter. Successful folds must preserve
// the whole observable; error/undefined-native cases must remain unfurled.
const CASES: &[&str] = &[
    "9223372036854775807 * 2",
    "((-9223372036854775807 - 1) - 1) == 9223372036854775807",
    "(9223372036854775807 + 1) < 0",
    "-(-9223372036854775807 - 1) < 0",
    "((-9223372036854775807 - 1) / -1) < 0",
    "18446744073709551615 + 0",
    "18446744073709551616 + 0",
    "(1 << 63) < 0",
    "1 << 64",
    "8 >> 64",
    "-1 >> 64",
    "2 ** 64",
    "-7 / 3",
    "7 / -3",
    "-7 % 3",
    "7 % -3",
    "~9223372036854775808",
    "18446744073709551615 == -1",
    "~18446744073709551616",
    "1.5 + 2.5",
];

fn compare(path: &std::path::Path, profile: &'static DialectProfile) {
    let name = profile.name;
    let policy = FoldPolicy::for_profile(None, Some(profile))
        .with_invocation_dialect(InvocationDialect::of_profile(profile));
    let mut proved = 0;
    for expression in CASES {
        let node = tcl_syntax::expr::parser::parse_expr_for_profile(expression, Some(profile));
        let script = format!("puts [list [catch {{expr {{{expression}}}}} value] $value]\n");
        let native = run_script(path, script.as_bytes())
            .expect("native expression")
            .strict_text()
            .expect("clean native observation");
        if let Some(value) = eval_tcl_expr_with_policy(&node, &Env::new(), policy) {
            let value = format_tcl_value(&value);
            assert_eq!(native, format!("0 {value}"), "{name}: {expression}");
            proved += 1;
        } else {
            assert!(
                native.starts_with("1 "),
                "unexplained declined fold: {name}: {expression}: {native}"
            );
        }
    }
    assert!(proved >= 16, "insufficient native proof coverage: {name}");
}

#[test]
fn folds_match_actual_c_tcl_all_releases() {
    let references = available_tclshs();
    assert!(
        !references.is_empty(),
        "configure C Tcl reference interpreters"
    );
    for reference in references {
        compare(
            &reference.path,
            DialectProfile::find(reference.version.dialect_profile_name()).unwrap(),
        );
    }
}

#[test]
fn folds_match_actual_current_jim() {
    let reference = if std::env::var_os("TCL_LSP_REQUIRE_JIM_ORACLE").is_some() {
        Some(require_jimsh().expect("required current Jim"))
    } else {
        locate_jimsh().expect("validated Jim override")
    };
    if let Some(reference) = reference {
        let profile = DialectProfile::projected_from_point(
            "jim",
            &[],
            "Jim",
            tcl_dialect::model::DialectPoint::canonical(tcl_dialect::model::Release::JIM_0_84),
        )
        .intern();
        compare(&reference.path, profile);
    }
}
