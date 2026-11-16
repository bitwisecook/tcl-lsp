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

//! Expression operands a compiler visits: math-function resolution, compiler steps and validation.

use super::NativeCompilationFailure;
use tcl_dialect::TclVersion;

/// A literal expression visited by the native compiler.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NativeCompiledExpressionOperand {
    /// Post-head source operand index.
    pub argument: usize,
    /// Authored note before the enclosing command's compiler frame.
    pub error_context: NativeCompiledExpressionErrorContext,
}

/// Compiler context for a rejected expression operand.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NativeCompiledExpressionErrorContext {
    /// Direct expression compilation adds no operand-specific note.
    None,
    /// Conditional test expression.
    IfTest,
    /// While-loop test expression.
    WhileTest,
    /// For-loop test expression.
    ForTest,
}

impl NativeCompiledExpressionErrorContext {
    /// Render the independently authored compiler annotation.
    #[must_use]
    pub fn note(self) -> &'static str {
        match self {
            Self::None => "",
            Self::IfTest => "\n    (\"if\" test expression)",
            Self::WhileTest => "\n    (\"while\" test expression)",
            Self::ForTest => "\n    (\"for\" test expression)",
        }
    }
}

/// Result of compile-time expression validation, not runtime evaluation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NativeExpressionCompilation {
    /// This operand cannot cause a syntax rejection at this compiler boundary.
    Accepted,
    /// The selected native compiler definitely rejects the expression.
    Rejected(NativeCompilationFailure),
    /// Parser recovery, unsupported syntax or context retains uncertainty.
    Unknown,
}

/// Actual fixed-function registration selected by a native compiler provider.
/// Catalogue visibility alone cannot establish one of these outcomes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum NativeMathFunctionResolution {
    /// A retained registration proves the compiler's exact argument count.
    Known {
        /// Exact argument count required by this registration.
        arity: usize,
    },
    /// A closed observed registration table proves the name absent.
    Absent,
    /// Function registration, identity or argument count was not retained.
    Unknown,
}

/// One ordered C 8.4 expression compiler visit.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NativeExpressionCompilerStep {
    /// Compile this bracketed script before proceeding to the next visit.
    Script(tcl_lexer::Span),
    /// Stop compilation with the exact retained rejection, if presented.
    Failure(NativeCompilationFailure),
    /// Stop proof traversal because a compiler lookup or grammar is unresolved.
    Unknown,
}

/// Build compiler visits in native order, including all runtime-lazy branches.
/// Function lookup precedes its required argument visits; argument-count errors
/// follow those visits. Extra arguments are not compiled after a count error.
pub(crate) fn expression_compiler_visits<Text: tcl_syntax::expr::ast::ExprText>(
    node: &tcl_syntax::expr::ast::ExprNode<Text>,
    scripts: &[tcl_lexer::Span],
    lookup: &mut impl FnMut(&str) -> NativeMathFunctionResolution,
    visits: &mut Vec<NativeExpressionCompilerStep>,
) -> bool {
    use NativeExpressionCompilerStep as Step;
    use tcl_syntax::expr::ast::ExprNode;
    match node {
        ExprNode::Call { function, args, .. } => {
            let Some(function) = function.try_text() else {
                visits.push(Step::Unknown);
                return false;
            };
            let arity = match lookup(function) {
                NativeMathFunctionResolution::Known { arity } => arity,
                NativeMathFunctionResolution::Absent => {
                    visits.push(Step::Failure(native_math_compile_error(format!(
                        "unknown math function \"{function}\""
                    ))));
                    return false;
                }
                NativeMathFunctionResolution::Unknown => {
                    visits.push(Step::Unknown);
                    return false;
                }
            };
            for argument in args.iter().take(arity) {
                if !expression_compiler_visits(argument, scripts, lookup, visits) {
                    return false;
                }
            }
            if args.len() != arity {
                let count = if args.len() < arity { "few" } else { "many" };
                visits.push(Step::Failure(native_math_compile_error(format!(
                    "too {count} arguments for math function"
                ))));
                return false;
            }
        }
        ExprNode::Binary { left, right, .. } => {
            return expression_compiler_visits(left, scripts, lookup, visits)
                && expression_compiler_visits(right, scripts, lookup, visits);
        }
        ExprNode::Unary { operand, .. } => {
            return expression_compiler_visits(operand, scripts, lookup, visits);
        }
        ExprNode::Ternary {
            condition,
            true_branch,
            false_branch,
        } => {
            return expression_compiler_visits(condition, scripts, lookup, visits)
                && expression_compiler_visits(true_branch, scripts, lookup, visits)
                && expression_compiler_visits(false_branch, scripts, lookup, visits);
        }
        ExprNode::String { start, end, .. }
        | ExprNode::Var { start, end, .. }
        | ExprNode::Command { start, end, .. } => {
            visits.extend(
                scripts
                    .iter()
                    .filter(|span| span.start() >= *start && span.end().saturating_sub(1) <= *end)
                    .copied()
                    .map(Step::Script),
            );
        }
        ExprNode::Literal { .. } => {}
        ExprNode::CompiledWord { .. } | ExprNode::Raw { .. } => {
            visits.push(Step::Unknown);
            return false;
        }
    }
    true
}

pub(crate) fn native_math_compile_error(message: String) -> NativeCompilationFailure {
    NativeCompilationFailure {
        message: Some(message),
        error_code: Some("NONE".into()),
        error_info: None,
    }
}

impl NativeCompiledExpressionOperand {
    /// Whether this native compiler must inspect an expression value before
    /// the surrounding chunk executes. Unknown policies retain an obligation.
    #[must_use]
    pub fn requires_source_preflight(
        self,
        context: &tcl_syntax::expr::parser::ExprParseContext,
    ) -> bool {
        use tcl_syntax::expr::parser::NativeExprSyntax;
        !matches!(context.native_syntax, NativeExprSyntax::Jim084)
            && !matches!(context.native_syntax, NativeExprSyntax::Tcl(version) if version > TclVersion::V8_4)
    }

    /// Resolve actual fixed functions and script operands in compiler order.
    /// The supplied lookup must use a retained native registration table,
    /// independently of the command catalogue. Missing proof stays explicit.
    #[must_use]
    pub fn compiler_steps(
        self,
        source: &str,
        context: &tcl_syntax::expr::parser::ExprParseContext,
        mut lookup: impl FnMut(&str) -> NativeMathFunctionResolution,
    ) -> Vec<NativeExpressionCompilerStep> {
        use NativeExpressionCompilerStep as Step;
        use tcl_syntax::expr::parser::{CheckedExprParse, NativeExprSyntax};
        match context.native_syntax {
            NativeExprSyntax::Tcl(TclVersion::V8_4) => {}
            NativeExprSyntax::Unknown => return vec![Step::Unknown],
            NativeExprSyntax::Tcl(_) | NativeExprSyntax::Jim084 => return Vec::new(),
        }
        match tcl_syntax::expr::parser::parse_expr_checked_with_context(source, context) {
            CheckedExprParse::Parsed(node) => {
                let Some(scripts) = self.compiled_substitutions(source, context) else {
                    return vec![Step::Unknown];
                };
                let mut visits = Vec::new();
                expression_compiler_visits(&node, &scripts, &mut lookup, &mut visits);
                visits
            }
            CheckedExprParse::Unsupported(_) => vec![Step::Unknown],
            CheckedExprParse::ProvedSyntaxFailure(_) => match self.validate(source, context) {
                NativeExpressionCompilation::Rejected(error) => vec![Step::Failure(error)],
                _ => vec![Step::Unknown],
            },
        }
    }

    /// Native compiler command visits inside an accepted literal expression.
    /// C 8.4 compiles all branches, including runtime-lazy operands. Later
    /// releases cannot raise a chunk-entry script compilation error here.
    #[must_use]
    pub fn compiled_substitutions(
        self,
        source: &str,
        context: &tcl_syntax::expr::parser::ExprParseContext,
    ) -> Option<Vec<tcl_lexer::Span>> {
        use tcl_syntax::expr::parser::NativeExprSyntax;
        match context.native_syntax {
            NativeExprSyntax::Tcl(TclVersion::V8_4) => {
                tcl_syntax::expr::substitution::command_substitutions_in_checked_expression(
                    source, context,
                )
            }
            NativeExprSyntax::Unknown => None,
            NativeExprSyntax::Tcl(_) | NativeExprSyntax::Jim084 => Some(Vec::new()),
        }
    }

    /// Check native compile-time syntax with all selected lexical axes intact.
    /// Modern C releases emit runtime syntax errors; C 8.4 rejects the chunk.
    /// C 8.4 also resolves math functions and validates their arity while
    /// compiling. A parsed function call remains unknown until an independent
    /// function-table identity and arity contract proves that compiler lookup.
    #[must_use]
    pub fn validate(
        self,
        source: &str,
        context: &tcl_syntax::expr::parser::ExprParseContext,
    ) -> NativeExpressionCompilation {
        use tcl_syntax::expr::parser::{CheckedExprParse, NativeExprSyntax};
        match context.native_syntax {
            NativeExprSyntax::Tcl(version) if version != TclVersion::V8_4 => {
                return NativeExpressionCompilation::Accepted;
            }
            NativeExprSyntax::Jim084 => return NativeExpressionCompilation::Accepted,
            NativeExprSyntax::Unknown => return NativeExpressionCompilation::Unknown,
            NativeExprSyntax::Tcl(_) => {}
        }
        match tcl_syntax::expr::parser::parse_expr_checked_with_context(source, context) {
            CheckedExprParse::Parsed(node) => {
                if node.function_calls().is_empty() {
                    NativeExpressionCompilation::Accepted
                } else {
                    NativeExpressionCompilation::Unknown
                }
            }
            CheckedExprParse::Unsupported(_) => NativeExpressionCompilation::Unknown,
            CheckedExprParse::ProvedSyntaxFailure(error) => {
                let diagnostic = error.native_diagnostic_with_context(source, context);
                NativeExpressionCompilation::Rejected(NativeCompilationFailure {
                    message: diagnostic
                        .as_ref()
                        .map(|diagnostic| diagnostic.message.clone()),
                    error_code: diagnostic.and_then(|diagnostic| diagnostic.error_code),
                    error_info: None,
                })
            }
        }
    }
}
