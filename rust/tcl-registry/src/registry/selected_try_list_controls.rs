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
fn selected_literal_list_values_do_not_repeat_source_substitution_checks() {
    let dialect = crate::InvocationDialect::for_version(tcl_dialect::TclVersion::V8_6);
    let values = [
        "body",
        "trap",
        "$literal [inert]",
        "$message [options]",
        "handler",
    ];
    let words = values.map(crate::InvocationWord::Literal);
    let arguments = crate::InvocationArguments::structured(&words).with_dialect(dialect);
    let selected = selected_try_control_invocation(arguments, 0).expect("closed literal values");
    assert_eq!(selected.clauses.len(), 1);
    assert_eq!(selected.clauses[0].kind, TryClauseKind::Trap);
    assert!(
        selected_try_control_invocation(
            crate::InvocationArguments::literals(&values).with_dialect(dialect),
            0,
        )
        .is_none()
    );
    let dynamic = [
        crate::InvocationWord::Literal("body"),
        crate::InvocationWord::Literal("trap"),
        crate::InvocationWord::Dynamic,
        crate::InvocationWord::Literal("m o"),
        crate::InvocationWord::Literal("handler"),
    ];
    assert!(
        selected_try_control_invocation(
            crate::InvocationArguments::structured(&dynamic).with_dialect(dialect),
            0
        )
        .is_none()
    );
    assert!(
        selected_try_control_invocation(crate::InvocationArguments::structured(&words), 0)
            .is_none()
    );
}

#[test]
fn selected_try_lists_obey_the_supplied_strict_or_lenient_grammar() {
    let mut dialect = crate::InvocationDialect::for_version(tcl_dialect::TclVersion::V8_6);
    let words = [
        crate::InvocationWord::Literal("body"),
        crate::InvocationWord::Literal("trap"),
        crate::InvocationWord::Literal("{unfinished"),
        crate::InvocationWord::Literal("m o"),
        crate::InvocationWord::Literal("handler"),
    ];
    assert!(
        selected_try_control_invocation(
            crate::InvocationArguments::structured(&words).with_dialect(dialect),
            0
        )
        .is_none()
    );
    dialect.lexer_grammar.list_parse = tcl_dialect::ListParse::Lenient;
    assert!(
        selected_try_control_invocation(
            crate::InvocationArguments::structured(&words).with_dialect(dialect),
            0
        )
        .is_some()
    );
    let overfull = [
        crate::InvocationWord::Literal("body"),
        crate::InvocationWord::Literal("on"),
        crate::InvocationWord::Literal("error"),
        crate::InvocationWord::Literal("a b c"),
        crate::InvocationWord::Literal("handler"),
    ];
    assert!(
        selected_try_control_invocation(
            crate::InvocationArguments::structured(&overfull).with_dialect(dialect),
            0
        )
        .is_none()
    );
}
