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

//! Explicit authored Tcl 8.4 function preparation, separate from native tables.

use tcl_runtime_api::expression_policy::{
    AuthoredMathFunctionProvider, ExpressionEvaluationPolicy,
};
use tcl_syntax::expr::parser::{CheckedExprParse, NativeExprSyntaxDiagnostic};

/// Logical preparation failure; no variant supplies native compiler admission.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AuthoredFunctionPreparationError {
    /// Checked authored syntax or fixed-function argument contract rejected.
    Rejected(NativeExprSyntaxDiagnostic<Vec<u8>>),
    /// The installed logical policy or original source cannot support this query.
    Unavailable,
}

/// Select only an explicitly installed, internally consistent authored provider.
/// Native expression policies and parsing/numeric capabilities alone decline.
#[must_use]
pub fn provider(policy: &ExpressionEvaluationPolicy) -> Option<AuthoredMathFunctionProvider> {
    let selected = policy.authored_functions?;
    let mut expected = crate::native_expression_program::authored_expression_evaluation_policy(
        policy.profile.profile(),
        crate::invocation_words::LogicalExpressionParseProvider::Tcl84CoreSimulation,
        policy.numeric_simulation,
    )?;
    expected.authored_functions = Some(selected);
    (expected == *policy && policy.numeric_simulation.is_some()).then_some(selected)
}

/// Exact fixed arity of a function supplied by the authored core implementation.
/// A missing name proves only absence in this explicit simulation surface.
#[must_use]
pub fn arity(selected: AuthoredMathFunctionProvider, name: &str) -> Option<usize> {
    match selected {
        AuthoredMathFunctionProvider::Tcl84Core => {
            let spec = tcl_syntax::expr::mathfunc::spec(name)?;
            (spec.since == tcl_syntax::expr::mathfunc::MathFuncSince::Tcl84
                && spec.arity.max == Some(spec.arity.min))
            .then_some(usize::from(spec.arity.min))
        }
    }
}

/// Validate original logical syntax and all fixed-function names/arity before
/// operand execution. Pure compiler geometry is reused without acquiring its
/// native registration, child compiler, literal or result-header permissions.
///
/// # Errors
/// Returns a logical rejection or typed unavailability for foreign policy/source.
pub fn prepare(
    source: &[u8],
    policy: &ExpressionEvaluationPolicy,
) -> Result<tcl_syntax::expr::NativeExprNode, AuthoredFunctionPreparationError> {
    use crate::native_compilation::{
        NativeExpressionCompilerStep, NativeMathFunctionResolution, expression_compiler_visits,
    };
    let selected = provider(policy).ok_or(AuthoredFunctionPreparationError::Unavailable)?;
    let tree = match tcl_syntax::expr::parser::parse_expr_bytes_checked_with_context(
        source,
        &policy.context,
    ) {
        CheckedExprParse::Parsed(tree) => tree,
        CheckedExprParse::ProvedSyntaxFailure(error) => {
            let diagnostic = error
                .native_diagnostic_bytes_with_context(source, &policy.context)
                .ok_or(AuthoredFunctionPreparationError::Unavailable)?;
            return Err(AuthoredFunctionPreparationError::Rejected(diagnostic));
        }
        CheckedExprParse::Unsupported(_) => {
            return Err(AuthoredFunctionPreparationError::Unavailable);
        }
    };
    let mut visits = Vec::new();
    // Reuse only the pure function traversal. An empty script inventory cannot
    // issue child compiler visits or convert this authored lookup to native proof.
    expression_compiler_visits(
        &tree,
        &[],
        &mut |name| {
            arity(selected, name).map_or(NativeMathFunctionResolution::Absent, |arity| {
                NativeMathFunctionResolution::Known { arity }
            })
        },
        &mut visits,
    );
    // Without script visits, the traversal records only its first refusal.
    if let Some(step) = visits.into_iter().next() {
        match step {
            NativeExpressionCompilerStep::Failure(failure) => {
                let message = failure
                    .message
                    .ok_or(AuthoredFunctionPreparationError::Unavailable)?;
                return Err(AuthoredFunctionPreparationError::Rejected(
                    NativeExprSyntaxDiagnostic {
                        message: message.into_bytes(),
                        error_code: failure.error_code.map(String::into_bytes),
                    },
                ));
            }
            NativeExpressionCompilerStep::Unknown => {
                return Err(AuthoredFunctionPreparationError::Unavailable);
            }
            NativeExpressionCompilerStep::Script(_) => unreachable!("no script inventory"),
        }
    }
    Ok(tree)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn installed() -> ExpressionEvaluationPolicy {
        let mut policy = crate::native_expression_program::authored_expression_evaluation_policy(
            tcl_dialect::DialectProfile::irules(),
            crate::invocation_words::LogicalExpressionParseProvider::Tcl84CoreSimulation,
            Some(
                tcl_syntax::logical_numeric_simulation::AuthoredLogicalNumericSimulation::Tcl84Core,
            ),
        )
        .unwrap();
        policy.authored_functions = Some(AuthoredMathFunctionProvider::Tcl84Core);
        policy
    }

    fn physical_program() -> crate::native_expression_program::NativeExpressionProgram {
        let dialect = crate::InvocationDialect::for_version(tcl_dialect::TclVersion::V9_0);
        let image = tcl_lexer::SourceImage::native(b"expr {abs(077)}".as_slice());
        let command = tcl_lexer::native_script_words_in(
            image.clone(),
            tcl_lexer::Span::new(0, image.len().try_into().unwrap()),
            tcl_lexer::LexerConfig::from_grammar(dialect.lexer_grammar),
        )
        .unwrap()
        .commands
        .remove(0);
        let words = crate::native_compiler_words::NativeCompilerWords::capture(
            &command.words,
            dialect.native_source_string_protocol().unwrap(),
        )
        .unwrap();
        crate::native_expression_program::native_expression_instruction(&words, 1, dialect)
            .unwrap()
            .program
            .unwrap()
    }

    #[test]
    fn authored_fixed_functions_have_separate_closed_lookup_and_source_emission() {
        let mut policy = installed();
        assert!(provider(&policy).is_some());
        assert_eq!(
            arity(AuthoredMathFunctionProvider::Tcl84Core, "abs"),
            Some(1)
        );
        assert_eq!(
            arity(AuthoredMathFunctionProvider::Tcl84Core, "entier"),
            None
        );
        assert_eq!(
            crate::native_expression_program::expression_function_dispatch(
                Some(&policy),
                crate::InvocationDialect::for_version(tcl_dialect::TclVersion::V9_0)
            ),
            None
        );
        assert!(prepare(b"abs(077)", &policy).is_ok());
        let program = physical_program();
        let physical = crate::InvocationDialect::for_version(tcl_dialect::TclVersion::V9_0);
        assert_eq!(
            crate::native_expression_program::expression_program_emission_for_policy(
                &program,
                Some(&policy),
                physical,
            ),
            crate::native_expression_program::ExpressionProgramEmission::AuthoredSource,
        );
        assert_eq!(
            program.context.native_syntax,
            tcl_syntax::expr::parser::NativeExprSyntax::Tcl(tcl_dialect::TclVersion::V9_0),
        );
        for source in [b"pow([set effect ENTERED])".as_slice(), b"0 && future(1)"] {
            assert!(matches!(
                prepare(source, &policy),
                Err(AuthoredFunctionPreparationError::Rejected(_))
            ));
        }
        policy.authored_functions = None;
        assert_eq!(provider(&policy), None);
        assert_eq!(
            crate::native_expression_program::expression_program_emission_for_policy(
                &program,
                Some(&policy),
                physical,
            ),
            crate::native_expression_program::ExpressionProgramEmission::Unavailable,
        );
        assert!(matches!(
            prepare(b"abs(1)", &policy),
            Err(AuthoredFunctionPreparationError::Unavailable)
        ));
    }
}
