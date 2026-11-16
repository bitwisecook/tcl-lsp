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

fn selected(version: tcl_dialect::TclVersion) -> FoldPolicy {
    FoldPolicy::default()
        .with_invocation_dialect(tcl_registry::InvocationDialect::for_version(version))
}

#[test]
fn floating_string_folds_require_proved_native_precision() {
    let value = TclValue::Float(1.0 / 3.0);
    for version in [
        tcl_dialect::TclVersion::V8_4,
        tcl_dialect::TclVersion::V8_5,
        tcl_dialect::TclVersion::V8_6,
    ] {
        assert_eq!(
            format_tcl_value_with_policy(&value, selected(version)),
            None
        );
    }
    assert_eq!(
        format_tcl_value_with_policy(&value, selected(tcl_dialect::TclVersion::V9_1)),
        Some("0.3333333333333333".to_owned())
    );
    let jim = tcl_registry::InvocationDialect::of_point(
        tcl_dialect::model::DialectPoint::canonical(tcl_dialect::model::Release::JIM_0_84),
    );
    assert_eq!(
        format_tcl_value_with_policy(&value, FoldPolicy::default().with_invocation_dialect(jim)),
        Some("0.333333333333".to_owned())
    );
}

#[test]
fn mutable_precision_blocks_string_comparison_but_keeps_numeric_comparison() {
    let policy = selected(tcl_dialect::TclVersion::V8_6);
    let string = crate::expr_parser::parse_expr(r#"(1.0/3) eq "0.3333333333333333""#, None);
    let numeric = crate::expr_parser::parse_expr("(1.0/3) < 1", None);
    assert_eq!(
        eval_tcl_expr_with_policy(&string, &Env::new(), policy),
        None
    );
    assert_eq!(
        eval_tcl_expr_with_policy(&numeric, &Env::new(), policy),
        Some(TclValue::Int(1))
    );
}
