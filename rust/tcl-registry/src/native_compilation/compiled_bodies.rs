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

//! The literal script bodies a compiler enters: their contexts, error notes and the order of compilation steps.

use super::expression_compilation::{
    NativeCompiledExpressionErrorContext, NativeCompiledExpressionOperand,
};
use super::{
    NativeBodyCompilation, NativeCompilationContext, NativeCompilationFailureScope,
    NativeCompilationFrame, NativeCompilationGrammar, NativeCompilationMode,
    NativeCompilationSelection, NativeCompilationSpec, NativeCompilationWordShape, literal_shape,
};
use crate::{InvocationDialect, InvocationWords, SemanticOperationId};
use tcl_dialect::TclVersion;
use tcl_dialect::model::Family;

/// Lexical compiler environment entered for one inline script operand.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum NativeCompiledBodyContext {
    /// Retain the environment already selected by the wrapper descriptor.
    Inherit,
    /// Enter the native compiler's loop exception range.
    Loop,
    /// Enter a protected native try body, handler or finally script.
    ExceptionRange,
    /// Enter a loop range nested inside the compiler's own catch range
    /// (`dict for`/`dict map`).
    ProtectedLoop,
}

impl NativeCompiledBodyContext {
    /// Retain the selected compiler environment while entering one body.
    /// Loop-depth overflow and unknown catch depth cannot invent nesting.
    #[must_use]
    pub fn entered_context(
        self,
        enclosing: NativeCompilationContext,
    ) -> Option<NativeCompilationContext> {
        Some(match self {
            Self::Inherit => enclosing,
            Self::ExceptionRange => enclosing.with_inline_exception_range(),
            Self::Loop => NativeCompilationContext {
                loop_depth: enclosing.loop_depth.checked_add(1)?,
                ..enclosing
            },
            Self::ProtectedLoop => NativeCompilationContext {
                loop_depth: enclosing.loop_depth.checked_add(1)?,
                ..enclosing.with_inline_exception_range()
            },
        })
    }
}

/// Authored context appended after a rejected inline child script.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NativeCompiledBodyErrorContext {
    /// The compiler appends no child-specific annotation.
    None,
    /// An if/elseif branch's then script.
    IfThen,
    /// The conditional's else script.
    IfElse,
    /// The initial command of a for loop.
    ForInitial,
    /// The repeated body of a for loop.
    ForBody,
    /// The loop-end command of a for loop.
    ForNext,
    /// A foreach loop's repeated body.
    ForeachBody,
    /// A while loop's repeated body.
    WhileBody,
}

impl NativeCompiledBodyErrorContext {
    /// Render the selected C Tcl compiler annotation with the child error line.
    /// Missing required line evidence abstains instead of inventing a line.
    #[must_use]
    pub fn note(self, line: Option<u32>) -> Option<String> {
        match self {
            Self::None => Some(String::new()),
            Self::ForInitial => Some("\n    (\"for\" initial command)".into()),
            Self::ForNext => Some("\n    (\"for\" loop-end command)".into()),
            context => {
                let line = line.filter(|line| *line != 0)?;
                let label = match context {
                    Self::IfThen => "\"if\" then script",
                    Self::IfElse => "\"if\" else script",
                    Self::ForBody => "\"for\" body",
                    Self::ForeachBody => "\"foreach\" body",
                    Self::WhileBody => "\"while\" body",
                    _ => unreachable!("fixed annotations already handled"),
                };
                Some(format!("\n    ({label} line {line})"))
            }
        }
    }
}

/// An authored inline script operand, independently of runtime reachability.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NativeCompiledBodyOperand {
    /// Post-head source operand index.
    pub argument: usize,
    /// Child-specific native compiler environment transition.
    pub context: NativeCompiledBodyContext,
    /// Boundary that receives a rejected child compilation.
    pub failure_scope: NativeCompilationFailureScope,
    /// Child-specific compiler error context, independently of runtime frames.
    pub error_context: NativeCompiledBodyErrorContext,
}

impl NativeCompiledBodyOperand {
    /// Apply this operand's lexical compiler transition after the wrapper's
    /// selected body boundary. Overflow cannot invent a loop nesting depth.
    #[must_use]
    pub fn entered_context(
        self,
        enclosing: NativeCompilationContext,
    ) -> Option<NativeCompilationContext> {
        self.context.entered_context(enclosing)
    }
}

/// Compiler traversal selected by a proved native opcode grammar.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NativeCompiledBodies {
    /// Exactly these literal bodies are compiled, in native compiler order.
    Known(Vec<NativeCompiledBodyOperand>),
    /// Retained grammar/proof cannot close the native compiler traversal.
    Unknown,
}

/// One compiler visit, ordered independently of runtime control flow.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NativeCompilationStep {
    /// Recursively compile a literal script operand.
    Body(NativeCompiledBodyOperand),
    /// Compile a literal expression operand.
    Expression(NativeCompiledExpressionOperand),
}

/// Closed native compiler traversal, or a retained uncertainty.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NativeCompilationSteps {
    /// Native compiler visits in their exact order.
    Known(Vec<NativeCompilationStep>),
    /// Available syntax or context does not close compiler traversal.
    Unknown,
}

impl NativeCompilationSpec {
    /// Select compile-time body traversal separately from runtime body flow.
    /// Native if/while pruning uses the shared literal boolean conversion;
    /// expression folding must not hide a body which the native compiler visits.
    ///
    /// A consumer checks each returned literal script before executing any
    /// enclosing chunk effects. `FallbackToGeneric` prevents a rejected catch
    /// child from rejecting its parent: the runtime wrapper compiles that child
    /// separately and captures its error. `Unknown` requires residual compiler
    /// uncertainty; it never licenses successful compilation. This projection
    /// covers script traversal, not expression compiler success.
    #[must_use]
    pub fn compiled_bodies(
        self,
        selection: NativeCompilationSelection,
        words: InvocationWords<'_>,
        shapes: &[NativeCompilationWordShape],
        facts: &crate::InvocationFacts,
        dialect: Option<InvocationDialect>,
    ) -> NativeCompiledBodies {
        use NativeCompilationSelection as Selection;
        if selection == Selection::Unknown {
            return NativeCompiledBodies::Unknown;
        }
        if !matches!(selection, Selection::Inline { .. })
            || self.body != NativeBodyCompilation::Inherit
        {
            return NativeCompiledBodies::Known(Vec::new());
        }
        let Some(dialect) = dialect.filter(|d| d.family() == Some(Family::Tcl)) else {
            return NativeCompiledBodies::Unknown;
        };
        let flow = crate::script_body_flow::script_body_flow(facts);
        if flow == crate::script_body_flow::ScriptBodyFlow::None && facts.arg_roles_complete {
            // A selected compiler with no script operands does not need the
            // length of the argv produced later by runtime expansion.
            return NativeCompiledBodies::Known(Vec::new());
        }
        if words.arguments().exact_argv_len() != Some(shapes.len()) {
            return NativeCompiledBodies::Unknown;
        }
        if self.grammar == NativeCompilationGrammar::Try {
            return crate::native_try::compiled_bodies(words, dialect);
        }
        if flow == crate::script_body_flow::ScriptBodyFlow::None {
            return NativeCompiledBodies::Known(Vec::new());
        }
        if shapes.len() > usize::from(u8::MAX) {
            return NativeCompiledBodies::Unknown;
        }
        let loop_operands = match &flow {
            crate::script_body_flow::ScriptBodyFlow::Loop {
                repeated,
                continued,
                ..
            } => repeated
                .iter()
                .chain(continued)
                .copied()
                .collect::<Vec<_>>(),
            _ => Vec::new(),
        };
        let indices = self.compiled_body_indices(flow, words, shapes, facts, dialect);
        NativeCompiledBodies::Known(
            indices
                .into_iter()
                .filter(|index| {
                    shapes
                        .get(*index)
                        .is_some_and(|shape| literal_shape(*shape))
                })
                .map(|argument| NativeCompiledBodyOperand {
                    argument,
                    error_context: self.body_error_context(argument, facts),
                    context: if loop_operands.contains(&argument) {
                        NativeCompiledBodyContext::Loop
                    } else {
                        NativeCompiledBodyContext::Inherit
                    },
                    failure_scope: if self.grammar == NativeCompilationGrammar::Catch {
                        NativeCompilationFailureScope::FallbackToGeneric
                    } else {
                        NativeCompilationFailureScope::EnclosingChunk
                    },
                })
                .collect(),
        )
    }

    /// Order literal script and expression compiler visits. These visits are
    /// distinct from runtime reachability and preserve native failure priority.
    #[must_use]
    pub fn compilation_steps(
        self,
        selection: NativeCompilationSelection,
        words: InvocationWords<'_>,
        shapes: &[NativeCompilationWordShape],
        facts: &crate::InvocationFacts,
        dialect: Option<InvocationDialect>,
    ) -> NativeCompilationSteps {
        use NativeCompilationStep::{Body, Expression};
        use NativeCompiledExpressionErrorContext as ErrorContext;
        if !matches!(selection, NativeCompilationSelection::Inline { .. }) {
            return if selection == NativeCompilationSelection::Unknown {
                NativeCompilationSteps::Unknown
            } else {
                NativeCompilationSteps::Known(Vec::new())
            };
        }
        let Some(dialect) = dialect.filter(|dialect| dialect.family() == Some(Family::Tcl)) else {
            return NativeCompilationSteps::Unknown;
        };
        let expression = |argument, error_context| {
            Expression(NativeCompiledExpressionOperand {
                argument,
                error_context,
            })
        };
        if self.grammar == NativeCompilationGrammar::Expression {
            return NativeCompilationSteps::Known(
                if shapes.len() == 1 && literal_shape(shapes[0]) {
                    vec![expression(0, ErrorContext::None)]
                } else {
                    Vec::new()
                },
            );
        }
        let NativeCompiledBodies::Known(bodies) =
            self.compiled_bodies(selection, words, shapes, facts, Some(dialect))
        else {
            return NativeCompilationSteps::Unknown;
        };
        if self.grammar == NativeCompilationGrammar::Conditional {
            return conditional_steps(words, facts, dialect, &bodies);
        }
        let mut steps = bodies.iter().copied().map(Body).collect::<Vec<_>>();
        if self.grammar == NativeCompilationGrammar::ForLoop {
            steps.push(expression(1, ErrorContext::ForTest));
        } else if self.operation
            == SemanticOperationId::StructuredLowering(crate::hooks::LoweringHookId::While)
            && !bodies.is_empty()
            && words
                .arguments()
                .literal_at(0)
                .and_then(|text| tcl_syntax::boolean::truthiness_with(text, dialect.numbers))
                .is_none()
        {
            steps.push(expression(0, ErrorContext::WhileTest));
        }
        NativeCompilationSteps::Known(steps)
    }

    fn body_error_context(
        self,
        argument: usize,
        facts: &crate::InvocationFacts,
    ) -> NativeCompiledBodyErrorContext {
        use NativeCompiledBodyErrorContext as ErrorContext;
        match self.grammar {
            NativeCompilationGrammar::Conditional => {
                let else_body = match crate::script_body_flow::script_body_flow(facts) {
                    crate::script_body_flow::ScriptBodyFlow::Conditional(clauses) => clauses
                        .iter()
                        .any(|(condition, body)| condition.is_none() && *body == argument),
                    _ => false,
                };
                if else_body {
                    ErrorContext::IfElse
                } else {
                    ErrorContext::IfThen
                }
            }
            NativeCompilationGrammar::ForLoop => match argument {
                0 => ErrorContext::ForInitial,
                2 => ErrorContext::ForNext,
                _ => ErrorContext::ForBody,
            },
            NativeCompilationGrammar::Foreach => ErrorContext::ForeachBody,
            NativeCompilationGrammar::WhileLoop => ErrorContext::WhileBody,
            _ => ErrorContext::None,
        }
    }

    fn compiled_body_indices(
        self,
        flow: crate::script_body_flow::ScriptBodyFlow,
        words: InvocationWords<'_>,
        shapes: &[NativeCompilationWordShape],
        facts: &crate::InvocationFacts,
        dialect: InvocationDialect,
    ) -> Vec<usize> {
        match (self.grammar, flow) {
            (NativeCompilationGrammar::Foreach, _) => vec![shapes.len().saturating_sub(1)],
            (NativeCompilationGrammar::Catch, _) => vec![0],
            (_, crate::script_body_flow::ScriptBodyFlow::Conditional(branches)) => {
                compiled_conditional_bodies(&branches, words, dialect)
            }
            (
                _,
                crate::script_body_flow::ScriptBodyFlow::Loop {
                    initial,
                    repeated,
                    conditions,
                    ..
                },
            ) => {
                if self.operation
                    == SemanticOperationId::StructuredLowering(crate::hooks::LoweringHookId::While)
                    && conditions
                        .first()
                        .and_then(|index| words.arguments().literal_at(*index))
                        .and_then(|text| {
                            tcl_syntax::boolean::truthiness_with(text, dialect.numbers)
                        })
                        == Some(false)
                {
                    Vec::new()
                } else {
                    initial.into_iter().chain(repeated).collect()
                }
            }
            _ => facts
                .arg_roles
                .iter()
                .filter_map(|(index, role)| {
                    (*role == crate::ArgRole::Body)
                        .then_some(facts.argument_offset + usize::from(*index))
                })
                .collect(),
        }
    }

    /// Resolve an inline-capable body's actual compiler entry. A native
    /// fallback evaluates a separate object even in the caller's variable frame.
    #[must_use]
    pub fn body_context_for_operand(
        self,
        dialect: Option<InvocationDialect>,
        enclosing: NativeCompilationContext,
        selection: NativeCompilationSelection,
        shape: NativeCompilationWordShape,
    ) -> NativeCompilationContext {
        if self.body == NativeBodyCompilation::Inherit {
            if matches!(selection, NativeCompilationSelection::Inline { operation, .. } if operation == self.operation)
                && literal_shape(shape)
            {
                return if matches!(
                    self.grammar,
                    NativeCompilationGrammar::Catch | NativeCompilationGrammar::Dictionary { .. }
                ) {
                    enclosing.with_inline_exception_range()
                } else {
                    enclosing
                };
            }
            return Self {
                body: NativeBodyCompilation::ScriptObject,
                ..self
            }
            .body_context(dialect, enclosing);
        }
        self.body_context(dialect, enclosing)
    }

    /// Project a selected script operand's compiler context, including native
    /// protected Try ranges. Handler execution and source mapping remain separate.
    #[must_use]
    pub fn body_context_for_invocation_operand(
        self,
        enclosing: NativeCompilationContext,
        selection: NativeCompilationSelection,
        shape: NativeCompilationWordShape,
        arguments: crate::InvocationArguments<'_>,
        operand: usize,
    ) -> NativeCompilationContext {
        let entered =
            self.body_context_for_operand(arguments.dialect(), enclosing, selection, shape);
        if self.grammar == NativeCompilationGrammar::Try
            && matches!(selection, NativeCompilationSelection::Inline { .. })
            && literal_shape(shape)
            && crate::selected_try_control_invocation(arguments, 0).is_some_and(|layout| {
                !layout.clauses.is_empty()
                    && (layout.body_index == operand
                        || layout
                            .clauses
                            .iter()
                            .any(|clause| !clause.fallthrough && clause.body_index == operand))
            })
        {
            entered.with_inline_exception_range()
        } else {
            entered
        }
    }

    /// Enter this native implementation's script without conflating compiler
    /// capabilities with the caller's variable frame.
    #[must_use]
    pub fn body_context(
        self,
        dialect: Option<InvocationDialect>,
        _enclosing: NativeCompilationContext,
    ) -> NativeCompilationContext {
        let Some(dialect) = dialect else {
            return NativeCompilationContext::default();
        };
        if dialect.family() != Some(Family::Tcl) {
            return if dialect.family() == Some(Family::Jim) {
                NativeCompilationContext {
                    mode: NativeCompilationMode::Direct,
                    frame: NativeCompilationFrame::ScriptCode,
                    loop_depth: 0,
                    catch_depth: Some(0),
                }
            } else {
                NativeCompilationContext::default()
            };
        }
        match self.body {
            NativeBodyCompilation::Inherit => NativeCompilationContext::default(),
            NativeBodyCompilation::ProcedureObject => NativeCompilationContext {
                mode: NativeCompilationMode::BytecodeObject,
                frame: NativeCompilationFrame::ProcedureCode,
                loop_depth: 0,
                catch_depth: Some(0),
            },
            NativeBodyCompilation::ScriptObject => NativeCompilationContext {
                mode: NativeCompilationMode::BytecodeObject,
                frame: NativeCompilationFrame::ScriptCode,
                loop_depth: 0,
                catch_depth: Some(0),
            },
            NativeBodyCompilation::Direct => NativeCompilationContext {
                mode: NativeCompilationMode::Direct,
                frame: NativeCompilationFrame::ScriptCode,
                loop_depth: 0,
                catch_depth: Some(0),
            },
            NativeBodyCompilation::Uplevel => match dialect.tcl_version {
                Some(TclVersion::V8_4) => NativeCompilationContext {
                    mode: NativeCompilationMode::Direct,
                    frame: NativeCompilationFrame::ScriptCode,
                    loop_depth: 0,
                    catch_depth: Some(0),
                },
                Some(_) => NativeCompilationContext {
                    mode: NativeCompilationMode::BytecodeObject,
                    frame: NativeCompilationFrame::ScriptCode,
                    loop_depth: 0,
                    catch_depth: Some(0),
                },
                None => NativeCompilationContext::default(),
            },
        }
    }
}

fn compiled_conditional_bodies(
    branches: &[(Option<usize>, usize)],
    words: InvocationWords<'_>,
    dialect: InvocationDialect,
) -> Vec<usize> {
    let mut compiled = Vec::new();
    for &(condition, body) in branches {
        let boolean = condition
            .and_then(|index| words.arguments().literal_at(index))
            .and_then(|text| tcl_syntax::boolean::truthiness_with(text, dialect.numbers));
        if boolean != Some(false) {
            compiled.push(body);
        }
        if condition.is_none() || boolean == Some(true) {
            break;
        }
    }
    compiled
}

fn conditional_steps(
    words: InvocationWords<'_>,
    facts: &crate::InvocationFacts,
    dialect: InvocationDialect,
    bodies: &[NativeCompiledBodyOperand],
) -> NativeCompilationSteps {
    let crate::script_body_flow::ScriptBodyFlow::Conditional(clauses) =
        crate::script_body_flow::script_body_flow(facts)
    else {
        return NativeCompilationSteps::Unknown;
    };
    let mut steps = Vec::new();
    for (condition, body) in clauses {
        let truth = condition.and_then(|argument| {
            words
                .arguments()
                .literal_at(argument)
                .and_then(|text| tcl_syntax::boolean::truthiness_with(text, dialect.numbers))
        });
        if let Some(argument) = condition.filter(|_| truth.is_none()) {
            steps.push(NativeCompilationStep::Expression(
                NativeCompiledExpressionOperand {
                    argument,
                    error_context: NativeCompiledExpressionErrorContext::IfTest,
                },
            ));
        }
        if let Some(body) = bodies.iter().find(|operand| operand.argument == body) {
            steps.push(NativeCompilationStep::Body(*body));
        }
        if truth == Some(true) || condition.is_none() {
            break;
        }
    }
    NativeCompilationSteps::Known(steps)
}
