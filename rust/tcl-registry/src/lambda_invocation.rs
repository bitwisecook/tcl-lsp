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

//! Registry-owned anonymous-procedure invocation grammar.

use crate::InvocationArguments;
use crate::body_execution::BodyOperand;

/// Decoded lambda operands, retaining their effective argument and list roles.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LambdaInvocation {
    /// Effective argument containing the lambda list.
    pub lambda_argument: usize,
    /// Formal parameter-list value, interpreted by the native parameter owner.
    pub parameters: String,
    /// Selected script value; list decoding is complete before it executes.
    pub body: String,
    /// Namespace value interpreted relative to the global namespace.
    /// An omitted namespace is the global namespace itself.
    pub namespace: String,
    /// Structural script selector used to preserve actual authored source spans.
    pub body_operand: BodyOperand,
    /// First effective argument supplied to the anonymous procedure's parameters.
    pub call_arguments_from: usize,
}

/// Exact native list selection, an invalid lambda, or an unresolved value/shape.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LambdaInvocationSelection {
    /// The outer list has the native required two or three elements.
    Selected(LambdaInvocation),
    /// Known argument count or lambda list is invalid before body evaluation.
    Invalid,
    /// No exact native grammar, lambda value, or expanded argv shape is available.
    Unknown,
}

/// Decode one lambda using the actual engine's shared list grammar.
/// Formal validation and argument binding belong to the shared parameter/frame
/// owner; this selector never interprets trailing values as script source.
#[must_use]
pub fn select_lambda_invocation(
    arguments: InvocationArguments<'_>,
    lambda_argument: usize,
) -> LambdaInvocationSelection {
    use LambdaInvocationSelection::{Invalid, Selected, Unknown};
    let Some(dialect) = arguments.dialect() else {
        return Unknown;
    };
    if !matches!(
        dialect.family(),
        Some(tcl_dialect::model::Family::Tcl | tcl_dialect::model::Family::Jim)
    ) {
        return Unknown;
    }
    let Some(count) = arguments.exact_argv_len() else {
        return Unknown;
    };
    if count <= lambda_argument {
        return Invalid;
    }
    let Some(value) = arguments.literal_at(lambda_argument) else {
        return Unknown;
    };
    let Ok(elements) = dialect.word_values.split_list(value) else {
        return Invalid;
    };
    if !matches!(elements.len(), 2 | 3) {
        return Invalid;
    }
    Selected(LambdaInvocation {
        lambda_argument,
        parameters: elements[0].to_string(),
        body: elements[1].to_string(),
        namespace: elements
            .get(2)
            .map_or_else(|| "::".to_owned(), ToString::to_string),
        body_operand: BodyOperand {
            argument: lambda_argument,
            list_element: Some(1),
        },
        call_arguments_from: lambda_argument + 1,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use tcl_dialect::TclVersion;

    #[test]
    fn lambda_selection_keeps_body_namespace_and_ordinary_call_arguments() {
        let words = ["{x} {proc helper {} {return $x}} n", "VALUE"];
        let selected = select_lambda_invocation(
            InvocationArguments::literals(&words)
                .with_dialect(crate::InvocationDialect::for_version(TclVersion::V8_6)),
            0,
        );
        let LambdaInvocationSelection::Selected(selected) = selected else {
            panic!("known native lambda");
        };
        assert_eq!(selected.parameters, "x");
        assert_eq!(selected.namespace, "n");
        assert_eq!(selected.body_operand.list_element, Some(1));
        assert_eq!(selected.call_arguments_from, 1);
        assert!(selected.body.contains("return $x"));
    }

    #[test]
    fn lambda_selection_never_guesses_engine_or_expansion_shape() {
        assert_eq!(
            select_lambda_invocation(InvocationArguments::literals(&["{} {}"]), 0),
            LambdaInvocationSelection::Unknown
        );
        let dialect = crate::InvocationDialect::for_version(TclVersion::V8_6);
        assert_eq!(
            select_lambda_invocation(
                InvocationArguments::literals(&["one"]).with_dialect(dialect),
                0
            ),
            LambdaInvocationSelection::Invalid
        );
        let words = [
            crate::InvocationWord::Literal("{} {}"),
            crate::InvocationWord::Expanded,
        ];
        assert_eq!(
            select_lambda_invocation(
                InvocationArguments::structured(&words).with_dialect(dialect),
                0
            ),
            LambdaInvocationSelection::Unknown
        );
    }
}
