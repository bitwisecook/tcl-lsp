// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Native script compilation boundaries and operation selection before argv.
//!
//! These descriptors supplement runtime dispatch; a terminal runtime command
//! target cannot prove that an interpreter previously compiled an operation.

use crate::{InvocationDialect, InvocationWords, SemanticOperationId};
use tcl_dialect::TclVersion;
use tcl_dialect::model::Family;

/// How the native interpreter evaluates this script object.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum NativeCompilationMode {
    /// Parse/evaluate commands directly, selecting their targets after argv.
    Direct,
    /// Compile a Tcl script object, retaining native operation selection.
    BytecodeObject,
    /// Entry does not prove either execution protocol.
    #[default]
    Unknown,
}

/// Compilation capability, independently of the script's physical variable frame.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum NativeCompilationFrame {
    /// A procedure compiler owns a local-variable table.
    ProcedureCode,
    /// A script compiler has no procedure local-variable table.
    ScriptCode,
    /// Compilation environment has not been proved.
    #[default]
    Unknown,
}

/// Point at which an interpreter validates the command implementing an opcode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum NativeCompilationGuard {
    /// C Tcl 8.4 selects hooks against the compilation chunk entry table.
    ChunkEntry,
    /// Later C cores validate the command before this command's argv evaluation.
    BeforeArguments,
}

/// Source shape retained before any argument was substituted.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum NativeCompilationWordShape {
    /// A bare substitution-free source word.
    Literal,
    /// A quoted substitution-free `SIMPLE_WORD`, retaining its source delimiter.
    QuotedLiteral,
    /// Text and backslash components only; its value is static but it is not `SIMPLE_WORD`.
    BackslashLiteral,
    /// A braced substitution-free source word whose script can be compiled inline.
    BracedLiteral,
    /// A word with normal substitutions, yielding one argv entry.
    Substituted,
    /// A word expanding a runtime list into argv entries.
    Expanded,
    /// Native source shape was not retained.
    Opaque,
}

impl NativeCompilationWordShape {
    /// Whether this original head can select a native command compiler.
    /// C8.6 added static backslash-word simplification at the command-head boundary.
    #[must_use]
    pub fn compiler_head(self, dialect: Option<InvocationDialect>) -> Option<bool> {
        match self {
            Self::Literal | Self::QuotedLiteral | Self::BracedLiteral => Some(true),
            Self::BackslashLiteral => {
                let dialect = dialect?;
                match dialect.family()? {
                    Family::Tcl => Some(dialect.tcl_version? >= TclVersion::V8_6),
                    Family::Jim => Some(false),
                    _ => None,
                }
            }
            Self::Substituted | Self::Expanded | Self::Opaque => Some(false),
        }
    }
}

/// Native compiler context supplied by the script-entry owner.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct NativeCompilationContext {
    /// Entry evaluation protocol.
    pub mode: NativeCompilationMode,
    /// Whether compiler-local variable slots are available.
    pub frame: NativeCompilationFrame,
    /// Enclosing native loop depth, independent of runtime command nesting.
    pub loop_depth: u32,
    /// Enclosing inline catch exception ranges; absent compiler evidence abstains.
    pub catch_depth: Option<u32>,
}

impl NativeCompilationContext {
    /// Enter one inline exception range without inventing an unknown depth.
    /// Overflow retains unknown exception nesting instead of wrapping.
    #[must_use]
    pub fn with_inline_exception_range(self) -> Self {
        Self {
            catch_depth: self.catch_depth.and_then(|depth| depth.checked_add(1)),
            ..self
        }
    }
}

/// Stack and usage protocol of a compiler-selected command name.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum NativeNamedInvocationProtocol {
    /// Invoke the private name with its ordinary operand vector.
    Direct,
    /// Retain original ensemble words and rewrite them at invocation.
    EnsembleRewrite,
}

/// Result of an authored compiler grammar; runtime lookup remains independent.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum NativeCompilationSelection {
    /// A native operation was selected under an independently proved guard.
    Inline {
        /// Operation emitted by the native compiler.
        operation: SemanticOperationId,
        /// Required command-table validation point.
        guard: NativeCompilationGuard,
    },
    /// A compiler fixes a private command name, then resolves its handler after argv.
    NamedInvocation {
        /// Original ensemble mapping and compiler implementation used at compilation.
        lookup: &'static NativeCompilerImplementationLookup,
        /// Written post-head operands consumed by ensemble selection.
        arguments_from: usize,
        /// Native argument and wrong-argument presentation layout.
        protocol: NativeNamedInvocationProtocol,
    },
    /// The native compiler rejects syntax before evaluating argv.
    CompileError,
    /// The compiler deliberately emits a generic late invocation.
    Generic,
    /// The retained syntax/context does not select an authored outcome.
    Unknown,
}

/// Operand positions selected by an actual native compiler registration.
/// This does not select an executable operation or prove successful completion.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum NativeCompilationOperandLayout {
    /// Pattern operand of the compiled regexp matching protocol.
    Pattern {
        /// Zero-based post-head position in the original effective argv.
        argument_index: usize,
        /// Registration must remain valid at this independent selection boundary.
        guard: NativeCompilationGuard,
    },
}

/// A compiler rejection at a proved native selection boundary.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct NativeCompilationFailure {
    /// Exact native result when authored operand presentation proves it.
    /// Absence retains a definite error without inventing its result bytes.
    pub message: Option<String>,
    /// Exact native error-code list when the selected compiler grammar proves it.
    pub error_code: Option<String>,
    /// Complete contextual error-info bytes, if independently established.
    /// Missing presentation must not be replaced by the message alone.
    pub error_info: Option<String>,
}

/// How an inline child's compiler failure affects the enclosing chunk.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NativeCompilationFailureScope {
    /// The failure rejects the enclosing compilation chunk before its effects.
    EnclosingChunk,
    /// The wrapper declines inline compilation, then catches the separately
    /// compiled script's error when runtime execution reaches the invocation.
    FallbackToGeneric,
}

/// Lexical compiler environment entered for one inline script operand.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum NativeCompiledBodyContext {
    /// Retain the environment already selected by the wrapper descriptor.
    Inherit,
    /// Enter the native compiler's loop exception range.
    Loop,
    /// Enter a protected native try body, handler or finally script.
    ExceptionRange,
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
        Some(match self.context {
            NativeCompiledBodyContext::Inherit => enclosing,
            NativeCompiledBodyContext::ExceptionRange => enclosing.with_inline_exception_range(),
            NativeCompiledBodyContext::Loop => NativeCompilationContext {
                loop_depth: enclosing.loop_depth.checked_add(1)?,
                ..enclosing
            },
        })
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

// TclCompileSubstCmd (C8.6–9.1): TclWordKnownAtCompileTime options are
// considered only when the final template is the original SIMPLE_WORD.
fn substitution_template_compiler_grammar(
    words: InvocationWords<'_>,
    shapes: &[NativeCompilationWordShape],
    version: TclVersion,
    operation: SemanticOperationId,
) -> NativeCompilationSelection {
    use NativeCompilationSelection as Selection;
    use NativeCompilationWordShape as Shape;
    if version < TclVersion::V8_6 {
        return Selection::Generic;
    }
    match shapes.last() {
        None | Some(Shape::Substituted | Shape::BackslashLiteral | Shape::Expanded) => {
            return Selection::Generic;
        }
        Some(Shape::Opaque) => return Selection::Unknown,
        Some(_) => {}
    }
    for shape in &shapes[..shapes.len() - 1] {
        if matches!(shape, Shape::Substituted | Shape::Expanded) {
            return Selection::Generic;
        }
        if *shape == Shape::Opaque {
            return Selection::Unknown;
        }
    }
    let arguments = words.arguments();
    let kinds = match crate::substitution::compiler_template_kinds(arguments, version) {
        Ok(kinds) => kinds,
        Err(selection) => return selection,
    };
    let Some(template) = arguments.literal_at(shapes.len() - 1) else {
        return Selection::Unknown;
    };
    let dialect = InvocationDialect::for_version(version);
    let flags = tcl_lexer::word_parts::SubstFlags {
        vars: kinds.variables,
        cmds: kinds.commands,
        backslashes: kinds.backslashes,
        ..Default::default()
    };
    let Ok(end) = u32::try_from(template.len()) else {
        return Selection::Unknown;
    };
    let Ok(arena) = tcl_lexer::word_parts::ExecutablePartArena::decompose(
        tcl_lexer::SourceImage::native(template.as_bytes()),
        tcl_lexer::Span::new(0, end),
        flags,
        tcl_lexer::LexerConfig::from_grammar(dialect.lexer_grammar),
    ) else {
        return Selection::Unknown;
    };
    if arena.all_parts().all(|part| {
        matches!(
            part.part,
            tcl_lexer::word_parts::ExecutablePart::Text(_)
                | tcl_lexer::word_parts::ExecutablePart::Variable { index: None, .. }
        )
    }) {
        // TclSubstCompile emits these scalar reads; no nested compiler visit
        // or independent chunk failure is introduced before them.
        Selection::Inline {
            operation,
            guard: NativeCompilationGuard::BeforeArguments,
        }
    } else {
        Selection::Unknown
    }
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

/// Specialized native array compiler family. C8.6 introduced these private
/// workers; earlier C releases use their monolithic generic array handler.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum NativeArrayCommand {
    /// Test the selected root's array kind.
    Exists,
    /// Ensure a root or populate its elements from a list.
    Set,
    /// Delete an entire array, or invoke the pattern-aware worker.
    Unset,
}

/// Native compiler-hook grammar, separate from a front-end lowering hook.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum NativeCompilationGrammar {
    /// No native compiler hook is licensed by this descriptor.
    NoHook,
    /// Native hook availability or syntax is not yet an authored closed contract.
    Unresolved,
    /// Independently authored ordered prerequisites of a named-invocation or
    /// absent-hook compiler. This does not wrap body or opcode grammars.
    WithImplementationPath {
        /// Actual terminal compiler grammar; this carries no handler semantics.
        compiler: &'static NativeCompilationSpec,
        /// Original ensemble mappings and worker identities, outermost first.
        lookups: &'static [NativeCompilerImplementationLookup],
        /// First native release installing this exact path.
        implementation_from: TclVersion,
    },
    /// Original independently registered mathematical operator compiler.
    MathOperator(crate::native_mathop_compilation::NativeMathOperator),
    /// Native variable append grammar, including release-specific list forms.
    VariableAppend(NativeAppendKind),
    /// Original `TclOO` helper compiler, with a separate runtime method-frame check.
    TclOoHelper(crate::native_tcloo_compilation::NativeTclOoHelper),
    /// Native procedure tail replacement; C9.1 also builds expanded operand lists.
    Tailcall,
    /// C8.6+ template compiler requires the final original `SIMPLE_WORD`.
    /// Its accepted-template traversal remains independently unmodelled.
    SubstitutionTemplate,
    /// No hook before this C release; an actual hook with unmodelled grammar thereafter.
    HookFrom(TclVersion),
    /// C8.5's monolithic namespace compiler only attempts original upvar syntax.
    /// Other namespace operations decline before preparing any operands.
    NamespaceLegacy,
    /// C8.6+ original private namespace-upvar worker compiler.
    NamespaceUpvarBindings,
    /// A procedure-only hook from this release; script frames decline the hook.
    /// Procedure grammar remains unmodelled and cannot license an opcode.
    ProcedureHookFrom(TclVersion),
    /// An ensemble member compiler emits a named private invocation from this release.
    NamedEnsembleInvocation {
        /// Original member mapping and private compiler command.
        lookup: &'static NativeCompilerImplementationLookup,
        /// First release installing the private implementation slot.
        implementation_from: TclVersion,
        /// First release registering this member's compiler hook.
        hook_from: TclVersion,
        /// Native compiler's post-member operand count.
        arity: crate::Arity,
    },
    /// C8.6+ specialized array compiler with its original private worker.
    Array {
        /// Selected native compiler implementation family.
        command: NativeArrayCommand,
        /// Original public mapping and private handler/compiler token.
        lookup: &'static NativeCompilerImplementationLookup,
    },
    /// C 8.5+ procedure-local global aliases with compile-known scalar tails.
    GlobalBindings,
    /// C8.5+ procedure-local namespace bindings with interleaved scalar stores.
    NamespaceVariableBindings,
    /// Native dictionary body compiler and its ensemble operand offset.
    Dictionary {
        /// Selected private implementation's compiler grammar.
        command: crate::native_dictionary::NativeDictionaryCommand,
        /// The original source includes an ensemble member word.
        ensemble: bool,
    },
    /// C 8.5+ local upvar compiler with its compile-time leading-level probe.
    Upvar,
    /// C9.1 procedure compiler selects a known optional frame word and script argv.
    Uplevel,
    /// C 8.6+ unset with substitution-free option and variable-name operands.
    LiteralUnset,
    /// C 8.6+ command enumeration: private named invocation or absolute-name resolution.
    InfoCommands,
    /// C 8.5+ variable existence query with one scalar or array-name operand.
    InfoExists,
    /// C8.6+ stack-level introspection with zero or one evaluated operand.
    InfoLevel,
    /// C8.6+ current namespace with no source operands.
    NamespaceCurrent,
    /// C8.6+ original namespace-origin opcode with a late command-name lookup.
    NamespaceOrigin,
    /// C8.6+ literal namespace-prefix builder, with a genuine private-worker
    /// fallback for dynamic operands or an already-scoped prefix.
    NamespaceCode,
    /// Native string equality; unsupported options use a private invocation from 8.6.
    StringEqual(crate::native_scalar_compilation::NativeScalarScope),
    /// C8.4+ string length accepts one evaluated operand; refused modern
    /// public forms retain the selected private worker invocation.
    StringLength(crate::native_scalar_compilation::NativeScalarScope),
    /// Original C string-match hook and its actual registration operand scope.
    StringMatch(crate::native_string_compilation::NativeStringMatchScope),
    /// C8.6+ original trim subject and explicit/default character set.
    StringTrim {
        /// Public member or actual private worker operand layout.
        scope: crate::native_scalar_compilation::NativeScalarScope,
        /// Selected ends, independent of the current command spelling.
        operation: crate::native_string_trim_compilation::NativeStringTrimOperation,
    },
    /// C8.4+ list length, with authentic C8.4 compile-time arity rejection.
    ListLength,
    /// C8.4+ list extraction with one list operand and any index operands.
    ListIndex,
    /// C 8.6+ list range with compile-time, immediately encodable indices.
    ListRange,
    /// C 8.5+ list assignment with substitution-free output-name operands.
    ListAssignment,
    /// C8.6 static insertion index; C9 evaluates the index at runtime.
    ListInsertion,
    /// C8.6+ original message/options Error compiler.
    Error,
    /// Simple native return with zero or one post-head operand.
    Return,
    /// Native list construction accepts every retained post-head operand count.
    ArgumentList,
    /// Native variadic compilation accepts every retained operand count from
    /// this C release; earlier releases invoke the command generically.
    ArgumentListFrom(TclVersion),
    /// Original concat operands, including compile-known literal folding.
    ArgumentConcatFrom(TclVersion),
    /// C8.6+ coroutine relay; C9.1 also compiles expanded operands and
    /// the empty relay, whose missing target is rejected during execution.
    CoroutineRelay,
    /// C8.6+ zero- or one-value coroutine suspension compiler.
    CoroutineYield,
    /// A release-gated hook accepts every retained word within an authored
    /// argc range; rejected shapes invoke the original handler generically.
    ArityFrom {
        /// First C release registering this compiler hook.
        first: TclVersion,
        /// Post-head operand counts accepted by that hook.
        arity: crate::Arity,
    },
    /// A native fixed-argc compiler: C 8.4 rejects mismatches at compilation,
    /// while later C releases defer them to ordinary runtime invocation.
    CheckedArity {
        /// Accepted post-head operand counts, independently of word contents.
        arity: crate::Arity,
        /// Native primitive's compiler error usage, even after rename.
        usage: &'static str,
    },
    /// Literal if clauses validated by the existing authored clause grammar.
    Conditional,
    /// C8.5+ original literal-arm switch compiler.
    Switch,
    /// Native expression compilation with at least one operand.
    Expression,
    /// Regexp compiler declines capture variables and unsupported option shapes.
    Regexp,
    /// Native for requires literal condition/next/body, with any initial word.
    ForLoop,
    /// Native while requires a simple condition and body, with Boolean pruning.
    WhileLoop,
    /// Native foreach has literal var-lists/body and arbitrary list operands.
    Foreach,
    /// Native variable load/store with one or two post-head words.
    VariableLoadStore,
    /// Native increment with one or two post-head words.
    Increment,
    /// Native catch; result/options operands require compiler-local scalars.
    Catch,
    /// C8.6+ native try clauses, local outputs and protected compiler bodies.
    Try,
    /// Native loop control with no post-head operands.
    LoopControl,
    /// The registered break compiler, without a public command lookup.
    Break,
    /// The registered continue compiler, without a public command lookup.
    Continue,
    /// Literal expression/script operands are required for this exact arity.
    LiteralOperands {
        /// Post-head operand count accepted by this hook.
        count: usize,
    },
}

/// Namespace and argument-stack protocol of the selected C tailcall compiler.
/// This projection describes emission after the hook has been admitted; it
/// grants neither procedure-frame eligibility nor command identity.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum NativeTailcallStack {
    /// C8.6/9.0 evaluate argv before capturing the live namespace.
    NamespaceAfterArguments,
    /// C9.1 captures the namespace before evaluating a bounded argv stack.
    NamespaceBeforeArguments,
    /// C9.1 captures namespace then builds an expanded or unbounded argv list.
    NamespacePrefixedList,
}

impl NativeTailcallStack {
    /// Native C9.1 builders flush a pending segment after this many words.
    /// Intermediate list construction preserves the engine's sharing lifetime.
    pub const LIST_SEGMENT_LIMIT: usize = 1 << 15;

    /// Shared native argument-list segment limit after a list-building compiler
    /// operation has been admitted. No limit grants an opcode or engine proof.
    #[must_use]
    pub fn argument_list_segment_limit(dialect: InvocationDialect) -> Option<usize> {
        (dialect.family() == Some(Family::Tcl) && dialect.tcl_version == Some(TclVersion::V9_1))
            .then_some(Self::LIST_SEGMENT_LIMIT)
    }

    /// Project the actual compiler's stack protocol without consulting a
    /// catalogue profile or reinterpreting already evaluated argument values.
    #[must_use]
    pub fn for_invocation(
        dialect: InvocationDialect,
        argument_count: usize,
        expanded: bool,
    ) -> Option<Self> {
        if dialect.family() != Some(Family::Tcl) {
            return None;
        }
        match dialect.tcl_version? {
            TclVersion::V8_6 | TclVersion::V9_0
                if !expanded && (1..255).contains(&argument_count) =>
            {
                Some(Self::NamespaceAfterArguments)
            }
            TclVersion::V8_4 | TclVersion::V8_5 | TclVersion::V8_6 | TclVersion::V9_0 => None,
            TclVersion::V9_1
                if expanded
                    || argument_count == 0
                    || argument_count >= Self::LIST_SEGMENT_LIMIT =>
            {
                Some(Self::NamespacePrefixedList)
            }
            TclVersion::V9_1 => Some(Self::NamespaceBeforeArguments),
        }
    }
}

/// Distinct native string and list append compiler protocols.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum NativeAppendKind {
    /// One-value string append; modern multiple values require a local scalar.
    String,
    /// List append; modern stack/array forms also accept multiple values.
    List,
}

impl NativeAppendKind {
    /// Whether the native no-value handler validates its existing value as a
    /// list. This runtime coercion policy grants no compiler operation.
    #[must_use]
    pub fn validates_empty_result(self, dialect: InvocationDialect) -> Option<bool> {
        match dialect.family()? {
            Family::Tcl | Family::F5Irules => Some(self == Self::List),
            Family::Jim
                if dialect.core_point.is_some_and(|point| {
                    point.release() == tcl_dialect::model::Release::JIM_0_84
                }) =>
            {
                Some(false)
            }
            _ => None,
        }
    }
}

/// Compilation boundary when the selected native command enters a script body.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum NativeBodyCompilation {
    /// Inline script bodies retain their enclosing compiler environment.
    Inherit,
    /// A separately compiled script object has no procedure-local compiler table.
    ScriptObject,
    /// A procedure body owns a fresh procedure compiler environment.
    ProcedureObject,
    /// Native direct evaluation does not select bytecode operations.
    Direct,
    /// C 8.4's uplevel evaluates directly; later C cores compile script objects.
    Uplevel,
}

/// Authored native operation grammar and independently selected body boundary.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct NativeCompilationSpec {
    /// Native grammar; front-end lowering metadata alone never licenses it.
    pub grammar: NativeCompilationGrammar,
    /// Native operation selected when the grammar succeeds.
    pub operation: SemanticOperationId,
    /// Entry environment for scripts run or retained by this implementation.
    pub body: NativeBodyCompilation,
}

/// Original argv layout of a selected C9.1 frame-evaluation operation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NativeUplevelOperands {
    /// Original explicit level operand; absent means the native default `1`.
    pub level: Option<usize>,
    /// First script operand, concatenated with every following original word.
    pub script_from: usize,
}

/// Original private command used by an ensemble compiler.
/// Admission of this slot is an entry-table fact; the descriptor cannot prove
/// that a live interpreter still contains its original implementation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct NativeCompilerImplementationLookup {
    /// Original ensemble whose dispatch mapping must select this member slot.
    pub ensemble: &'static str,
    /// Literal ensemble member in the original mapping.
    pub member: &'static str,
    /// Fully qualified original implementation slot.
    pub slot: &'static str,
    /// Catalogue command carrying the implementation's semantic descriptors.
    pub command: &'static str,
    /// Semantic argument prefix, excluding the catalogue command head.
    pub prepended: &'static [&'static str],
}

/// A vendor-owned value-handler contract independent of C compiler workers.
/// The caller must still prove the original converged public implementation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum NormalValueLeafProvider {
    /// BIG-IP's documented string value operations on the selected TMM handler.
    F5String,
    /// BIG-IP's documented binary value operations on the selected TMM handler.
    F5Binary,
}

impl NormalValueLeafProvider {
    fn admits(self, dialect: InvocationDialect) -> bool {
        matches!(self, Self::F5String | Self::F5Binary)
            && dialect.family() == Some(Family::F5Irules)
    }
}

/// Audited successful-handler transfer, independently of compiler selection.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SuccessfulHandlerSpec {
    /// Evaluate the selected expression arguments on the handler's normal
    /// path. Compiler preparation, operand callbacks and result headers are
    /// independent obligations of the expression owner.
    ExpressionArguments,
    /// A native leaf with no script or variable-name operands.
    Leaf,
    /// Sequence operands preserve the world only when the shared native
    /// argument decoder never enters its Tcl expression callback.
    ArithmeticSequenceArguments,
    /// Dictionary construction retains value objects without coercing them;
    /// only keys require materialised strings for closed normal world effects.
    DictionaryConstructor {
        /// Original mapping and independently mutable native worker.
        lookup: &'static NativeCompilerImplementationLookup,
        /// First C release installing this worker.
        implementation_from: TclVersion,
    },
    /// A native value leaf whose C ensemble worker became independently mutable.
    EnsembleLeaf {
        /// Original mapping and selected private worker on applicable C releases.
        lookup: &'static NativeCompilerImplementationLookup,
        /// First C release with this private worker protocol.
        implementation_from: TclVersion,
        /// Independent vendor handler contract, without a C worker assumption.
        direct_provider: Option<NormalValueLeafProvider>,
    },
    /// A native value leaf reached through multiple independently mutable ensembles.
    EnsemblePathLeaf {
        /// Frozen selector and all original public-to-private map dependencies.
        lookup: &'static crate::native_handler_path::NativeHandlerLookupPaths,
        /// First C release installing this nested worker protocol.
        implementation_from: TclVersion,
    },
    /// Audited binding installation uses only declared command-table transitions.
    /// This does not imply a pure value leaf or execute the installed target.
    CommandBindingTransition,
    /// Variable access roles describe all selected target names on success.
    VariableOperands,
    /// Variable targets remain conditional until this native matcher proves
    /// which stores occur on the successful continuation.
    ConditionalVariableOperands(crate::variable_output::NativeVariableOutputSpec),
    /// Read the selected variable, creating an empty value only when it remains
    /// undefined after read observers. Existing values are not stored again.
    InitialiseEmptyVariable,
    /// Catch captures selected output names after its body completes.
    CatchOutputs,
    /// Entry and completion-dependent writes belong to the shared dictionary scope protocol.
    DictionaryScope,
    /// Audited caller-frame scripts can execute under an unknown compiler
    /// protocol. Their inventory is possible execution, never opcode proof.
    PossibleBodies,
    /// A converged handler invokes the named procedure with its remaining argv.
    /// This caller inventory grants no compiler operation or procedure effects.
    UserProcedureCall,
}

/// Additional identity required before a reached normal handler can preserve world facts.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NormalHandlerImplementationLookup {
    /// This implementation invokes no independently selected worker.
    NoneRequired,
    /// The selected original worker and ensemble mapping must be proved.
    Required(NativeCompilerImplementationLookup),
    /// Each selected path edge requires its own actual ensemble and worker proof.
    RequiredPath(&'static crate::native_handler_path::NativeHandlerLookupPaths),
    /// The selected native protocol is not known well enough to choose a dependency.
    Unknown,
}

/// Exact operand layout of a converged handler's procedure invocation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UserProcedureInvocation {
    /// Effective operand containing the literal procedure name.
    pub target_at: usize,
    /// First operand passed to the procedure, preserving every remaining slot.
    pub arguments_from: usize,
}

/// Candidate script execution of a converged native handler. Compiler entry
/// and execution certainty remain independent of this handler grammar.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PossibleHandlerBodyFlow {
    /// Shared native argument grammar's execution ordering.
    pub flow: crate::script_body_flow::ScriptBodyFlow,
    /// Proved caller activation in which the handler evaluates these scripts.
    pub frame: crate::VariableAliasFrame,
}

/// When an operation selects the variable named by a retained argv operand.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum VariableOperandBindingPhase {
    /// The selected arguments and frame identify the target before body execution.
    AfterArguments,
    /// A script body can retarget aliases before an output variable is selected.
    NormalContinuation,
    /// Entry and writeback select addresses through a body execution protocol.
    BodyProtocol,
}

/// Whether outputs are looked up once or separately around observable writes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum VariableOutputLookup {
    /// The operation selects one target before any value write callbacks.
    SingleTarget,
    /// Each named output is looked up after the preceding output write.
    Sequential,
    /// Catch captures follow a dialect- and compiler-selected order after its body.
    CatchSelected,
    /// A body protocol owns ordered entry and completion-dependent target selection.
    BodyProtocol,
}

impl SuccessfulHandlerSpec {
    /// Original native worker retained by a normal-handler contract. The
    /// caller independently proves applicable engine/release and live identity.
    #[must_use]
    pub const fn native_worker(
        self,
    ) -> Option<(&'static NativeCompilerImplementationLookup, TclVersion)> {
        match self {
            Self::EnsembleLeaf {
                lookup,
                implementation_from,
                ..
            }
            | Self::DictionaryConstructor {
                lookup,
                implementation_from,
            } => Some((lookup, implementation_from)),
            _ => None,
        }
    }

    /// Original private slots needed by an explicitly trusted stock bootstrap.
    /// Enumerating alternatives proves neither a selected path nor a live
    /// worker, ensemble mapping, native compiler hook, or execution effect.
    #[must_use]
    pub fn stock_native_workers(
        self,
        dialect: InvocationDialect,
    ) -> Vec<NativeCompilerImplementationLookup> {
        if dialect.family() != Some(Family::Tcl) {
            return Vec::new();
        }
        let Some(version) = dialect.tcl_version else {
            return Vec::new();
        };
        if let Self::EnsemblePathLeaf {
            lookup,
            implementation_from,
        } = self
        {
            return if version >= implementation_from {
                lookup
                    .alternatives
                    .iter()
                    .flat_map(|path| path.lookups.iter().copied())
                    .collect()
            } else {
                Vec::new()
            };
        }
        self.native_worker()
            .filter(|(_, first)| version >= *first)
            .map_or_else(Vec::new, |(lookup, _)| vec![*lookup])
    }
    /// Physical target lookup order; output callbacks are not replay-free.
    #[must_use]
    pub const fn variable_output_lookup(self) -> VariableOutputLookup {
        match self {
            Self::Leaf
            | Self::ExpressionArguments
            | Self::ArithmeticSequenceArguments
            | Self::DictionaryConstructor { .. }
            | Self::EnsembleLeaf { .. }
            | Self::EnsemblePathLeaf { .. }
            | Self::PossibleBodies
            | Self::UserProcedureCall
            | Self::InitialiseEmptyVariable
            | Self::CommandBindingTransition => VariableOutputLookup::SingleTarget,
            Self::VariableOperands | Self::ConditionalVariableOperands(_) => {
                VariableOutputLookup::Sequential
            }
            Self::CatchOutputs => VariableOutputLookup::CatchSelected,
            Self::DictionaryScope => VariableOutputLookup::BodyProtocol,
        }
    }

    /// Address selection phase for outputs of this normal transfer contract.
    #[must_use]
    pub const fn variable_binding_phase(self) -> VariableOperandBindingPhase {
        match self {
            Self::CatchOutputs => VariableOperandBindingPhase::NormalContinuation,
            Self::DictionaryScope => VariableOperandBindingPhase::BodyProtocol,
            Self::Leaf
            | Self::ExpressionArguments
            | Self::ArithmeticSequenceArguments
            | Self::DictionaryConstructor { .. }
            | Self::EnsembleLeaf { .. }
            | Self::EnsemblePathLeaf { .. }
            | Self::VariableOperands
            | Self::ConditionalVariableOperands(_)
            | Self::InitialiseEmptyVariable
            | Self::PossibleBodies
            | Self::UserProcedureCall
            | Self::CommandBindingTransition => VariableOperandBindingPhase::AfterArguments,
        }
    }
}

impl crate::InvocationFacts {
    /// Project an authored normal-handler worker dependency independently of
    /// compiler selection. Unknown C release/protocol cannot mean no dependency.
    #[must_use]
    pub fn normal_handler_implementation_lookup(
        &self,
        dialect: Option<InvocationDialect>,
    ) -> NormalHandlerImplementationLookup {
        use NormalHandlerImplementationLookup as Lookup;
        if self.body_execution == Some(crate::body_execution::BodyExecutionSpec::ArrayIteration) {
            return match dialect {
                Some(dialect)
                    if dialect.family() == Some(Family::Tcl)
                        && dialect
                            .tcl_version
                            .is_some_and(|version| version >= TclVersion::V9_0) =>
                {
                    Lookup::Required(crate::array_iteration::IMPLEMENTATION_LOOKUP)
                }
                _ => Lookup::Unknown,
            };
        }
        let Some(contract) = self.successful_handler else {
            return Lookup::NoneRequired;
        };
        if let SuccessfulHandlerSpec::EnsemblePathLeaf {
            lookup,
            implementation_from,
        } = contract
        {
            return match dialect {
                Some(dialect)
                    if dialect.family() == Some(Family::Tcl)
                        && dialect
                            .tcl_version
                            .is_some_and(|version| version >= implementation_from) =>
                {
                    Lookup::RequiredPath(lookup)
                }
                _ => Lookup::Unknown,
            };
        }
        let Some((lookup, implementation_from)) = contract.native_worker() else {
            return Lookup::NoneRequired;
        };
        let direct_provider = match contract {
            SuccessfulHandlerSpec::EnsembleLeaf {
                direct_provider, ..
            } => direct_provider,
            _ => None,
        };
        let Some(dialect) = dialect else {
            return Lookup::Unknown;
        };
        match dialect.family() {
            Some(Family::Tcl) => match dialect.tcl_version {
                Some(version) if version >= implementation_from => Lookup::Required(*lookup),
                Some(_) => Lookup::NoneRequired,
                None => Lookup::Unknown,
            },
            Some(Family::Jim) => Lookup::NoneRequired,
            _ if direct_provider.is_some_and(|provider| provider.admits(dialect)) => {
                Lookup::NoneRequired
            }
            _ => Lookup::Unknown,
        }
    }
    /// Caller inventory of an independently proved normal handler. Neither
    /// compiler uncertainty nor possible pre-entry failure invents callers;
    /// actual handler or operand uncertainty still prevents this projection.
    #[must_use]
    pub fn successful_handler_user_procedure_call(
        &self,
        arguments: crate::InvocationArguments<'_>,
    ) -> Option<UserProcedureInvocation> {
        if self.successful_handler != Some(SuccessfulHandlerSpec::UserProcedureCall)
            || !self.traits.contains(crate::Traits::INVOKES_USER_PROC)
            || !self.arg_roles_complete
        {
            return None;
        }
        arguments.exact_argv_len()?;
        if self.arity_accepts_frozen_arguments() != Some(true) {
            return None;
        }
        let mut names = self.arg_roles.iter().filter_map(|&(index, role)| {
            (role == crate::ArgRole::Name).then_some(self.argument_offset + usize::from(index))
        });
        let target_at = names.next()?;
        if names.next().is_some() {
            return None;
        }
        arguments.literal_at(target_at)?;
        Some(UserProcedureInvocation {
            target_at,
            arguments_from: target_at + 1,
        })
    }

    /// Candidate body entry after independently proved handler convergence.
    /// Uses the existing shared script grammar and preserves unknown native
    /// compilation protocol; consumers must not turn candidates into Must.
    #[must_use]
    pub fn possible_handler_body_flow(
        &self,
        registry: &crate::CommandRegistry,
        arguments: crate::InvocationArguments<'_>,
        frame: crate::VariableAliasFrame,
    ) -> Option<PossibleHandlerBodyFlow> {
        if !matches!(
            self.successful_handler,
            Some(SuccessfulHandlerSpec::PossibleBodies | SuccessfulHandlerSpec::CatchOutputs)
        ) || frame == crate::VariableAliasFrame::Unknown
        {
            return None;
        }
        arguments.exact_argv_len()?;
        let flow = crate::case_bodies::script_body_flow_in_registry(registry, self, arguments);
        if self.arity_accepts_frozen_arguments() != Some(true)
            && !(self.arity_accepts_frozen_arguments().is_none()
                && matches!(flow, crate::script_body_flow::ScriptBodyFlow::CaseBodies(_)))
        {
            return None;
        }
        if !self.arg_roles_complete
            && !matches!(flow, crate::script_body_flow::ScriptBodyFlow::CaseBodies(_))
        {
            return None;
        }
        if matches!(flow, crate::script_body_flow::ScriptBodyFlow::Unknown(_)) {
            return None;
        }
        Some(PossibleHandlerBodyFlow { flow, frame })
    }

    /// Closed normal effects after every coercing operand has materialised.
    /// Dictionary construction retains its values without coercion and only
    /// requires known keys. Other dynamic objects may invoke coercion observers
    /// and retain the ordinary envelope.
    /// This never describes an invocation's error edge or proves worker identity.
    #[must_use]
    pub fn successful_value_leaf_world(
        &self,
        arguments: crate::InvocationArguments<'_>,
    ) -> Option<crate::EffectFootprint> {
        if !matches!(
            self.successful_handler,
            Some(
                SuccessfulHandlerSpec::Leaf
                    | SuccessfulHandlerSpec::ArithmeticSequenceArguments
                    | SuccessfulHandlerSpec::EnsembleLeaf { .. }
                    | SuccessfulHandlerSpec::EnsemblePathLeaf { .. }
                    | SuccessfulHandlerSpec::DictionaryConstructor { .. }
            )
        ) || self
            .successful_handler_effects(arguments, crate::VariableAliasFrame::Unknown)
            .is_none()
        {
            return None;
        }
        for index in 0..arguments.exact_argv_len()? {
            if !matches!(
                self.successful_handler,
                Some(SuccessfulHandlerSpec::DictionaryConstructor { .. })
            ) || index < self.argument_offset
                || (index - self.argument_offset).is_multiple_of(2)
            {
                arguments.literal_at(index)?;
            }
        }
        let mut effects = crate::EffectFootprint::default();
        for access in self.effects.accesses() {
            if access.mode == crate::EffectAccessMode::Read
                && !matches!(
                    self.successful_handler,
                    Some(SuccessfulHandlerSpec::DictionaryConstructor { .. })
                )
            {
                effects.add_access(access.clone());
            }
        }
        effects.add_access(crate::EffectAccess::new(
            crate::WorldStateDomain::InterpreterResult,
            crate::EffectAccessMode::Write,
            crate::InterpreterScope::Current,
            crate::NamespaceScope::Any,
            crate::SubjectScope::Wildcard,
        ));
        Some(effects)
    }

    /// Command-binding contracts admit only their declared table transitions;
    /// callback, body and variable effects are separate semantic purposes.
    fn command_binding_handler_admitted(&self) -> Option<()> {
        let transitions = self.state_transitions.declared()?;
        if transitions
            .facts()
            .iter()
            .any(|fact| match &fact.transition {
                crate::StateTransition::CommandBinding(_) => false,
                crate::StateTransition::Widen(widening) => widening
                    .domains
                    .iter()
                    .any(|domain| *domain != crate::StateTransitionDomain::CommandBindings),
                _ => true,
            })
            || self.arg_roles.iter().any(|(_, role)| {
                role.carries_script()
                    || matches!(role, crate::ArgRole::VarRead | crate::ArgRole::VarWrite)
            })
        {
            return None;
        }
        Some(())
    }

    /// Normal effects require an independently proved converged handler identity.
    /// This projection cannot license compiler entry, body traversal or opcodes.
    #[must_use]
    pub fn successful_handler_effects(
        &self,
        arguments: crate::InvocationArguments<'_>,
        frame: crate::VariableAliasFrame,
    ) -> Option<SuccessfulHandlerEffects> {
        let Some(contract) = self.successful_handler else {
            return self
                .native_compilation?
                .successful_handler_effects(arguments, frame);
        };
        let count = arguments.exact_argv_len()?;
        count.checked_sub(self.argument_offset)?;
        if self.arity_accepts_frozen_arguments() != Some(true) || !self.arg_roles_complete {
            return None;
        }
        match contract {
            SuccessfulHandlerSpec::ArithmeticSequenceArguments => {
                self.closed_arithmetic_sequence_arguments(arguments)?;
            }
            SuccessfulHandlerSpec::Leaf
            | SuccessfulHandlerSpec::EnsembleLeaf { .. }
            | SuccessfulHandlerSpec::EnsemblePathLeaf { .. }
            | SuccessfulHandlerSpec::DictionaryConstructor { .. }
                if self.arg_roles.iter().any(|(_, role)| {
                    role.carries_script()
                        || matches!(
                            role,
                            crate::ArgRole::VarRead
                                | crate::ArgRole::VarWrite
                                | crate::ArgRole::CommandPrefix
                        )
                }) =>
            {
                return None;
            }
            SuccessfulHandlerSpec::CommandBindingTransition => {
                self.command_binding_handler_admitted()?;
            }
            SuccessfulHandlerSpec::VariableOperands
            | SuccessfulHandlerSpec::ConditionalVariableOperands(_)
            | SuccessfulHandlerSpec::InitialiseEmptyVariable => {
                if frame == crate::VariableAliasFrame::Unknown
                    || self.arg_roles.iter().any(|(_, role)| {
                        matches!(role, crate::ArgRole::Body | crate::ArgRole::Expr)
                    })
                {
                    return None;
                }
                for &(index, role) in &self.arg_roles {
                    if matches!(role, crate::ArgRole::VarRead | crate::ArgRole::VarWrite) {
                        match arguments.get(self.argument_offset + usize::from(index))? {
                            crate::InvocationWord::Literal(_)
                            | crate::InvocationWord::ArrayElementName { .. } => {}
                            _ => return None,
                        }
                    }
                }
            }
            SuccessfulHandlerSpec::CatchOutputs => {
                if frame == crate::VariableAliasFrame::Unknown {
                    return None;
                }
                let dialect = arguments.dialect()?;
                let crate::catch_invocation::CatchInvocationSelection::Valid(selected) =
                    crate::catch_invocation::select_catch_invocation(arguments, dialect)
                else {
                    return None;
                };
                if selected.ignores(0) {
                    return None;
                }
                for index in [selected.result_var_at, selected.options_var_at]
                    .into_iter()
                    .flatten()
                {
                    arguments.literal_at(index)?;
                }
            }
            SuccessfulHandlerSpec::Leaf
            | SuccessfulHandlerSpec::EnsembleLeaf { .. }
            | SuccessfulHandlerSpec::EnsemblePathLeaf { .. }
            | SuccessfulHandlerSpec::DictionaryConstructor { .. } => {
                match self.normal_handler_implementation_lookup(arguments.dialect()) {
                    NormalHandlerImplementationLookup::Unknown => return None,
                    NormalHandlerImplementationLookup::RequiredPath(paths)
                        if paths.select(arguments, self.argument_offset).is_none() =>
                    {
                        return None;
                    }
                    _ => {}
                }
            }
            SuccessfulHandlerSpec::PossibleBodies
            | SuccessfulHandlerSpec::ExpressionArguments
            | SuccessfulHandlerSpec::DictionaryScope
            | SuccessfulHandlerSpec::UserProcedureCall => {
                return None;
            }
        }
        Some(SuccessfulHandlerEffects {
            operation: self.operation,
        })
    }

    fn closed_arithmetic_sequence_arguments(
        &self,
        arguments: crate::InvocationArguments<'_>,
    ) -> Option<()> {
        if self.native_result
            != Some(crate::native_result::NativeResultContract::ArithmeticSequence)
        {
            return None;
        }
        self.normal_list_method_provider(arguments)?;
        let values = (self.argument_offset..arguments.exact_argv_len()?)
            .map(|index| arguments.literal_at(index).map(str::as_bytes))
            .collect::<Option<Vec<_>>>()?;
        tcl_cmd_core::lseq::decode(&values).ok()?;
        Some(())
    }
}

/// Normal-edge effects shared by a stable native handler's generic and
/// compiled implementations. This is not a native operation selection.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SuccessfulHandlerEffects {
    /// Authored normative effect operation. Result object identity and internal
    /// representation remain independent; no compiler entry or error timing
    /// is established by this projection.
    pub operation: SemanticOperationId,
}

impl NativeCompilationSpec {
    /// Whether compilation prepares original source operands through a shared
    /// instruction recipe. This capability supplies neither a selected hook
    /// nor permission to execute a generic handler after compilation fails.
    #[must_use]
    pub const fn requires_original_word_preparation(self) -> bool {
        match self.grammar {
            NativeCompilationGrammar::WithImplementationPath { compiler, .. } => {
                compiler.requires_original_word_preparation()
            }
            NativeCompilationGrammar::Dictionary { command, .. } => matches!(
                command,
                crate::native_dictionary::NativeDictionaryCommand::Get
                    | crate::native_dictionary::NativeDictionaryCommand::Exists
                    | crate::native_dictionary::NativeDictionaryCommand::GetDefault
                    | crate::native_dictionary::NativeDictionaryCommand::GetWithDefault
                    | crate::native_dictionary::NativeDictionaryCommand::Set
                    | crate::native_dictionary::NativeDictionaryCommand::Unset
                    | crate::native_dictionary::NativeDictionaryCommand::Append
                    | crate::native_dictionary::NativeDictionaryCommand::Lappend
                    | crate::native_dictionary::NativeDictionaryCommand::Incr
                    | crate::native_dictionary::NativeDictionaryCommand::Update
                    | crate::native_dictionary::NativeDictionaryCommand::With
            ),
            NativeCompilationGrammar::MathOperator(_)
            | NativeCompilationGrammar::VariableLoadStore
            | NativeCompilationGrammar::ListIndex
            | NativeCompilationGrammar::LiteralUnset
            | NativeCompilationGrammar::Upvar
            | NativeCompilationGrammar::InfoExists
            | NativeCompilationGrammar::Array { .. }
            | NativeCompilationGrammar::InfoLevel
            | NativeCompilationGrammar::NamespaceCurrent
            | NativeCompilationGrammar::NamespaceOrigin
            | NativeCompilationGrammar::NamespaceCode
            | NativeCompilationGrammar::ArgumentConcatFrom(_)
            | NativeCompilationGrammar::Uplevel
            | NativeCompilationGrammar::GlobalBindings
            | NativeCompilationGrammar::NamespaceVariableBindings
            | NativeCompilationGrammar::NamespaceLegacy
            | NativeCompilationGrammar::NamespaceUpvarBindings
            | NativeCompilationGrammar::Switch
            | NativeCompilationGrammar::Conditional
            | NativeCompilationGrammar::WhileLoop
            | NativeCompilationGrammar::ForLoop
            | NativeCompilationGrammar::Catch
            | NativeCompilationGrammar::Foreach
            | NativeCompilationGrammar::Try
            | NativeCompilationGrammar::Expression
            | NativeCompilationGrammar::Return
            | NativeCompilationGrammar::StringTrim { .. }
            | NativeCompilationGrammar::StringMatch(_)
            | NativeCompilationGrammar::StringEqual(_)
            | NativeCompilationGrammar::StringLength(_)
            | NativeCompilationGrammar::ListLength
            | NativeCompilationGrammar::Tailcall
            | NativeCompilationGrammar::CoroutineYield
            | NativeCompilationGrammar::CoroutineRelay
            | NativeCompilationGrammar::Error => true,
            _ => false,
        }
    }

    /// Actual namespace/frame instruction purpose retained by its compiler.
    #[must_use]
    pub const fn introspection_compilation(
        self,
    ) -> Option<crate::native_introspection_compilation::NativeIntrospectionKind> {
        use crate::native_introspection_compilation::NativeIntrospectionKind as Kind;
        match self.grammar {
            NativeCompilationGrammar::InfoLevel => Some(Kind::InfoLevel),
            NativeCompilationGrammar::NamespaceCurrent => Some(Kind::NamespaceCurrent),
            NativeCompilationGrammar::NamespaceOrigin => Some(Kind::NamespaceOrigin),
            NativeCompilationGrammar::NamespaceCode => Some(Kind::NamespaceCode),
            _ => None,
        }
    }

    /// Original scalar opcode and its actual registration operand layout.
    #[must_use]
    pub const fn scalar_compilation(
        self,
    ) -> Option<(
        crate::native_scalar_compilation::NativeScalarOperation,
        crate::native_scalar_compilation::NativeScalarScope,
    )> {
        use crate::native_scalar_compilation::{
            NativeScalarOperation as Operation, NativeScalarScope as Scope,
        };
        match self.grammar {
            NativeCompilationGrammar::StringEqual(scope) => Some((Operation::StringEqual, scope)),
            NativeCompilationGrammar::StringLength(scope) => Some((Operation::StringLength, scope)),
            NativeCompilationGrammar::ListLength => {
                Some((Operation::ListLength, Scope::PrivateOperands))
            }
            _ => None,
        }
    }

    /// Authored iterator result policy, independent of the callable's spelling.
    #[must_use]
    pub const fn each_collection(
        self,
    ) -> Option<crate::native_each_compilation::NativeEachCollection> {
        use crate::native_each_compilation::NativeEachCollection;
        if !matches!(self.grammar, NativeCompilationGrammar::Foreach) {
            return None;
        }
        match self.operation {
            SemanticOperationId::StructuredLowering(crate::hooks::LoweringHookId::Foreach) => {
                Some(NativeEachCollection::Foreach)
            }
            SemanticOperationId::StructuredLowering(crate::hooks::LoweringHookId::Lmap) => {
                Some(NativeEachCollection::Lmap)
            }
            _ => None,
        }
    }
    /// Original implementation prerequisites for this compiler purpose.
    /// An empty vector proves no additional slot is required by the authored
    /// descriptor; `None` retains an unavailable or unresolved protocol.
    /// Live command identity and every original map remain caller obligations.
    #[must_use]
    pub fn implementation_prerequisites(
        self,
        dialect: InvocationDialect,
    ) -> Option<Vec<NativeCompilerImplementationLookup>> {
        if let NativeCompilationGrammar::WithImplementationPath {
            compiler,
            lookups,
            implementation_from,
            ..
        } = self.grammar
        {
            return (dialect.family() == Some(Family::Tcl)
                && dialect
                    .tcl_version
                    .is_some_and(|version| version >= implementation_from)
                && !lookups.is_empty()
                && compiler.operation == self.operation
                && self.body == compiler.body
                // A NULL compileProc does not select a compiled body. Its
                // runtime implementation can still enter a script object or
                // direct evaluation; both descriptors retain that same policy.
                && (compiler.body == NativeBodyCompilation::Inherit
                    || compiler.grammar == NativeCompilationGrammar::NoHook)
                && matches!(
                    compiler.grammar,
                    NativeCompilationGrammar::NoHook
                        | NativeCompilationGrammar::HookFrom(_)
                        | NativeCompilationGrammar::TclOoHelper(_)
                        | NativeCompilationGrammar::StringTrim { .. }
                        | NativeCompilationGrammar::StringMatch(_)
                        | NativeCompilationGrammar::NamedEnsembleInvocation { .. }
                )
                && lookups
                    .windows(2)
                    .all(|edges| edges[0].slot == edges[1].ensemble)
                && compiler
                    .implementation_lookup(dialect)
                    .is_none_or(|terminal| lookups.last() == Some(&terminal)))
            .then(|| lookups.to_vec());
        }
        (self.grammar != NativeCompilationGrammar::Unresolved)
            .then(|| self.implementation_lookup(dialect).into_iter().collect())
    }

    /// Independently describe operand layout for a proved compiler registration.
    /// Consumers retain its actual target, registration and guard prerequisites;
    /// the returned layout cannot turn an unresolved operation into an opcode.
    #[must_use]
    pub fn operand_layout(
        self,
        words: InvocationWords<'_>,
        shapes: &[NativeCompilationWordShape],
        dialect: Option<InvocationDialect>,
        context: NativeCompilationContext,
    ) -> Option<NativeCompilationOperandLayout> {
        let dialect = dialect.filter(|dialect| dialect.family() == Some(Family::Tcl))?;
        if self.grammar != NativeCompilationGrammar::Regexp
            || dialect.tcl_version? < TclVersion::V8_5
            || context.mode != NativeCompilationMode::BytecodeObject
            || context.frame == NativeCompilationFrame::Unknown
            || words.arguments().exact_argv_len() != Some(shapes.len())
            || shapes.contains(&NativeCompilationWordShape::Expanded)
        {
            return None;
        }
        Some(NativeCompilationOperandLayout::Pattern {
            argument_index: regexp_compiler_pattern_index(words, shapes).ok()?,
            guard: NativeCompilationGuard::BeforeArguments,
        })
    }

    /// Known original argv accepted by C9.1's frame-evaluation compiler.
    /// Frame existence is checked during execution, independently of this layout.
    #[must_use]
    pub fn uplevel_operands(
        self,
        arguments: crate::InvocationArguments<'_>,
    ) -> Option<NativeUplevelOperands> {
        if self.grammar != NativeCompilationGrammar::Uplevel {
            return None;
        }
        let count = arguments.exact_argv_len()?;
        if !(1..=255).contains(&count) || arguments.literal_at(0).is_none() {
            return None;
        }
        let crate::frame_effect::FrameArgumentResolution::Valid {
            level_word_len,
            level,
        } = crate::FrameEffectSpec::UPLEVEL.resolve_arguments(arguments)
        else {
            return None;
        };
        match level {
            crate::FrameLevel::Relative(number) | crate::FrameLevel::Absolute(number)
                if number > i32::MAX as u32 =>
            {
                return None;
            }
            crate::FrameLevel::Dynamic => return None,
            _ => {}
        }
        Some(NativeUplevelOperands {
            level: (level_word_len == 1).then_some(0),
            script_from: level_word_len,
        })
    }

    /// Select the original root or procedure namespace binding compiler.
    #[must_use]
    pub const fn namespace_binding_kind(
        self,
    ) -> Option<crate::native_namespace_binding_compilation::NativeNamespaceBindingKind> {
        use crate::native_namespace_binding_compilation::NativeNamespaceBindingKind;
        match self.grammar {
            NativeCompilationGrammar::NamespaceLegacy
            | NativeCompilationGrammar::NamespaceUpvarBindings => {
                Some(NativeNamespaceBindingKind::Upvar)
            }
            NativeCompilationGrammar::GlobalBindings => Some(NativeNamespaceBindingKind::Global),
            NativeCompilationGrammar::NamespaceVariableBindings => {
                Some(NativeNamespaceBindingKind::Variable)
            }
            _ => None,
        }
    }

    fn namespace_binding_selection(
        self,
        outcome: crate::native_namespace_binding_compilation::NativeNamespaceBindingOutcome,
    ) -> NativeCompilationSelection {
        use crate::native_namespace_binding_compilation::NativeNamespaceBindingOutcome;
        match outcome {
            NativeNamespaceBindingOutcome::Inline => NativeCompilationSelection::Inline {
                operation: self.operation,
                guard: NativeCompilationGuard::BeforeArguments,
            },
            NativeNamespaceBindingOutcome::Generic => NativeCompilationSelection::Generic,
            NativeNamespaceBindingOutcome::Unknown => NativeCompilationSelection::Unknown,
        }
    }

    /// Actual compiler-hook registration for the authored native engine point.
    /// Missing engine/version evidence retains uncertainty; handler semantics
    /// and the presence of a descriptor do not establish hook registration.
    #[must_use]
    pub fn compiler_hook_presence(self, dialect: InvocationDialect) -> Option<bool> {
        if let NativeCompilationGrammar::WithImplementationPath { compiler, .. } = self.grammar {
            self.implementation_prerequisites(dialect)?;
            return compiler.compiler_hook_presence(dialect);
        }
        if self.grammar == NativeCompilationGrammar::NoHook || dialect.family() == Some(Family::Jim)
        {
            return Some(false);
        }
        if dialect.family() != Some(Family::Tcl) {
            return None;
        }
        let version = dialect.tcl_version?;
        Some(match self.grammar {
            NativeCompilationGrammar::NoHook => false,
            NativeCompilationGrammar::MathOperator(operator) => version >= operator.first_version(),
            NativeCompilationGrammar::NamespaceUpvarBindings => version >= TclVersion::V8_6,
            NativeCompilationGrammar::Dictionary { command, .. } => version >= command.hook_from(),
            NativeCompilationGrammar::HookFrom(first)
            | NativeCompilationGrammar::ProcedureHookFrom(first)
            | NativeCompilationGrammar::ArgumentListFrom(first)
            | NativeCompilationGrammar::ArgumentConcatFrom(first)
            | NativeCompilationGrammar::ArityFrom { first, .. }
            | NativeCompilationGrammar::NamedEnsembleInvocation {
                hook_from: first, ..
            } => version >= first,
            NativeCompilationGrammar::GlobalBindings
            | NativeCompilationGrammar::NamespaceVariableBindings
            | NativeCompilationGrammar::Upvar
            | NativeCompilationGrammar::InfoExists
            | NativeCompilationGrammar::ListAssignment
            | NativeCompilationGrammar::NamespaceLegacy => version >= TclVersion::V8_5,
            NativeCompilationGrammar::StringTrim { .. }
            | NativeCompilationGrammar::Array { .. }
            | NativeCompilationGrammar::Error
            | NativeCompilationGrammar::TclOoHelper(_)
            | NativeCompilationGrammar::CoroutineRelay
            | NativeCompilationGrammar::CoroutineYield
            | NativeCompilationGrammar::LiteralUnset
            | NativeCompilationGrammar::InfoCommands
            | NativeCompilationGrammar::InfoLevel
            | NativeCompilationGrammar::NamespaceCurrent
            | NativeCompilationGrammar::NamespaceOrigin
            | NativeCompilationGrammar::NamespaceCode
            | NativeCompilationGrammar::Tailcall
            | NativeCompilationGrammar::ListRange
            | NativeCompilationGrammar::ListInsertion
            | NativeCompilationGrammar::SubstitutionTemplate => version >= TclVersion::V8_6,
            NativeCompilationGrammar::Uplevel => version >= TclVersion::V9_1,
            _ => true,
        })
    }

    /// Additional original command identity required by this compiler grammar.
    /// C 8.5+ info is an ensemble whose exists compiler belongs to its private
    /// implementation command. C 8.4 and Jim have no such native dependency.
    #[must_use]
    pub fn implementation_lookup(
        self,
        dialect: InvocationDialect,
    ) -> Option<NativeCompilerImplementationLookup> {
        if let NativeCompilationGrammar::WithImplementationPath { compiler, .. } = self.grammar {
            self.implementation_prerequisites(dialect)?;
            return compiler.implementation_lookup(dialect);
        }
        if let NativeCompilationGrammar::Array { lookup, .. } = self.grammar {
            return (dialect.family() == Some(Family::Tcl)
                && dialect
                    .tcl_version
                    .is_some_and(|version| version >= TclVersion::V8_6))
            .then_some(*lookup);
        }
        if self.grammar == NativeCompilationGrammar::NamespaceUpvarBindings {
            return (dialect.family() == Some(Family::Tcl)
                && dialect
                    .tcl_version
                    .is_some_and(|version| version >= TclVersion::V8_6))
            .then_some(NativeCompilerImplementationLookup {
                ensemble: "::namespace",
                member: "upvar",
                slot: "::tcl::namespace::upvar",
                command: "namespace",
                prepended: &["upvar"],
            });
        }
        if matches!(
            self.grammar,
            NativeCompilationGrammar::NamespaceCurrent
                | NativeCompilationGrammar::NamespaceOrigin
                | NativeCompilationGrammar::NamespaceCode
        ) {
            return (dialect.family() == Some(Family::Tcl)
                && dialect
                    .tcl_version
                    .is_some_and(|version| version >= TclVersion::V8_6))
            .then_some(match self.grammar {
                NativeCompilationGrammar::NamespaceOrigin => NAMESPACE_ORIGIN_IMPLEMENTATION,
                NativeCompilationGrammar::NamespaceCode => NAMESPACE_CODE_IMPLEMENTATION,
                _ => NAMESPACE_CURRENT_IMPLEMENTATION,
            });
        }
        if matches!(
            self.grammar,
            NativeCompilationGrammar::StringEqual(
                crate::native_scalar_compilation::NativeScalarScope::PublicMember
            ) | NativeCompilationGrammar::StringLength(
                crate::native_scalar_compilation::NativeScalarScope::PublicMember
            )
        ) {
            return (dialect.family() == Some(Family::Tcl)
                && dialect
                    .tcl_version
                    .is_some_and(|version| version >= TclVersion::V8_5))
            .then_some(
                if matches!(self.grammar, NativeCompilationGrammar::StringLength(_)) {
                    STRING_LENGTH_IMPLEMENTATION
                } else {
                    STRING_EQUAL_IMPLEMENTATION
                },
            );
        }
        if let NativeCompilationGrammar::StringTrim {
            scope: crate::native_scalar_compilation::NativeScalarScope::PublicMember,
            operation,
        } = self.grammar
        {
            return (dialect.family() == Some(Family::Tcl)
                && dialect
                    .tcl_version
                    .is_some_and(|version| version >= TclVersion::V8_5))
            .then_some(operation.lookup());
        }
        if self.grammar
            == NativeCompilationGrammar::StringMatch(
                crate::native_string_compilation::NativeStringMatchScope::PublicMember,
            )
        {
            return (dialect.family() == Some(Family::Tcl)
                && dialect
                    .tcl_version
                    .is_some_and(|version| version >= TclVersion::V8_5))
            .then_some(crate::native_string_compilation::STRING_MATCH_IMPLEMENTATION);
        }
        if self.grammar == NativeCompilationGrammar::InfoCommands {
            return (dialect.family() == Some(Family::Tcl)
                && dialect
                    .tcl_version
                    .is_some_and(|version| version >= TclVersion::V8_5))
            .then_some(NativeCompilerImplementationLookup {
                ensemble: "::info",
                member: "commands",
                slot: "::tcl::info::commands",
                command: "info",
                prepended: &["commands"],
            });
        }
        if let NativeCompilationGrammar::NamedEnsembleInvocation {
            lookup,
            implementation_from,
            ..
        } = self.grammar
        {
            return (dialect.family() == Some(Family::Tcl)
                && dialect
                    .tcl_version
                    .is_some_and(|version| version >= implementation_from))
            .then_some(*lookup);
        }
        if let NativeCompilationGrammar::Dictionary {
            command,
            ensemble: true,
        } = self.grammar
        {
            return (dialect.family() == Some(Family::Tcl)
                && dialect
                    .tcl_version
                    .is_some_and(|version| version >= TclVersion::V8_5))
            .then(|| command.lookup());
        }
        if dialect.family() != Some(Family::Tcl)
            || dialect
                .tcl_version
                .is_none_or(|version| version < TclVersion::V8_5)
        {
            return None;
        }
        match self.grammar {
            NativeCompilationGrammar::InfoExists => Some(INFO_EXISTS_IMPLEMENTATION),
            NativeCompilationGrammar::InfoLevel => Some(INFO_LEVEL_IMPLEMENTATION),
            _ => None,
        }
    }

    /// Prove only successful handler effects when every possible compiler
    /// protocol reaches the same original handler and frozen argv. The caller
    /// must prove that identity convergence independently before using this.
    ///
    /// Body/control compilers may change traversal and exception boundaries;
    /// they are deliberately absent. Variable operations additionally require
    /// the actual physical frame and literal selected name. This never grants
    /// executable lowering, opcode eligibility or successful compilation.
    #[must_use]
    pub fn successful_handler_effects(
        self,
        arguments: crate::InvocationArguments<'_>,
        frame: crate::VariableAliasFrame,
    ) -> Option<SuccessfulHandlerEffects> {
        let count = arguments.exact_argv_len()?;
        match self.grammar {
            NativeCompilationGrammar::VariableLoadStore | NativeCompilationGrammar::Increment
                if matches!(count, 1 | 2)
                    && frame != crate::VariableAliasFrame::Unknown
                    && arguments.literal_at(0).is_some() => {}
            NativeCompilationGrammar::GlobalBindings
                if frame != crate::VariableAliasFrame::Unknown && arguments.are_all_literals() => {}
            NativeCompilationGrammar::Upvar
                if frame != crate::VariableAliasFrame::Unknown
                    && arguments.are_all_literals()
                    && matches!(
                        crate::FrameEffectSpec::UPVAR
                            .successful_layout(arguments)
                            .layout,
                        crate::frame_effect::FrameArgumentResolution::Valid { .. }
                    ) => {}
            NativeCompilationGrammar::ArgumentList => {}
            _ => return None,
        }
        Some(SuccessfulHandlerEffects {
            operation: self.operation,
        })
    }

    /// Resolve native compilation using source shape, never computed argv text.
    #[must_use]
    pub fn select(
        self,
        words: InvocationWords<'_>,
        shapes: &[NativeCompilationWordShape],
        dialect: Option<InvocationDialect>,
        context: NativeCompilationContext,
    ) -> NativeCompilationSelection {
        use NativeCompilationSelection as Selection;
        if let NativeCompilationGrammar::WithImplementationPath { compiler, .. } = self.grammar {
            if dialect.is_none_or(|dialect| self.implementation_prerequisites(dialect).is_none()) {
                return Selection::Unknown;
            }
            return compiler.select(words, shapes, dialect, context);
        }
        if context.mode == NativeCompilationMode::Direct {
            return Selection::Generic;
        }
        let Some(dialect) = dialect else {
            return Selection::Unknown;
        };
        if dialect.family() != Some(Family::Tcl) {
            return if dialect.family() == Some(Family::Jim) {
                Selection::Generic
            } else {
                Selection::Unknown
            };
        }
        let Some(version) = dialect.tcl_version else {
            return Selection::Unknown;
        };
        if context.mode != NativeCompilationMode::BytecodeObject {
            return Selection::Unknown;
        }
        if words.head().literal().is_none()
            || matches!(self.grammar, NativeCompilationGrammar::NoHook)
        {
            return Selection::Generic;
        }
        // Tcl 9.1 registers CMD_COMPILES_EXPANDED on this compiler. Its
        // namespace-prefixed list builder consumes original source words;
        // the expanded runtime argv length is deliberately not a prerequisite.
        if self.grammar == NativeCompilationGrammar::CoroutineRelay && version >= TclVersion::V9_1 {
            return if shapes.contains(&NativeCompilationWordShape::Opaque) {
                Selection::Unknown
            } else {
                Selection::Inline {
                    operation: self.operation,
                    guard: NativeCompilationGuard::BeforeArguments,
                }
            };
        }
        if let NativeCompilationGrammar::TclOoHelper(helper) = self.grammar {
            return crate::native_tcloo_compilation::select(
                helper,
                shapes,
                words.arguments().literal_at(0).map(str::as_bytes),
                version,
                self.operation,
            );
        }
        if self.grammar == NativeCompilationGrammar::Tailcall {
            return self.select_tailcall(shapes, version, context);
        }
        if self.grammar == NativeCompilationGrammar::VariableAppend(NativeAppendKind::List)
            && version >= TclVersion::V9_1
        {
            return variable_append_grammar(
                NativeAppendKind::List,
                self.operation,
                words.with_dialect(dialect),
                shapes,
                version,
                context,
            );
        }
        let Some(count) = words.arguments().exact_argv_len() else {
            return Selection::Generic;
        };
        if shapes.len() != count || shapes.contains(&NativeCompilationWordShape::Opaque) {
            return Selection::Unknown;
        }
        if shapes.contains(&NativeCompilationWordShape::Expanded) {
            return Selection::Generic;
        }
        self.select_grammar(words.with_dialect(dialect), shapes, version, context)
    }

    /// Select an independently registered compiler using original byte words.
    /// `operand_from` addresses the complete original vector including its
    /// head; it is not a runtime argc or a written-only role offset. Shape-only
    /// descriptors do not require Unicode. Value-dependent descriptors use an
    /// unchanged checked view or abstain; opaque literals never become dynamic.
    /// This selection does not attest registration or implementation dependencies.
    #[must_use]
    pub fn select_native_words(
        self,
        words: &crate::native_compiler_words::NativeCompilerWords<'_>,
        operand_from: usize,
        dialect: Option<InvocationDialect>,
        context: NativeCompilationContext,
    ) -> NativeCompilationSelection {
        use NativeCompilationSelection as Selection;
        if let NativeCompilationGrammar::WithImplementationPath { compiler, .. } = self.grammar {
            if dialect.is_none_or(|dialect| self.implementation_prerequisites(dialect).is_none()) {
                return Selection::Unknown;
            }
            return compiler.select_native_words(words, operand_from, dialect, context);
        }
        if context.mode == NativeCompilationMode::Direct {
            return Selection::Generic;
        }
        let Some(dialect) = dialect else {
            return Selection::Unknown;
        };
        if dialect.family() != Some(Family::Tcl) {
            return if dialect.family() == Some(Family::Jim) {
                Selection::Generic
            } else {
                Selection::Unknown
            };
        }
        let Some(version) = dialect.tcl_version else {
            return Selection::Unknown;
        };
        if context.mode != NativeCompilationMode::BytecodeObject || operand_from == 0 {
            return Selection::Unknown;
        }
        let Some(shapes) = words.shapes().get(operand_from..) else {
            return Selection::Unknown;
        };
        if words.shapes()[0] == NativeCompilationWordShape::Expanded {
            return Selection::Unknown;
        }
        match words.shapes()[0].compiler_head(Some(dialect)) {
            Some(false) => return Selection::Generic,
            None => return Selection::Unknown,
            Some(true) => {}
        }
        if self.grammar == NativeCompilationGrammar::NoHook {
            return Selection::Generic;
        }
        if let NativeCompilationGrammar::MathOperator(operator) = self.grammar {
            return match crate::native_mathop_compilation::compile_native_mathop(
                words,
                operand_from,
                operator,
                version,
                context,
            ) {
                Ok(Some(_)) => self.selected_grammar_result(true, version),
                Ok(None) => Selection::Generic,
                Err(_) => Selection::Unknown,
            };
        }
        if let NativeCompilationGrammar::StringTrim { scope, .. } = self.grammar {
            return crate::native_string_trim_compilation::select_original(
                words,
                operand_from,
                scope,
                version,
                self.operation,
            );
        }
        if let NativeCompilationGrammar::StringMatch(scope) = self.grammar {
            return crate::native_string_compilation::select_original(
                words,
                operand_from,
                scope,
                version,
                self.operation,
            );
        }

        if let Some(kind) = self.introspection_compilation()
            && crate::native_introspection_compilation::compile_native_introspection(
                words,
                operand_from,
                kind,
                version,
            )
            .is_some()
        {
            return Selection::Inline {
                operation: self.operation,
                guard: NativeCompilationGuard::BeforeArguments,
            };
        }
        if let Some((operation, scope)) = self.scalar_compilation() {
            return crate::native_scalar_compilation::select_original(
                words,
                operand_from,
                operation,
                scope,
                version,
                self.operation,
            );
        }

        if self.grammar == NativeCompilationGrammar::ListIndex {
            return crate::native_list_index_compilation::select_original(
                words,
                operand_from,
                version,
                self,
            );
        }

        if self.grammar == NativeCompilationGrammar::Upvar {
            if version == TclVersion::V8_4 || context.frame == NativeCompilationFrame::ScriptCode {
                return Selection::Generic;
            }
            return match crate::native_upvar_compilation::compile_native_upvar(
                words,
                operand_from,
                version,
                context,
            ) {
                Ok(_) => self.selected_grammar_result(true, version),
                Err(crate::native_upvar_compilation::NativeUpvarUnavailable::Geometry) => {
                    Selection::Generic
                }
                Err(_) => Selection::Unknown,
            };
        }
        if self.grammar == NativeCompilationGrammar::NamespaceUpvarBindings {
            return match crate::native_namespace_upvar_compilation::compile_native_namespace_upvar_worker(
                words, operand_from, version, context,
            ) {
                Ok(recipe) => self.namespace_binding_selection(recipe.outcome),
                Err(_) => Selection::Unknown,
            };
        }
        if self.grammar == NativeCompilationGrammar::NamespaceLegacy {
            return self.select_original_namespace_legacy(words, operand_from, version, context);
        }
        if self.grammar == NativeCompilationGrammar::LiteralUnset {
            return match crate::native_unset_compilation::compile_native_unset(
                words,
                operand_from,
                version,
            ) {
                Ok(Some(_)) => self.selected_grammar_result(true, version),
                Ok(None) => Selection::Generic,
                Err(_) => Selection::Unknown,
            };
        }
        if matches!(
            self.grammar,
            NativeCompilationGrammar::Error
                | NativeCompilationGrammar::Conditional
                | NativeCompilationGrammar::ForLoop
                | NativeCompilationGrammar::Catch
                | NativeCompilationGrammar::WhileLoop
                | NativeCompilationGrammar::Foreach
                | NativeCompilationGrammar::Try
        ) {
            return self.select_original_control_words(
                words,
                operand_from,
                dialect,
                context,
                version,
            );
        }
        if matches!(
            self.grammar,
            NativeCompilationGrammar::TclOoHelper(_)
                | NativeCompilationGrammar::Tailcall
                | NativeCompilationGrammar::Switch
                | NativeCompilationGrammar::GlobalBindings
                | NativeCompilationGrammar::NamespaceVariableBindings
        ) || (self.grammar == NativeCompilationGrammar::CoroutineRelay
            && version >= TclVersion::V9_1)
        {
            return self.select_original_special_words(
                words,
                operand_from,
                shapes,
                context,
                version,
            );
        }
        self.select_original_operand_words(words, operand_from, shapes, dialect, context, version)
    }

    fn select_original_control_outcome<T>(
        self,
        outcome: &crate::native_control_compilation::NativeControlOutcome<T>,
        version: TclVersion,
    ) -> NativeCompilationSelection {
        use crate::native_control_compilation::NativeControlOutcome;
        match outcome {
            NativeControlOutcome::Inline(_) => self.selected_grammar_result(true, version),
            NativeControlOutcome::Generic => NativeCompilationSelection::Generic,
            NativeControlOutcome::Rejected(_) => NativeCompilationSelection::CompileError,
        }
    }

    fn select_original_namespace_legacy(
        self,
        words: &crate::native_compiler_words::NativeCompilerWords<'_>,
        operand_from: usize,
        version: TclVersion,
        context: NativeCompilationContext,
    ) -> NativeCompilationSelection {
        use crate::native_namespace_binding_compilation::NativeNamespaceBindingOutcome;
        match crate::native_namespace_upvar_compilation::compile_native_namespace_upvar(
            words,
            operand_from,
            version,
            context,
        ) {
            Ok(recipe) => match recipe.outcome {
                NativeNamespaceBindingOutcome::Inline => NativeCompilationSelection::Inline {
                    operation: self.operation,
                    guard: NativeCompilationGuard::BeforeArguments,
                },
                NativeNamespaceBindingOutcome::Generic => NativeCompilationSelection::Generic,
                NativeNamespaceBindingOutcome::Unknown => NativeCompilationSelection::Unknown,
            },
            Err(_) => NativeCompilationSelection::Unknown,
        }
    }

    fn select_original_control_words(
        self,
        words: &crate::native_compiler_words::NativeCompilerWords<'_>,
        operand_from: usize,
        dialect: InvocationDialect,
        context: NativeCompilationContext,
        version: TclVersion,
    ) -> NativeCompilationSelection {
        use NativeCompilationSelection as Selection;
        if self.grammar == NativeCompilationGrammar::Error {
            return match crate::native_error_compilation::compile_native_error(
                words,
                operand_from,
                version,
            ) {
                Ok(_) => self.selected_grammar_result(true, version),
                Err(
                    crate::native_error_compilation::NativeErrorCompilationUnavailable::Generic,
                ) => Selection::Generic,
                Err(
                    crate::native_error_compilation::NativeErrorCompilationUnavailable::Geometry,
                ) => Selection::Unknown,
            };
        }
        if matches!(
            self.grammar,
            NativeCompilationGrammar::Conditional
                | NativeCompilationGrammar::ForLoop
                | NativeCompilationGrammar::Catch
                | NativeCompilationGrammar::WhileLoop
        ) {
            return match crate::native_control_instructions::native_control_instruction(
                self.grammar,
                words,
                operand_from,
                dialect,
                context,
            ) {
                Ok(recipe) => self.select_original_control_outcome(&recipe.outcome, version),
                Err(_) => Selection::Unknown,
            };
        }
        if self.grammar == NativeCompilationGrammar::Foreach {
            let Some(collection) = self.each_collection() else {
                return Selection::Unknown;
            };
            return match crate::native_each_compilation::compile_native_each(
                words,
                operand_from,
                version,
                context,
                collection,
            ) {
                Ok(recipe) => self.select_original_control_outcome(&recipe.outcome, version),
                Err(_) => Selection::Unknown,
            };
        }
        if self.grammar == NativeCompilationGrammar::Try {
            return match crate::native_try_compilation::compile_native_try(
                words,
                operand_from,
                version,
                context,
            ) {
                Ok(recipe) => self.select_original_control_outcome(&recipe.outcome, version),
                Err(_) => Selection::Unknown,
            };
        }
        unreachable!("selected original control grammar")
    }

    fn select_original_special_words(
        self,
        words: &crate::native_compiler_words::NativeCompilerWords<'_>,
        operand_from: usize,
        shapes: &[NativeCompilationWordShape],
        context: NativeCompilationContext,
        version: TclVersion,
    ) -> NativeCompilationSelection {
        use NativeCompilationSelection as Selection;
        if let NativeCompilationGrammar::TclOoHelper(helper) = self.grammar {
            return crate::native_tcloo_compilation::select_original(
                helper,
                words,
                operand_from,
                version,
                self.operation,
            );
        }
        if self.grammar == NativeCompilationGrammar::Tailcall {
            return self.select_tailcall(shapes, version, context);
        }
        if self.grammar == NativeCompilationGrammar::CoroutineRelay && version >= TclVersion::V9_1 {
            return self.selected_grammar_result(true, version);
        }
        if self.grammar == NativeCompilationGrammar::Switch {
            return match crate::native_switch_compilation::native_switch_instruction(
                words,
                operand_from,
                version,
            ) {
                Ok(_) => self.selected_grammar_result(true, version),
                Err(crate::native_switch_compilation::NativeSwitchUnavailable::Generic) => {
                    Selection::Generic
                }
                Err(_) => Selection::Unknown,
            };
        }
        if let Some(kind) = self.namespace_binding_kind() {
            return match crate::native_namespace_binding_compilation::compile_native_namespace_bindings(
                words, operand_from, version, context, kind,
            ) {
                Ok(recipe) => self.namespace_binding_selection(recipe.outcome),
                Err(_) => Selection::Unknown,
            };
        }
        unreachable!("selected original special grammar")
    }

    fn select_original_operand_words(
        self,
        words: &crate::native_compiler_words::NativeCompilerWords<'_>,
        operand_from: usize,
        shapes: &[NativeCompilationWordShape],
        dialect: InvocationDialect,
        context: NativeCompilationContext,
        version: TclVersion,
    ) -> NativeCompilationSelection {
        use NativeCompilationSelection as Selection;
        let expanded_list_append = self.grammar
            == NativeCompilationGrammar::VariableAppend(NativeAppendKind::List)
            && version >= TclVersion::V9_1;
        if let NativeCompilationGrammar::Dictionary { command, ensemble } = self.grammar
            && let Some(selection) =
                command.select_original_compilation(words, operand_from, ensemble, version, context)
        {
            return selection;
        }
        if let NativeCompilationGrammar::ArgumentConcatFrom(first) = self.grammar {
            if version < first {
                return Selection::Generic;
            }
            let Ok(projected) =
                crate::native_compiler_word_projection::project_native_compiler_words(
                    words, version,
                )
            else {
                return Selection::Unknown;
            };
            let Some(operands) = projected.get(operand_from..) else {
                return Selection::Unknown;
            };
            return self.selected_grammar_result(
                !operands
                    .iter()
                    .any(|word| word.shape == NativeCompilationWordShape::Expanded),
                version,
            );
        }
        if !expanded_list_append && shapes.contains(&NativeCompilationWordShape::Expanded) {
            // A static expansion may have been flattened by TclParseCommand
            // before the compiler sees its tokens. It needs that projection.
            return if (operand_from..words.shapes().len()).any(|index| {
                words.shapes()[index] == NativeCompilationWordShape::Expanded
                    && words.literal(index).is_some()
            }) {
                Selection::Unknown
            } else {
                Selection::Generic
            };
        }
        if let Some(selection) = self.select_shape_grammar(shapes, version, context) {
            return selection;
        }
        if let NativeCompilationGrammar::VariableAppend(kind) = self.grammar {
            return variable_append_grammar_bytes(
                kind,
                self.operation,
                words.literal(operand_from),
                shapes,
                version,
                context,
            );
        }
        if self.grammar == NativeCompilationGrammar::Return {
            return match crate::native_return_compilation::native_return_instruction(
                words,
                operand_from,
                version,
                context,
            ) {
                Ok(_) => self.selected_grammar_result(true, version),
                Err(
                    crate::native_return_compilation::NativeReturnCompilationUnavailable::Generic,
                ) => Selection::Generic,
                Err(_) => Selection::Unknown,
            };
        }
        let Some(projected) = words.checked_words() else {
            return Selection::Unknown;
        };
        self.select(
            InvocationWords::structured(projected[0], &projected[operand_from..]),
            shapes,
            Some(dialect),
            context,
        )
    }

    /// Whether this actual descriptor is a basic named-invocation compiler.
    /// An ensemble-shaped runtime handler does not change its CPP kind.
    #[must_use]
    pub const fn is_named_invocation_compiler(self) -> bool {
        match self.grammar {
            NativeCompilationGrammar::NamedEnsembleInvocation { .. } => true,
            NativeCompilationGrammar::WithImplementationPath { compiler, .. } => {
                compiler.is_named_invocation_compiler()
            }
            _ => false,
        }
    }

    /// Select the actual installed worker's grammar, independently of the
    /// original public command's spelling. This grants no registration proof.
    #[must_use]
    pub fn select_registered_worker_native_words(
        self,
        words: &crate::native_compiler_words::NativeCompilerWords<'_>,
        operand_from: usize,
        dialect: Option<InvocationDialect>,
        context: NativeCompilationContext,
    ) -> NativeCompilationSelection {
        if let NativeCompilationGrammar::Array { command, lookup } = self.grammar {
            let Some(dialect) = dialect.filter(|dialect| dialect.family() == Some(Family::Tcl))
            else {
                return NativeCompilationSelection::Unknown;
            };
            let Some(version) = dialect.tcl_version else {
                return NativeCompilationSelection::Unknown;
            };
            if context.mode != NativeCompilationMode::BytecodeObject {
                return self.select_native_words(words, operand_from, Some(dialect), context);
            }
            let Some(projected) = words.checked_words() else {
                return NativeCompilationSelection::Unknown;
            };
            let Some(arguments) = projected.get(operand_from..) else {
                return NativeCompilationSelection::Unknown;
            };
            let Some(shapes) = words.shapes().get(operand_from..) else {
                return NativeCompilationSelection::Unknown;
            };
            return array_compilation_grammar_for(
                (self.operation, command, lookup, true),
                InvocationWords::structured(projected[0], arguments),
                shapes,
                version,
                context,
            );
        }
        let selection = self.select_native_words(words, operand_from, dialect, context);
        if matches!(
            self.grammar,
            NativeCompilationGrammar::InfoLevel
                | NativeCompilationGrammar::NamespaceCurrent
                | NativeCompilationGrammar::NamespaceOrigin
                | NativeCompilationGrammar::NamespaceCode
        ) && matches!(
            selection,
            NativeCompilationSelection::NamedInvocation {
                protocol: NativeNamedInvocationProtocol::EnsembleRewrite,
                ..
            }
        ) {
            NativeCompilationSelection::Generic
        } else {
            selection
        }
    }

    /// Select using the resolved member's operand view while retaining every
    /// original compiler token shape. Primitive descriptors consume their own
    /// operands; ensemble-wide grammars retain the public member word.
    #[must_use]
    pub fn select_for_facts(
        self,
        words: InvocationWords<'_>,
        shapes: &[NativeCompilationWordShape],
        facts: &crate::InvocationFacts,
        dialect: Option<InvocationDialect>,
        context: NativeCompilationContext,
    ) -> NativeCompilationSelection {
        let offset = if matches!(
            self.grammar,
            NativeCompilationGrammar::InfoExists
                | NativeCompilationGrammar::InfoLevel
                | NativeCompilationGrammar::NamespaceCurrent
                | NativeCompilationGrammar::NamespaceOrigin
                | NativeCompilationGrammar::NamespaceCode
        ) {
            let private = dialect.and_then(|dialect| self.implementation_lookup(dialect));
            if private.is_some_and(|lookup| {
                words.head_literal().is_some_and(|head| {
                    head.strip_prefix("::").unwrap_or(head)
                        == lookup.slot.strip_prefix("::").unwrap_or(lookup.slot)
                })
            }) {
                0
            } else {
                facts.argument_offset
            }
        } else {
            0
        };
        let Some(shapes) = shapes.get(offset..) else {
            return NativeCompilationSelection::Unknown;
        };
        self.select(
            InvocationWords::from_arguments(words.head(), words.arguments().slice_from(offset)),
            shapes,
            dialect,
            context,
        )
    }

    /// Project native compiler failures through authored grammar, never the
    /// invoked token's spelling. C Tcl 8.4's fixed arity errors name the native
    /// primitive even when the command object was renamed.
    #[must_use]
    pub fn failure_for_selection(
        self,
        selection: NativeCompilationSelection,
        _words: InvocationWords<'_>,
        facts: &crate::InvocationFacts,
        dialect: Option<InvocationDialect>,
    ) -> Option<NativeCompilationFailure> {
        if selection != NativeCompilationSelection::CompileError
            || dialect
                .filter(|dialect| dialect.family() == Some(Family::Tcl))
                .and_then(|dialect| dialect.tcl_version)
                != Some(TclVersion::V8_4)
        {
            return None;
        }
        let usage = match self.grammar {
            NativeCompilationGrammar::CheckedArity { usage, .. } => Some(usage),
            NativeCompilationGrammar::ListLength => Some("llength list"),
            NativeCompilationGrammar::VariableLoadStore => Some("set varName ?newValue?"),
            NativeCompilationGrammar::Increment => Some("incr varName ?increment?"),
            NativeCompilationGrammar::VariableAppend(NativeAppendKind::String) => {
                Some("append varName ?value value ...?")
            }
            NativeCompilationGrammar::VariableAppend(NativeAppendKind::List) => {
                Some("lappend varName ?value value ...?")
            }
            NativeCompilationGrammar::Catch => Some("catch command ?varName?"),
            NativeCompilationGrammar::Expression => Some("expr arg ?arg ...?"),
            NativeCompilationGrammar::ForLoop => Some("for start test next command"),
            NativeCompilationGrammar::Foreach => {
                Some("foreach varList list ?varList list ...? command")
            }
            NativeCompilationGrammar::LoopControl
            | NativeCompilationGrammar::Break
            | NativeCompilationGrammar::Continue => match facts.completion.codes {
                crate::completion::CompletionCodeDomain::Exact(
                    [crate::completion::CompletionCode::Break],
                ) => Some("break"),
                crate::completion::CompletionCodeDomain::Exact(
                    [crate::completion::CompletionCode::Continue],
                ) => Some("continue"),
                _ => None,
            },
            NativeCompilationGrammar::WhileLoop => Some("while test command"),
            _ => None,
        };
        Some(NativeCompilationFailure {
            message: usage.map(|usage| format!("wrong # args: should be \"{usage}\"")),
            error_code: Some("NONE".into()),
            error_info: None,
        })
    }

    /// Present a compiler rejection with independently retained source words.
    /// Raw spellings are needed when a missing script follows a quoted or
    /// braced expression; decoded values alone cannot prove those bytes.
    #[must_use]
    pub fn failure_for_selection_with_source(
        self,
        selection: NativeCompilationSelection,
        words: InvocationWords<'_>,
        facts: &crate::InvocationFacts,
        dialect: Option<InvocationDialect>,
        written_head: &str,
        operand_spellings: &[&str],
    ) -> Option<NativeCompilationFailure> {
        let mut failure = self.failure_for_selection(selection, words, facts, dialect)?;
        if matches!(self.grammar, NativeCompilationGrammar::Conditional) {
            let values = words.arguments().literal_values()?;
            if values.len() != operand_spellings.len() {
                return None;
            }
            failure.message = crate::commands::tcl::native_if_compile_shape_message(
                &values,
                written_head,
                operand_spellings,
            );
        }
        Some(failure)
    }

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

    fn select_grammar(
        self,
        words: InvocationWords<'_>,
        shapes: &[NativeCompilationWordShape],
        version: TclVersion,
        context: NativeCompilationContext,
    ) -> NativeCompilationSelection {
        use NativeCompilationSelection as Selection;
        if let Some(selection) = self.select_delegated_grammar(words, shapes, version, context) {
            return selection;
        }
        if let Some(selection) = self.select_shape_grammar(shapes, version, context) {
            return selection;
        }
        let valid = match self.grammar {
            NativeCompilationGrammar::Catch => match catch_grammar(words, shapes, version, context)
            {
                Ok(valid) => valid,
                Err(selection) => return selection,
            },
            NativeCompilationGrammar::Conditional => {
                match conditional_grammar(words, shapes, version) {
                    Ok(valid) => valid,
                    Err(selection) => return selection,
                }
            }
            NativeCompilationGrammar::Upvar => match upvar_grammar(words, shapes, version, context)
            {
                Ok(valid) => valid,
                Err(selection) => return selection,
            },
            NativeCompilationGrammar::Return => {
                match return_grammar(words, shapes, version, context) {
                    Ok(valid) => valid,
                    Err(selection) => return selection,
                }
            }
            _ => return Selection::Unknown,
        };
        self.selected_grammar_result(valid, version)
    }

    /// Shared native token/count grammar. Neither byte values nor unchanged
    /// Unicode views are required by these compiler descriptors.
    fn select_shape_grammar(
        self,
        shapes: &[NativeCompilationWordShape],
        version: TclVersion,
        context: NativeCompilationContext,
    ) -> Option<NativeCompilationSelection> {
        use NativeCompilationSelection as Selection;
        let valid = match self.grammar {
            NativeCompilationGrammar::CheckedArity { arity, .. } => {
                u16::try_from(shapes.len()).is_ok_and(|count| arity.accepts(count))
            }
            NativeCompilationGrammar::MathOperator(operator) => {
                return Some(match operator.accepts(shapes.len(), version, context) {
                    Some(true) => self.selected_grammar_result(true, version),
                    Some(false) => Selection::Generic,
                    None => Selection::Unknown,
                });
            }
            NativeCompilationGrammar::ListLength => shapes.len() == 1,
            NativeCompilationGrammar::VariableLoadStore | NativeCompilationGrammar::Increment => {
                matches!(shapes.len(), 1 | 2)
            }
            NativeCompilationGrammar::Return if shapes.len() <= 1 => {
                // Zero/one-result Return compilers inspect the original count,
                // not result bytes. C8.4 additionally requires an uncaught
                // procedure compiler context.
                if version == TclVersion::V8_4 {
                    if context.frame != NativeCompilationFrame::ProcedureCode {
                        return Some(Selection::Generic);
                    }
                    match context.catch_depth {
                        Some(0) => {}
                        Some(_) => return Some(Selection::Generic),
                        None => return Some(Selection::Unknown),
                    }
                }
                true
            }
            NativeCompilationGrammar::Catch if shapes.len() == 1 => true,
            NativeCompilationGrammar::LoopControl
            | NativeCompilationGrammar::Break
            | NativeCompilationGrammar::Continue => shapes.is_empty(),
            NativeCompilationGrammar::Expression => !shapes.is_empty(),
            NativeCompilationGrammar::ForLoop => match for_loop_grammar(shapes) {
                Ok(valid) => valid,
                Err(selection) => return Some(selection),
            },
            NativeCompilationGrammar::LiteralUnset => {
                match literal_unset_grammar(shapes, version) {
                    Ok(valid) => valid,
                    Err(selection) => return Some(selection),
                }
            }
            NativeCompilationGrammar::InfoExists => {
                if version < TclVersion::V8_5 {
                    return Some(Selection::Generic);
                }
                shapes.len() == 1
            }
            NativeCompilationGrammar::ListIndex => {
                if shapes.is_empty() {
                    return Some(Selection::Generic);
                }
                u32::try_from(shapes.len()).is_ok_and(|count| count < u32::MAX)
            }
            NativeCompilationGrammar::ArgumentList => {
                if version <= TclVersion::V8_5 {
                    match context.frame {
                        NativeCompilationFrame::ScriptCode => return Some(Selection::Generic),
                        NativeCompilationFrame::Unknown => return Some(Selection::Unknown),
                        NativeCompilationFrame::ProcedureCode => {}
                    }
                }
                true
            }
            NativeCompilationGrammar::ArgumentListFrom(first)
            | NativeCompilationGrammar::ArgumentConcatFrom(first) => {
                if version < first {
                    return Some(Selection::Generic);
                }
                true
            }
            NativeCompilationGrammar::LiteralOperands { count } => {
                match literal_operands_grammar(shapes, count) {
                    Ok(valid) => valid,
                    Err(selection) => return Some(selection),
                }
            }
            NativeCompilationGrammar::WhileLoop => match literal_operands_grammar(shapes, 2) {
                Ok(valid) => valid,
                Err(selection) => return Some(selection),
            },
            _ => return None,
        };
        Some(self.selected_grammar_result(valid, version))
    }

    fn select_uplevel(
        self,
        words: InvocationWords<'_>,
        shapes: &[NativeCompilationWordShape],
        version: TclVersion,
        context: NativeCompilationContext,
    ) -> NativeCompilationSelection {
        use NativeCompilationSelection as Selection;
        if version < TclVersion::V9_1 || context.frame == NativeCompilationFrame::ScriptCode {
            Selection::Generic
        } else if context.frame != NativeCompilationFrame::ProcedureCode {
            Selection::Unknown
        } else if !shapes.first().is_some_and(|shape| {
            matches!(
                shape,
                NativeCompilationWordShape::Literal
                    | NativeCompilationWordShape::QuotedLiteral
                    | NativeCompilationWordShape::BracedLiteral
                    | NativeCompilationWordShape::BackslashLiteral
            )
        }) || self.uplevel_operands(words.arguments()).is_none()
        {
            Selection::Generic
        } else {
            Selection::Inline {
                operation: self.operation,
                guard: NativeCompilationGuard::BeforeArguments,
            }
        }
    }

    fn select_delegated_grammar(
        self,
        words: InvocationWords<'_>,
        shapes: &[NativeCompilationWordShape],
        version: TclVersion,
        context: NativeCompilationContext,
    ) -> Option<NativeCompilationSelection> {
        use NativeCompilationSelection as Selection;
        Some(match self.grammar {
            NativeCompilationGrammar::VariableAppend(kind) => {
                variable_append_grammar(kind, self.operation, words, shapes, version, context)
            }
            NativeCompilationGrammar::Array { command, lookup } => array_compilation_grammar(
                self.operation,
                command,
                lookup,
                words,
                shapes,
                version,
                context,
            ),
            NativeCompilationGrammar::Foreach => {
                self.select_foreach(words, shapes, version, context)
            }
            NativeCompilationGrammar::Dictionary { command, ensemble } => {
                command.select(ensemble, words, shapes, version, context)
            }
            NativeCompilationGrammar::NamedEnsembleInvocation {
                lookup,
                hook_from,
                arity,
                ..
            } => named_ensemble_grammar(lookup, hook_from, arity, words, shapes, version),
            NativeCompilationGrammar::Try => {
                crate::native_try::select(self.operation, words, shapes, version, context)
            }
            NativeCompilationGrammar::Uplevel => {
                self.select_uplevel(words, shapes, version, context)
            }
            NativeCompilationGrammar::Unresolved => Selection::Unknown,
            NativeCompilationGrammar::GlobalBindings
            | NativeCompilationGrammar::NamespaceVariableBindings => self
                .namespace_binding_selection(
                    crate::native_namespace_binding_compilation::namespace_binding_shape_outcome(
                        words,
                        shapes,
                        version,
                        context,
                        self.namespace_binding_kind()?,
                    ),
                ),
            NativeCompilationGrammar::SubstitutionTemplate => {
                substitution_template_compiler_grammar(words, shapes, version, self.operation)
            }
            NativeCompilationGrammar::HookFrom(first) => unmodelled_hook_selection(version, first),
            NativeCompilationGrammar::NamespaceUpvarBindings => {
                if version < TclVersion::V8_6 || context.frame == NativeCompilationFrame::ScriptCode
                {
                    Selection::Generic
                } else {
                    Selection::Unknown
                }
            }
            NativeCompilationGrammar::NamespaceLegacy => {
                if version < TclVersion::V8_5 || context.frame == NativeCompilationFrame::ScriptCode
                {
                    Selection::Generic
                } else {
                    Selection::Unknown
                }
            }
            NativeCompilationGrammar::ProcedureHookFrom(first) => {
                procedure_hook_selection(version, first, context.frame)
            }
            NativeCompilationGrammar::Error => gated_arity_grammar(
                TclVersion::V8_6,
                crate::Arity::new(1, 3),
                shapes,
                version,
                self.operation,
            ),
            NativeCompilationGrammar::ArityFrom { first, arity } => {
                gated_arity_grammar(first, arity, shapes, version, self.operation)
            }
            NativeCompilationGrammar::CoroutineYield => gated_arity_grammar(
                TclVersion::V8_6,
                crate::Arity::new(0, 1),
                shapes,
                version,
                self.operation,
            ),
            NativeCompilationGrammar::CoroutineRelay => gated_arity_grammar(
                TclVersion::V8_6,
                crate::Arity::at_least(1),
                shapes,
                version,
                self.operation,
            ),
            NativeCompilationGrammar::NoHook => Selection::Generic,
            _ => return self.select_delegated_word_grammar(words, shapes, version),
        })
    }

    fn select_delegated_word_grammar(
        self,
        words: InvocationWords<'_>,
        shapes: &[NativeCompilationWordShape],
        version: TclVersion,
    ) -> Option<NativeCompilationSelection> {
        Some(match self.grammar {
            NativeCompilationGrammar::InfoLevel => {
                info_level_grammar(words, shapes, version, self.operation)
            }
            NativeCompilationGrammar::NamespaceCurrent => {
                namespace_current_grammar(words, shapes, version, self.operation)
            }
            NativeCompilationGrammar::NamespaceOrigin => {
                namespace_origin_grammar(words, shapes, version, self.operation)
            }
            NativeCompilationGrammar::NamespaceCode => {
                namespace_code_grammar(words, shapes, version, self.operation)
            }
            NativeCompilationGrammar::InfoCommands => {
                info_commands_grammar(words, shapes, version, self.operation)
            }
            NativeCompilationGrammar::StringEqual(scope) => string_member_arity_grammar(
                words,
                shapes,
                version,
                self.operation,
                &STRING_EQUAL_IMPLEMENTATION,
                scope,
                2,
            ),
            NativeCompilationGrammar::StringTrim { scope, .. } => {
                let from = usize::from(
                    scope == crate::native_scalar_compilation::NativeScalarScope::PublicMember,
                );
                if version < TclVersion::V8_6
                    || !shapes.get(from..).is_some_and(|arguments| {
                        matches!(arguments.len(), 1 | 2)
                            && !arguments.contains(&NativeCompilationWordShape::Expanded)
                    })
                {
                    NativeCompilationSelection::Generic
                } else {
                    NativeCompilationSelection::Inline {
                        operation: self.operation,
                        guard: NativeCompilationGuard::BeforeArguments,
                    }
                }
            }
            NativeCompilationGrammar::StringMatch(scope) => {
                let from = usize::from(
                    scope == crate::native_string_compilation::NativeStringMatchScope::PublicMember,
                );
                if from == 1
                    && !shapes.first().is_some_and(|shape| {
                        literal_shape(*shape)
                            && (version != TclVersion::V8_4
                                || *shape == NativeCompilationWordShape::Literal)
                    })
                {
                    return Some(NativeCompilationSelection::Generic);
                }
                let Some(arguments) = shapes.get(from..) else {
                    return Some(NativeCompilationSelection::Generic);
                };
                crate::native_string_compilation::select(
                    arguments,
                    words.arguments().literal_at(from).map(str::as_bytes),
                    version,
                    self.operation,
                    from,
                )
            }
            NativeCompilationGrammar::StringLength(scope) => string_member_arity_grammar(
                words,
                shapes,
                version,
                self.operation,
                &STRING_LENGTH_IMPLEMENTATION,
                scope,
                1,
            ),
            NativeCompilationGrammar::ListRange => {
                list_range_grammar(words, shapes, version, self.operation)
            }
            NativeCompilationGrammar::ListAssignment => {
                list_assignment_grammar(shapes, version, self.operation)
            }
            NativeCompilationGrammar::ListInsertion => {
                list_insertion_grammar(words, shapes, version, self.operation)
            }
            NativeCompilationGrammar::Regexp => regexp_grammar(words, shapes),
            _ => return None,
        })
    }

    fn select_tailcall(
        self,
        shapes: &[NativeCompilationWordShape],
        version: TclVersion,
        context: NativeCompilationContext,
    ) -> NativeCompilationSelection {
        use NativeCompilationSelection as Selection;
        if version < TclVersion::V8_6 || context.frame == NativeCompilationFrame::ScriptCode {
            return Selection::Generic;
        }
        if context.frame != NativeCompilationFrame::ProcedureCode
            || shapes.contains(&NativeCompilationWordShape::Opaque)
        {
            return Selection::Unknown;
        }
        if version < TclVersion::V9_1
            && (shapes.is_empty()
                || shapes.len() >= 255
                || shapes.contains(&NativeCompilationWordShape::Expanded))
        {
            return Selection::Generic;
        }
        self.selected_grammar_result(true, version)
    }

    fn selected_grammar_result(
        self,
        valid: bool,
        version: TclVersion,
    ) -> NativeCompilationSelection {
        use NativeCompilationSelection as Selection;
        if !valid {
            return if version == TclVersion::V8_4 {
                Selection::CompileError
            } else {
                Selection::Generic
            };
        }
        Selection::Inline {
            operation: self.operation,
            guard: if version == TclVersion::V8_4 {
                NativeCompilationGuard::ChunkEntry
            } else {
                NativeCompilationGuard::BeforeArguments
            },
        }
    }

    fn select_foreach(
        self,
        words: InvocationWords<'_>,
        shapes: &[NativeCompilationWordShape],
        version: TclVersion,
        context: NativeCompilationContext,
    ) -> NativeCompilationSelection {
        use NativeCompilationSelection as Selection;
        if context.frame != NativeCompilationFrame::ProcedureCode {
            return if context.frame == NativeCompilationFrame::ScriptCode {
                Selection::Generic
            } else {
                Selection::Unknown
            };
        }
        if shapes.len() < 3 || shapes.len().is_multiple_of(2) {
            return if version == TclVersion::V8_4 {
                Selection::CompileError
            } else {
                Selection::Generic
            };
        }
        if !literal_shape(*shapes.last().unwrap()) {
            return Selection::Generic;
        }
        for index in (0..shapes.len() - 1).step_by(2) {
            let Some(var_list) = words.arguments().literal_at(index) else {
                return Selection::Generic;
            };
            let Ok(names) = tcl_syntax::list::split_list(var_list) else {
                return Selection::Unknown;
            };
            if names.is_empty() {
                return Selection::Unknown;
            }
            if names.iter().any(|name| {
                tcl_syntax::naming::is_qualified(name.as_bytes())
                    || tcl_syntax::naming::split_element_ref(name).is_some()
            }) {
                return Selection::Generic;
            }
        }
        Selection::Inline {
            operation: self.operation,
            guard: if version == TclVersion::V8_4 {
                NativeCompilationGuard::ChunkEntry
            } else {
                NativeCompilationGuard::BeforeArguments
            },
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

// TclCompileRegexpCmd in all pinned C releases accepts only -nocase
// abbreviations and -- before exactly two matching operands. Captures and
// other options invoke the live handler; accepted regexp opcodes remain
// unmodelled here rather than granting a generic dispatch proof.
fn unmodelled_hook_selection(version: TclVersion, first: TclVersion) -> NativeCompilationSelection {
    if version < first {
        NativeCompilationSelection::Generic
    } else {
        NativeCompilationSelection::Unknown
    }
}

fn procedure_hook_selection(
    version: TclVersion,
    first: TclVersion,
    frame: NativeCompilationFrame,
) -> NativeCompilationSelection {
    if frame == NativeCompilationFrame::ScriptCode {
        NativeCompilationSelection::Generic
    } else {
        unmodelled_hook_selection(version, first)
    }
}

fn for_loop_grammar(
    shapes: &[NativeCompilationWordShape],
) -> Result<bool, NativeCompilationSelection> {
    if shapes.len() != 4 {
        Ok(false)
    } else if shapes[1..].iter().all(|shape| literal_shape(*shape)) {
        Ok(true)
    } else {
        Err(NativeCompilationSelection::Generic)
    }
}

fn literal_unset_grammar(
    shapes: &[NativeCompilationWordShape],
    version: TclVersion,
) -> Result<bool, NativeCompilationSelection> {
    if version < TclVersion::V8_6 {
        Err(NativeCompilationSelection::Generic)
    } else if !shapes.iter().all(|shape| literal_shape(*shape)) {
        // Dynamic operands can interleave evaluation and individual unsets.
        Err(NativeCompilationSelection::Unknown)
    } else {
        Ok(true)
    }
}

const INFO_EXISTS_IMPLEMENTATION: NativeCompilerImplementationLookup =
    NativeCompilerImplementationLookup {
        ensemble: "::info",
        member: "exists",
        slot: "::tcl::info::exists",
        command: "info",
        prepended: &["exists"],
    };
const INFO_LEVEL_IMPLEMENTATION: NativeCompilerImplementationLookup =
    NativeCompilerImplementationLookup {
        ensemble: "::info",
        member: "level",
        slot: "::tcl::info::level",
        command: "info",
        prepended: &["level"],
    };

const NAMESPACE_CURRENT_IMPLEMENTATION: NativeCompilerImplementationLookup =
    NativeCompilerImplementationLookup {
        ensemble: "::namespace",
        member: "current",
        slot: "::tcl::namespace::current",
        command: "namespace",
        prepended: &["current"],
    };

const NAMESPACE_ORIGIN_IMPLEMENTATION: NativeCompilerImplementationLookup =
    NativeCompilerImplementationLookup {
        ensemble: "::namespace",
        member: "origin",
        slot: "::tcl::namespace::origin",
        command: "namespace",
        prepended: &["origin"],
    };

const NAMESPACE_CODE_IMPLEMENTATION: NativeCompilerImplementationLookup =
    NativeCompilerImplementationLookup {
        ensemble: "::namespace",
        member: "code",
        slot: "::tcl::namespace::code",
        command: "namespace",
        prepended: &["code"],
    };

fn namespace_code_grammar(
    words: InvocationWords<'_>,
    shapes: &[NativeCompilationWordShape],
    version: TclVersion,
    operation: SemanticOperationId,
) -> NativeCompilationSelection {
    if version < TclVersion::V8_6 {
        return NativeCompilationSelection::Generic;
    }
    // TclCompileNamespaceCodeCmd accepts one SIMPLE_WORD. The namespace is
    // captured at execution, and an existing scoped prefix uses the handler.
    let value = words.arguments().literal_at(0);
    let already_scoped =
        value.is_some_and(|value| value.len() > 20 && value.starts_with("::namespace inscope "));
    if shapes.len() == 1 && literal_shape(shapes[0]) && value.is_some() && !already_scoped {
        return NativeCompilationSelection::Inline {
            operation,
            guard: NativeCompilationGuard::BeforeArguments,
        };
    }
    if words.head_literal().is_some_and(|head| {
        head.strip_prefix("::").unwrap_or(head)
            == NAMESPACE_CODE_IMPLEMENTATION
                .slot
                .strip_prefix("::")
                .unwrap()
    }) {
        NativeCompilationSelection::Generic
    } else {
        NativeCompilationSelection::NamedInvocation {
            lookup: &NAMESPACE_CODE_IMPLEMENTATION,
            arguments_from: 1,
            protocol: NativeNamedInvocationProtocol::EnsembleRewrite,
        }
    }
}

fn namespace_origin_grammar(
    words: InvocationWords<'_>,
    shapes: &[NativeCompilationWordShape],
    version: TclVersion,
    operation: SemanticOperationId,
) -> NativeCompilationSelection {
    if version < TclVersion::V8_6 {
        return NativeCompilationSelection::Generic;
    }
    if shapes.len() == 1 && !matches!(shapes[0], NativeCompilationWordShape::Expanded) {
        return NativeCompilationSelection::Inline {
            operation,
            guard: NativeCompilationGuard::BeforeArguments,
        };
    }
    if words.head_literal().is_some_and(|head| {
        head.strip_prefix("::").unwrap_or(head)
            == NAMESPACE_ORIGIN_IMPLEMENTATION
                .slot
                .strip_prefix("::")
                .unwrap()
    }) {
        NativeCompilationSelection::Generic
    } else {
        NativeCompilationSelection::NamedInvocation {
            lookup: &NAMESPACE_ORIGIN_IMPLEMENTATION,
            arguments_from: 1,
            protocol: NativeNamedInvocationProtocol::EnsembleRewrite,
        }
    }
}

fn namespace_current_grammar(
    words: InvocationWords<'_>,
    shapes: &[NativeCompilationWordShape],
    version: TclVersion,
    operation: SemanticOperationId,
) -> NativeCompilationSelection {
    if version < TclVersion::V8_6 {
        return NativeCompilationSelection::Generic;
    }
    if shapes.is_empty() {
        return NativeCompilationSelection::Inline {
            operation,
            guard: NativeCompilationGuard::BeforeArguments,
        };
    }
    if words.head_literal().is_some_and(|head| {
        head.strip_prefix("::").unwrap_or(head)
            == NAMESPACE_CURRENT_IMPLEMENTATION
                .slot
                .strip_prefix("::")
                .unwrap()
    }) {
        NativeCompilationSelection::Generic
    } else {
        NativeCompilationSelection::NamedInvocation {
            lookup: &NAMESPACE_CURRENT_IMPLEMENTATION,
            arguments_from: 1,
            protocol: NativeNamedInvocationProtocol::EnsembleRewrite,
        }
    }
}

fn info_level_grammar(
    words: InvocationWords<'_>,
    shapes: &[NativeCompilationWordShape],
    version: TclVersion,
    operation: SemanticOperationId,
) -> NativeCompilationSelection {
    if version < TclVersion::V8_6 {
        return NativeCompilationSelection::Generic;
    }
    if shapes.len() <= 1 {
        return NativeCompilationSelection::Inline {
            operation,
            guard: NativeCompilationGuard::BeforeArguments,
        };
    }
    if words
        .head_literal()
        .is_some_and(|head| head.strip_prefix("::").unwrap_or(head) == "tcl::info::level")
    {
        NativeCompilationSelection::Generic
    } else {
        NativeCompilationSelection::NamedInvocation {
            lookup: &INFO_LEVEL_IMPLEMENTATION,
            arguments_from: 1,
            protocol: NativeNamedInvocationProtocol::EnsembleRewrite,
        }
    }
}

fn array_compilation_grammar(
    operation: SemanticOperationId,
    command: NativeArrayCommand,
    lookup: &'static NativeCompilerImplementationLookup,
    words: InvocationWords<'_>,
    shapes: &[NativeCompilationWordShape],
    version: TclVersion,
    context: NativeCompilationContext,
) -> NativeCompilationSelection {
    use NativeCompilationSelection as Selection;
    if version < TclVersion::V8_6 {
        return Selection::Generic;
    }
    let private = words.head_literal().is_some_and(|head| {
        head.strip_prefix("::").unwrap_or(head)
            == lookup.slot.strip_prefix("::").unwrap_or(lookup.slot)
    });
    array_compilation_grammar_for(
        (operation, command, lookup, private),
        words,
        shapes,
        version,
        context,
    )
}

fn array_compilation_grammar_for(
    descriptor: (
        SemanticOperationId,
        NativeArrayCommand,
        &'static NativeCompilerImplementationLookup,
        bool,
    ),
    words: InvocationWords<'_>,
    shapes: &[NativeCompilationWordShape],
    version: TclVersion,
    context: NativeCompilationContext,
) -> NativeCompilationSelection {
    use NativeCompilationSelection as Selection;
    let (operation, command, lookup, private) = descriptor;
    if version < TclVersion::V8_6 {
        return Selection::Generic;
    }
    let from = usize::from(!private);
    let Some(operands) = shapes.get(from..) else {
        return Selection::Generic;
    };
    let arity = match command {
        NativeArrayCommand::Exists => crate::Arity::exact(1),
        NativeArrayCommand::Set => crate::Arity::exact(2),
        NativeArrayCommand::Unset => crate::Arity::new(1, 2),
    };
    if !u16::try_from(operands.len()).is_ok_and(|count| arity.accepts(count))
        || (!private && !literal_shape(shapes[0]))
    {
        return Selection::Generic;
    }
    let arguments = words.arguments().slice_from(from);
    if command == NativeArrayCommand::Set {
        let values = arguments
            .literal_at(1)
            .and_then(|value| tcl_syntax::list::split_list(value).ok());
        let empty = values.as_ref().is_some_and(Vec::is_empty);
        let odd = values
            .as_ref()
            .is_some_and(|values| !values.len().is_multiple_of(2));
        if odd
            || !literal_shape(operands[0])
            || (context.frame != NativeCompilationFrame::ProcedureCode && !empty)
        {
            return Selection::NamedInvocation {
                lookup,
                arguments_from: from,
                protocol: NativeNamedInvocationProtocol::Direct,
            };
        }
    } else if command == NativeArrayCommand::Unset && operands.len() == 2 {
        return Selection::NamedInvocation {
            lookup,
            arguments_from: from,
            protocol: NativeNamedInvocationProtocol::Direct,
        };
    }
    if literal_shape(operands[0])
        && arguments
            .literal_at(0)
            .is_some_and(|name| tcl_syntax::naming::split_array_name(name).1.is_some())
    {
        return Selection::Generic;
    }
    Selection::Inline {
        operation,
        guard: NativeCompilationGuard::BeforeArguments,
    }
}

fn named_ensemble_grammar(
    lookup: &'static NativeCompilerImplementationLookup,
    hook_from: TclVersion,
    arity: crate::Arity,
    words: InvocationWords<'_>,
    shapes: &[NativeCompilationWordShape],
    version: TclVersion,
) -> NativeCompilationSelection {
    let head = words
        .head_literal()
        .map(|head| head.trim_start_matches("::"));
    let from = if head == Some(lookup.slot.trim_start_matches("::")) {
        0
    } else if head == Some(lookup.ensemble.trim_start_matches("::")) {
        1
    } else {
        lookup.prepended.len()
    };
    if version < hook_from
        || shapes.len() < from
        || !shapes[..from].iter().all(|shape| literal_shape(*shape))
        || !u16::try_from(shapes.len() - from).is_ok_and(|count| arity.accepts(count))
    {
        return NativeCompilationSelection::Generic;
    }
    NativeCompilationSelection::NamedInvocation {
        lookup,
        arguments_from: from,
        protocol: NativeNamedInvocationProtocol::Direct,
    }
}

const STRING_EQUAL_IMPLEMENTATION: NativeCompilerImplementationLookup =
    NativeCompilerImplementationLookup {
        ensemble: "::string",
        member: "equal",
        slot: "::tcl::string::equal",
        command: "string",
        prepended: &["equal"],
    };

const STRING_LENGTH_IMPLEMENTATION: NativeCompilerImplementationLookup =
    NativeCompilerImplementationLookup {
        ensemble: "::string",
        member: "length",
        slot: "::tcl::string::length",
        command: "string",
        prepended: &["length"],
    };

fn static_value_shape(shape: NativeCompilationWordShape) -> bool {
    literal_shape(shape) || shape == NativeCompilationWordShape::BackslashLiteral
}

fn variable_append_grammar(
    kind: NativeAppendKind,
    operation: SemanticOperationId,
    words: InvocationWords<'_>,
    shapes: &[NativeCompilationWordShape],
    version: TclVersion,
    context: NativeCompilationContext,
) -> NativeCompilationSelection {
    variable_append_grammar_bytes(
        kind,
        operation,
        words.arguments().literal_at(0).map(str::as_bytes),
        shapes,
        version,
        context,
    )
}

fn variable_append_grammar_bytes(
    kind: NativeAppendKind,
    operation: SemanticOperationId,
    name: Option<&[u8]>,
    shapes: &[NativeCompilationWordShape],
    version: TclVersion,
    context: NativeCompilationContext,
) -> NativeCompilationSelection {
    use NativeCompilationSelection as Selection;
    if shapes.contains(&NativeCompilationWordShape::Opaque) {
        return Selection::Unknown;
    }
    if kind == NativeAppendKind::List && version < TclVersion::V8_6 {
        match context.frame {
            NativeCompilationFrame::ScriptCode => return Selection::Generic,
            NativeCompilationFrame::Unknown => return Selection::Unknown,
            NativeCompilationFrame::ProcedureCode => {}
        }
    }
    if shapes.is_empty() {
        return if version == TclVersion::V8_4 {
            Selection::CompileError
        } else {
            Selection::Generic
        };
    }
    let selected = match kind {
        NativeAppendKind::String if shapes.len() <= 2 => true,
        NativeAppendKind::String if version < TclVersion::V8_6 => false,
        NativeAppendKind::String => {
            match context.frame {
                NativeCompilationFrame::ScriptCode => return Selection::Generic,
                NativeCompilationFrame::Unknown => return Selection::Unknown,
                NativeCompilationFrame::ProcedureCode => {}
            }
            if !literal_shape(shapes[0]) {
                return Selection::Generic;
            }
            let Some(name) = name else {
                return Selection::Unknown;
            };
            !tcl_syntax::naming::is_qualified(name)
                && tcl_syntax::naming::split_element_ref_bytes(name).is_none()
        }
        NativeAppendKind::List if version < TclVersion::V8_6 => shapes.len() == 2,
        NativeAppendKind::List if version < TclVersion::V9_1 => shapes.len() >= 2,
        NativeAppendKind::List => shapes[0] != NativeCompilationWordShape::Expanded,
    };
    if !selected {
        return Selection::Generic;
    }
    Selection::Inline {
        operation,
        guard: if version == TclVersion::V8_4 {
            NativeCompilationGuard::ChunkEntry
        } else {
            NativeCompilationGuard::BeforeArguments
        },
    }
}

#[cfg(test)]
mod append_compilation_tests {
    use super::*;

    fn selection(
        kind: NativeAppendKind,
        version: TclVersion,
        frame: NativeCompilationFrame,
        arguments: &[crate::InvocationWord<'_>],
        shapes: &[NativeCompilationWordShape],
    ) -> NativeCompilationSelection {
        NativeCompilationSpec {
            grammar: NativeCompilationGrammar::VariableAppend(kind),
            operation: SemanticOperationId::StructuredLowering(
                crate::hooks::LoweringHookId::AppendOrLappend,
            ),
            body: NativeBodyCompilation::Inherit,
        }
        .select(
            InvocationWords::structured(crate::InvocationWord::Literal("selected"), arguments),
            shapes,
            Some(InvocationDialect::for_version(version)),
            NativeCompilationContext {
                mode: NativeCompilationMode::BytecodeObject,
                frame,
                loop_depth: 0,
                catch_depth: Some(0),
            },
        )
    }

    #[test]
    fn string_and_list_append_compilers_preserve_native_release_and_frame_rules() {
        use crate::InvocationWord::{Dynamic, Literal as Word};
        use NativeCompilationFrame::{ProcedureCode, ScriptCode};
        use NativeCompilationSelection::{Generic, Inline};
        use NativeCompilationWordShape::{Literal, Substituted};
        for (version, modern, latest) in [
            (TclVersion::V8_4, false, false),
            (TclVersion::V8_5, false, false),
            (TclVersion::V8_6, true, false),
            (TclVersion::V9_0, true, false),
            (TclVersion::V9_1, true, true),
        ] {
            let inline = Inline {
                operation: SemanticOperationId::StructuredLowering(
                    crate::hooks::LoweringHookId::AppendOrLappend,
                ),
                guard: if version == TclVersion::V8_4 {
                    NativeCompilationGuard::ChunkEntry
                } else {
                    NativeCompilationGuard::BeforeArguments
                },
            };
            for kind in [NativeAppendKind::String, NativeAppendKind::List] {
                assert_eq!(
                    selection(
                        kind,
                        version,
                        ProcedureCode,
                        &[Word("x"), Dynamic],
                        &[Literal, Substituted]
                    ),
                    inline
                );
                assert_eq!(
                    selection(
                        kind,
                        version,
                        ProcedureCode,
                        &[Word("x"), Dynamic, Word("B")],
                        &[Literal, Substituted, Literal]
                    ),
                    if modern { inline } else { Generic }
                );
            }
            assert_eq!(
                selection(
                    NativeAppendKind::String,
                    version,
                    ScriptCode,
                    &[Word("x"), Dynamic],
                    &[Literal, Substituted]
                ),
                inline
            );
            assert_eq!(
                selection(
                    NativeAppendKind::String,
                    version,
                    ProcedureCode,
                    &[Word("::x"), Dynamic, Word("B")],
                    &[Literal, Substituted, Literal]
                ),
                Generic
            );
            assert_eq!(
                selection(
                    NativeAppendKind::String,
                    version,
                    ProcedureCode,
                    &[Dynamic, Dynamic, Word("B")],
                    &[Substituted, Substituted, Literal]
                ),
                Generic
            );
            assert_eq!(
                selection(
                    NativeAppendKind::List,
                    version,
                    ScriptCode,
                    &[Word("::a(k)"), Dynamic, Word("B")],
                    &[Literal, Substituted, Literal]
                ),
                if modern { inline } else { Generic }
            );
            assert_eq!(
                selection(
                    NativeAppendKind::List,
                    version,
                    ProcedureCode,
                    &[Word("x")],
                    &[Literal]
                ),
                if latest { inline } else { Generic }
            );
        }
    }

    #[test]
    fn expanded_list_append_requires_the_actual_tcl91_native_protocol() {
        use crate::InvocationWord::{Expanded, Literal};
        let value_shapes = [
            NativeCompilationWordShape::Literal,
            NativeCompilationWordShape::Expanded,
        ];
        assert!(matches!(
            selection(
                NativeAppendKind::List,
                TclVersion::V9_1,
                NativeCompilationFrame::ScriptCode,
                &[Literal("x"), Expanded],
                &value_shapes
            ),
            NativeCompilationSelection::Inline { .. }
        ));
        assert_eq!(
            selection(
                NativeAppendKind::List,
                TclVersion::V9_0,
                NativeCompilationFrame::ProcedureCode,
                &[Literal("x"), Expanded],
                &value_shapes
            ),
            NativeCompilationSelection::Generic
        );
        assert_eq!(
            selection(
                NativeAppendKind::String,
                TclVersion::V9_1,
                NativeCompilationFrame::ProcedureCode,
                &[Literal("x"), Expanded],
                &value_shapes
            ),
            NativeCompilationSelection::Generic
        );
        assert_eq!(
            selection(
                NativeAppendKind::List,
                TclVersion::V9_1,
                NativeCompilationFrame::ProcedureCode,
                &[Expanded, Literal("x")],
                &[
                    NativeCompilationWordShape::Expanded,
                    NativeCompilationWordShape::Literal
                ]
            ),
            NativeCompilationSelection::Generic
        );
    }
}

fn list_insertion_grammar(
    words: InvocationWords<'_>,
    shapes: &[NativeCompilationWordShape],
    version: TclVersion,
    operation: SemanticOperationId,
) -> NativeCompilationSelection {
    use NativeCompilationSelection as Selection;
    if version < TclVersion::V8_6 || shapes.len() < 2 {
        return Selection::Generic;
    }
    if version < TclVersion::V9_0 {
        if !static_value_shape(shapes[1]) {
            return Selection::Generic;
        }
        let Some(index) = words.arguments().literal_at(1) else {
            return Selection::Unknown;
        };
        match tcl_cmd_core::index::compiler_encodable_in(
            index,
            tcl_dialect::IndexSyntax::for_version(version),
        ) {
            Some(true) => {}
            Some(false) => return Selection::Generic,
            None => return Selection::Unknown,
        }
    }
    Selection::Inline {
        operation,
        guard: NativeCompilationGuard::BeforeArguments,
    }
}

fn list_range_grammar(
    words: InvocationWords<'_>,
    shapes: &[NativeCompilationWordShape],
    version: TclVersion,
    operation: SemanticOperationId,
) -> NativeCompilationSelection {
    use NativeCompilationSelection as Selection;
    if version < TclVersion::V8_6 || shapes.len() != 3 {
        return Selection::Generic;
    }
    for index in [1, 2] {
        if !static_value_shape(shapes[index]) {
            return Selection::Generic;
        }
        let Some(value) = words.arguments().literal_at(index) else {
            return Selection::Unknown;
        };
        let before = if index == 1 { 0 } else { -1 };
        let after = if index == 1 && version < TclVersion::V9_0 {
            i32::MAX
        } else if index == 1 {
            -1
        } else {
            -2
        };
        match tcl_cmd_core::index::compiled_list_bound_in(value, version, before, after) {
            Ok(Some(bound)) if index != 1 || bound.encoded() != -1 => {}
            Ok(_) => return Selection::Generic,
            Err(_) => return Selection::Unknown,
        }
    }
    Selection::Inline {
        operation,
        guard: NativeCompilationGuard::BeforeArguments,
    }
}

fn list_assignment_grammar(
    shapes: &[NativeCompilationWordShape],
    version: TclVersion,
    operation: SemanticOperationId,
) -> NativeCompilationSelection {
    use NativeCompilationSelection as Selection;
    if version < TclVersion::V8_5 || shapes.len() < 2 {
        return Selection::Generic;
    }
    Selection::Inline {
        operation,
        guard: NativeCompilationGuard::BeforeArguments,
    }
}

fn string_member_arity_grammar(
    words: InvocationWords<'_>,
    shapes: &[NativeCompilationWordShape],
    version: TclVersion,
    operation: SemanticOperationId,
    lookup: &'static NativeCompilerImplementationLookup,
    scope: crate::native_scalar_compilation::NativeScalarScope,
    arity: usize,
) -> NativeCompilationSelection {
    use NativeCompilationSelection as Selection;
    let private = scope == crate::native_scalar_compilation::NativeScalarScope::PrivateOperands;
    let from = usize::from(!private);
    if !private {
        let Some(shape) = shapes.first() else {
            return Selection::Generic;
        };
        // C84 parses the raw member token, including delimiters. Later
        // ensemble compilers select a literal member's unwrapped value.
        if (version == TclVersion::V8_4 && *shape != NativeCompilationWordShape::Literal)
            || !literal_shape(*shape)
            || !words
                .arguments()
                .literal_at(0)
                .is_some_and(|member| !member.is_empty() && lookup.member.starts_with(member))
        {
            return Selection::Generic;
        }
    } else if version == TclVersion::V8_4 {
        return Selection::Generic;
    }
    if shapes.len().checked_sub(from) == Some(arity) {
        return Selection::Inline {
            operation,
            guard: if version == TclVersion::V8_4 {
                NativeCompilationGuard::ChunkEntry
            } else {
                NativeCompilationGuard::BeforeArguments
            },
        };
    }
    if !private && version >= TclVersion::V8_6 {
        Selection::NamedInvocation {
            lookup,
            arguments_from: from,
            protocol: NativeNamedInvocationProtocol::EnsembleRewrite,
        }
    } else {
        Selection::Generic
    }
}

fn info_commands_grammar(
    words: InvocationWords<'_>,
    shapes: &[NativeCompilationWordShape],
    version: TclVersion,
    operation: SemanticOperationId,
) -> NativeCompilationSelection {
    if version < TclVersion::V8_6 || !matches!(shapes.len(), 1 | 2) || !literal_shape(shapes[0]) {
        return NativeCompilationSelection::Generic;
    }
    let absolute = shapes.len() == 2
        && literal_shape(shapes[1])
        && words.arguments().literal_at(1).is_some_and(|pattern| {
            pattern.starts_with("::")
                && !pattern
                    .chars()
                    .any(|ch| matches!(ch, '*' | '[' | '?' | '\\'))
        });
    if absolute {
        NativeCompilationSelection::Inline {
            operation,
            guard: NativeCompilationGuard::BeforeArguments,
        }
    } else {
        NativeCompilationSelection::NamedInvocation {
            lookup: &NativeCompilerImplementationLookup {
                ensemble: "::info",
                member: "commands",
                slot: "::tcl::info::commands",
                command: "info",
                prepended: &["commands"],
            },
            arguments_from: 1,
            protocol: NativeNamedInvocationProtocol::Direct,
        }
    }
}

fn regexp_grammar(
    words: InvocationWords<'_>,
    shapes: &[NativeCompilationWordShape],
) -> NativeCompilationSelection {
    match regexp_compiler_pattern_index(words, shapes) {
        Ok(_) => NativeCompilationSelection::Unknown,
        Err(selection) => selection,
    }
}

fn regexp_compiler_pattern_index(
    words: InvocationWords<'_>,
    shapes: &[NativeCompilationWordShape],
) -> Result<usize, NativeCompilationSelection> {
    use NativeCompilationSelection as Selection;
    if shapes.len() < 2 {
        return Err(Selection::Generic);
    }
    let mut index = 0;
    while index < shapes.len() - 2 {
        if !literal_shape(shapes[index]) {
            return Err(Selection::Generic);
        }
        let Some(option) = words.arguments().literal_at(index) else {
            return Err(Selection::Unknown);
        };
        index += 1;
        if option == "--" {
            break;
        }
        if option.len() <= 1 || !"-nocase".starts_with(option) {
            return Err(Selection::Generic);
        }
    }
    if shapes.len() - index == 2 {
        Ok(index)
    } else {
        Err(Selection::Generic)
    }
}

fn literal_operands_grammar(
    shapes: &[NativeCompilationWordShape],
    count: usize,
) -> Result<bool, NativeCompilationSelection> {
    if shapes.len() != count {
        Ok(false)
    } else if shapes.iter().all(|shape| literal_shape(*shape)) {
        Ok(true)
    } else {
        Err(NativeCompilationSelection::Generic)
    }
}

fn conditional_grammar(
    words: InvocationWords<'_>,
    shapes: &[NativeCompilationWordShape],
    version: TclVersion,
) -> Result<bool, NativeCompilationSelection> {
    use NativeCompilationSelection as Selection;
    if !shapes.iter().all(|shape| literal_shape(*shape)) {
        return Err(Selection::Generic);
    }
    let Some(arguments) = words.arguments().literal_values() else {
        return Err(Selection::Unknown);
    };
    if crate::commands::tcl::native_if_shape_error(&arguments).is_some() {
        return Err(if version == TclVersion::V8_4 {
            Selection::CompileError
        } else {
            Selection::Generic
        });
    }
    Ok(true)
}

fn upvar_grammar(
    words: InvocationWords<'_>,
    shapes: &[NativeCompilationWordShape],
    version: TclVersion,
    context: NativeCompilationContext,
) -> Result<bool, NativeCompilationSelection> {
    use NativeCompilationSelection as Selection;
    if version == TclVersion::V8_4 || context.frame == NativeCompilationFrame::ScriptCode {
        return Err(Selection::Generic);
    }
    if context.frame == NativeCompilationFrame::Unknown {
        return Err(Selection::Unknown);
    }
    if shapes.len() < 2 || !literal_shape(shapes[0]) {
        return Err(Selection::Generic);
    }
    let arguments = words
        .arguments()
        .with_dialect(InvocationDialect::for_version(version));
    let first = arguments.literal_at(0).ok_or(Selection::Unknown)?;
    let width = original_upvar_level_width(first, shapes.len(), version)?;
    for index in (width + 1..shapes.len()).step_by(2) {
        if !literal_shape(shapes[index]) {
            return Err(Selection::Generic);
        }
        let Some(name) = arguments.literal_at(index) else {
            return Err(Selection::Unknown);
        };
        if tcl_syntax::naming::is_qualified(name.as_bytes())
            || tcl_syntax::naming::split_element_ref(name).is_some()
        {
            return Err(Selection::Generic);
        }
    }
    Ok(true)
}

/// Compile-time leading-word probe shared by the original upvar recipe and
/// grammar selector. This does not select a runtime frame or install a cache.
///
/// # Errors
/// Returns native decline or unavailable invocation-stack evidence.
pub fn original_upvar_level_width(
    first: &str,
    count: usize,
    version: TclVersion,
) -> Result<usize, NativeCompilationSelection> {
    use crate::frame_effect::{
        FrameArgLayout, FrameArgumentResolution, FrameLevel, FrameLevelWord,
    };
    use NativeCompilationSelection as Selection;
    let mut operands = vec![crate::InvocationWord::Dynamic; count];
    let Some(leading) = operands.first_mut() else {
        return Err(Selection::Generic);
    };
    *leading = crate::InvocationWord::Literal(first);
    let grammar = crate::FrameEffectSpec {
        level_word: FrameLevelWord::LeadingProbe,
        layout: FrameArgLayout::AliasPairs,
    };
    let arguments = InvocationWords::structured(crate::InvocationWord::Dynamic, &operands)
        .arguments()
        .with_dialect(InvocationDialect::for_version(version));
    match grammar.resolve_arguments(arguments) {
        FrameArgumentResolution::Valid {
            level_word_len,
            level,
        } => {
            if level_word_len != 0
                && !matches!(
                    level,
                    FrameLevel::Relative(0 | 1) | FrameLevel::Absolute(0 | 1)
                )
            {
                return Err(Selection::Unknown);
            }
            Ok(level_word_len)
        }
        FrameArgumentResolution::Invalid => Err(Selection::Generic),
        FrameArgumentResolution::Unknown => Err(Selection::Unknown),
    }
}

fn gated_arity_grammar(
    first: TclVersion,
    arity: crate::Arity,
    shapes: &[NativeCompilationWordShape],
    version: TclVersion,
    operation: SemanticOperationId,
) -> NativeCompilationSelection {
    use NativeCompilationSelection as Selection;
    if version < first {
        return Selection::Generic;
    }
    let Ok(count) = u16::try_from(shapes.len()) else {
        return Selection::Unknown;
    };
    if !arity.accepts(count) {
        return Selection::Generic;
    }
    Selection::Inline {
        operation,
        guard: if version == TclVersion::V8_4 {
            NativeCompilationGuard::ChunkEntry
        } else {
            NativeCompilationGuard::BeforeArguments
        },
    }
}

fn return_grammar(
    words: InvocationWords<'_>,
    shapes: &[NativeCompilationWordShape],
    version: TclVersion,
    context: NativeCompilationContext,
) -> Result<bool, NativeCompilationSelection> {
    use NativeCompilationSelection as Selection;
    if shapes.len() > 1 {
        if version == TclVersion::V8_4 {
            return Err(Selection::Generic);
        }
        let pair_count = shapes.len() & !1;
        if shapes.len() == 3
            && words.arguments().literal_at(0) == Some("-options")
            && matches!(
                shapes[0],
                NativeCompilationWordShape::Literal
                    | NativeCompilationWordShape::QuotedLiteral
                    | NativeCompilationWordShape::BracedLiteral
            )
        {
            return Ok(true);
        }
        let Some(arguments) = (0..pair_count)
            .map(|index| {
                words
                    .arguments()
                    .literal_at(index)
                    .map(|word| word.as_bytes().to_vec())
            })
            .collect::<Option<Vec<_>>>()
        else {
            return if version >= TclVersion::V8_6 {
                Ok(true)
            } else {
                Err(Selection::Generic)
            };
        };
        return match crate::native_return_compilation::static_controls(&arguments, version) {
            Ok(_) => Ok(true),
            Err(error) if error.native_access_refusal().is_some() => Err(Selection::Unknown),
            Err(_) => Err(Selection::Generic),
        };
    }
    if version != TclVersion::V8_4 {
        return Ok(true);
    }
    if context.frame != NativeCompilationFrame::ProcedureCode {
        return Err(Selection::Generic);
    }
    match context.catch_depth {
        Some(0) => Ok(true),
        Some(_) => Err(Selection::Generic),
        None => Err(Selection::Unknown),
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

fn literal_shape(shape: NativeCompilationWordShape) -> bool {
    matches!(
        shape,
        NativeCompilationWordShape::Literal
            | NativeCompilationWordShape::QuotedLiteral
            | NativeCompilationWordShape::BracedLiteral
    )
}

fn catch_grammar(
    words: InvocationWords<'_>,
    shapes: &[NativeCompilationWordShape],
    version: TclVersion,
    context: NativeCompilationContext,
) -> Result<bool, NativeCompilationSelection> {
    use NativeCompilationSelection as Selection;
    let max = if version == TclVersion::V8_4 { 2 } else { 3 };
    if shapes.is_empty() || shapes.len() > max {
        return Ok(false);
    }
    if shapes.len() == 1 {
        return Ok(true);
    }
    match context.frame {
        NativeCompilationFrame::ScriptCode => return Err(Selection::Generic),
        NativeCompilationFrame::Unknown => return Err(Selection::Unknown),
        NativeCompilationFrame::ProcedureCode => {}
    }
    for index in 1..shapes.len() {
        let Some(name) = words.arguments().literal_at(index) else {
            return Err(Selection::Generic);
        };
        if tcl_syntax::naming::is_qualified(name.as_bytes())
            || tcl_syntax::naming::split_element_ref(name).is_some()
        {
            return Err(Selection::Generic);
        }
    }
    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hooks::LoweringHookId;

    #[test]
    fn namespace_no_hook_registration_keeps_public_and_terminal_body_policy() {
        for version in [TclVersion::V8_6, TclVersion::V9_0, TclVersion::V9_1] {
            let dialect = InvocationDialect::for_version(version);
            let profile =
                tcl_dialect::DialectProfile::find(&format!("tcl{}", version.version_string()))
                    .unwrap();
            let registry = crate::CommandRegistry::build_default().project_for_profile(profile);
            let namespace = registry.get("namespace").unwrap();
            for (member, body) in [
                ("eval", NativeBodyCompilation::ScriptObject),
                ("inscope", NativeBodyCompilation::ScriptObject),
                ("ensemble", NativeBodyCompilation::Direct),
            ] {
                let public = namespace
                    .resolve_subcommand_for_dialect(member, registry.own_surface_query())
                    .unwrap()
                    .native_compilation
                    .unwrap();
                assert_eq!(public.body, body);
                assert_eq!(public.compiler_hook_presence(dialect), Some(false));
                let lookups = public.implementation_prerequisites(dialect).unwrap();
                assert_eq!(lookups.len(), 1);
                assert_eq!(lookups[0].member, member);
                let terminal = registry
                    .native_compilation_for_registration(lookups[0].slot, dialect)
                    .unwrap();
                assert_eq!(terminal.body, body);
                assert_eq!(terminal.compiler_hook_presence(dialect), Some(false));
                assert_eq!(
                    terminal.implementation_prerequisites(dialect),
                    Some(lookups)
                );
            }
        }
    }

    #[test]
    fn no_hook_implementation_paths_keep_direct_body_and_reject_inconsistent_descriptors() {
        const LOOKUPS: &[NativeCompilerImplementationLookup] =
            &[NativeCompilerImplementationLookup {
                ensemble: "::namespace",
                member: "eval",
                slot: "::tcl::namespace::eval",
                command: "namespace",
                prepended: &["eval"],
            }];
        const DIRECT: NativeCompilationSpec = NativeCompilationSpec {
            grammar: NativeCompilationGrammar::NoHook,
            operation: SemanticOperationId::Invoke,
            body: NativeBodyCompilation::Direct,
        };
        const HOOK: NativeCompilationSpec = NativeCompilationSpec {
            grammar: NativeCompilationGrammar::HookFrom(TclVersion::V8_6),
            ..DIRECT
        };
        let direct = NativeCompilationSpec {
            grammar: NativeCompilationGrammar::WithImplementationPath {
                compiler: &DIRECT,
                lookups: LOOKUPS,
                implementation_from: TclVersion::V8_6,
            },
            operation: DIRECT.operation,
            body: DIRECT.body,
        };
        let modern = InvocationDialect::for_version(TclVersion::V9_0);
        assert_eq!(
            direct.implementation_prerequisites(modern),
            Some(LOOKUPS.to_vec())
        );
        assert_eq!(direct.compiler_hook_presence(modern), Some(false));
        assert_eq!(direct.body, NativeBodyCompilation::Direct);
        assert!(
            NativeCompilationSpec {
                body: NativeBodyCompilation::ScriptObject,
                ..direct
            }
            .implementation_prerequisites(modern)
            .is_none()
        );
        assert!(
            NativeCompilationSpec {
                grammar: NativeCompilationGrammar::WithImplementationPath {
                    compiler: &HOOK,
                    lookups: LOOKUPS,
                    implementation_from: TclVersion::V8_6,
                },
                ..direct
            }
            .implementation_prerequisites(modern)
            .is_none()
        );
        assert!(
            direct
                .implementation_prerequisites(InvocationDialect::for_version(TclVersion::V8_5))
                .is_none()
        );
        let jim = InvocationDialect::of_point(tcl_dialect::model::DialectPoint::canonical(
            tcl_dialect::model::Release::JIM_0_84,
        ));
        assert!(direct.implementation_prerequisites(jim).is_none());
    }

    #[test]
    fn coroutine_compilation_preserves_native_arity_and_expansion_gates() {
        let context = NativeCompilationContext {
            mode: NativeCompilationMode::BytecodeObject,
            frame: NativeCompilationFrame::ScriptCode,
            ..NativeCompilationContext::default()
        };
        let registry = crate::model::ingress::static_context_for("tcl9.1").commands();
        let yield_spec = registry.get("yield").unwrap().native_compilation.unwrap();
        let relay = registry.get("yieldto").unwrap().native_compilation.unwrap();
        for version in [TclVersion::V8_6, TclVersion::V9_0, TclVersion::V9_1] {
            let dialect = Some(InvocationDialect::for_version(version));
            for count in 0..=2 {
                let values = vec!["value"; count];
                let shapes = vec![NativeCompilationWordShape::Substituted; count];
                let selected = yield_spec.select(
                    InvocationWords::literals("yield", &values),
                    &shapes,
                    dialect,
                    context,
                );
                assert_eq!(
                    matches!(selected, NativeCompilationSelection::Inline { .. }),
                    count <= 1
                );
                let selected = relay.select(
                    InvocationWords::literals("yieldto", &values),
                    &shapes,
                    dialect,
                    context,
                );
                assert_eq!(
                    matches!(selected, NativeCompilationSelection::Inline { .. }),
                    count > 0 || version == TclVersion::V9_1
                );
            }
            let words = [crate::InvocationWord::Expanded];
            let selected = relay.select(
                InvocationWords::structured(crate::InvocationWord::Literal("yieldto"), &words),
                &[NativeCompilationWordShape::Expanded],
                dialect,
                context,
            );
            assert_eq!(
                matches!(selected, NativeCompilationSelection::Inline { .. }),
                version == TclVersion::V9_1
            );
        }
        assert_eq!(
            relay.select(
                InvocationWords::literals("yieldto", &["command"]),
                &[NativeCompilationWordShape::Opaque],
                Some(InvocationDialect::for_version(TclVersion::V9_1)),
                context,
            ),
            NativeCompilationSelection::Unknown
        );
    }

    #[test]
    fn array_specialised_compilation_retains_workers_and_shape_protocols() {
        let registry = crate::model::ingress::static_context_for("tcl8.6").commands();
        let procedure = NativeCompilationContext {
            frame: NativeCompilationFrame::ProcedureCode,
            mode: NativeCompilationMode::BytecodeObject,
            ..NativeCompilationContext::default()
        };
        for (values, shapes, named) in [
            (
                vec!["exists", "a"],
                vec![NativeCompilationWordShape::Literal; 2],
                false,
            ),
            (
                vec!["set", "a", "x 1"],
                vec![NativeCompilationWordShape::Literal; 3],
                false,
            ),
            (
                vec!["set", "a", "x"],
                vec![NativeCompilationWordShape::Literal; 3],
                true,
            ),
            (
                vec!["unset", "a", "x*"],
                vec![NativeCompilationWordShape::Literal; 3],
                true,
            ),
        ] {
            let facts = registry
                .resolve_invocation("array", &values, registry.own_surface_query())
                .unwrap()
                .facts();
            let spec = facts.native_compilation.unwrap();
            let old = crate::InvocationDialect::for_version(TclVersion::V8_5);
            let current = crate::InvocationDialect::for_version(TclVersion::V8_6);
            assert_eq!(
                spec.select(
                    InvocationWords::literals("array", &values),
                    &shapes,
                    Some(old),
                    procedure
                ),
                NativeCompilationSelection::Generic
            );
            assert!(spec.implementation_lookup(old).is_none());
            assert!(spec.implementation_lookup(current).is_some());
            let selected = spec.select(
                InvocationWords::literals("array", &values),
                &shapes,
                Some(current),
                procedure,
            );
            assert_eq!(
                matches!(selected, NativeCompilationSelection::NamedInvocation { .. }),
                named
            );
            assert!(named || matches!(selected, NativeCompilationSelection::Inline { .. }));
        }
    }

    #[test]
    fn possible_handler_bodies_share_grammar_without_native_compiler_licence() {
        let registry = crate::model::ingress::static_context_for("f5-irules").commands();
        let dialect = crate::InvocationDialect::of_profile(tcl_dialect::DialectProfile::irules());
        let words = ["$ready", "set flag 1", "else", "set flag 2"];
        let facts = registry
            .resolve_invocation("if", &words, registry.own_surface_query())
            .unwrap()
            .facts();
        let arguments = crate::InvocationArguments::literals(&words).with_dialect(dialect);
        let entry = facts
            .possible_handler_body_flow(registry, arguments, crate::VariableAliasFrame::Procedure)
            .expect("audited native handler candidate bodies");
        assert_eq!(
            entry.flow,
            crate::script_body_flow::ScriptBodyFlow::Conditional(vec![(Some(0), 1), (None, 3)])
        );
        assert!(
            facts
                .successful_handler_effects(arguments, crate::VariableAliasFrame::Procedure)
                .is_none()
        );
        assert!(
            facts
                .possible_handler_body_flow(registry, arguments, crate::VariableAliasFrame::Unknown)
                .is_none()
        );
    }

    #[test]
    fn successful_handler_metadata_does_not_depend_on_vendor_compiler_selection() {
        let registry = crate::model::ingress::static_context_for("f5-irules").commands();
        let dialect = crate::InvocationDialect::of_profile(tcl_dialect::DialectProfile::irules());
        for (head, words) in [
            ("IP::client_addr", vec![]),
            ("append", vec!["::counter", "x"]),
            ("unset", vec!["::counter"]),
            ("array", vec!["set", "::items", "a 1"]),
            ("scan", vec!["value", "%s", "::parsed"]),
            ("catch", vec!["set x 1", "::err"]),
        ] {
            let facts = registry
                .resolve_invocation(head, &words, registry.own_surface_query())
                .unwrap()
                .facts();
            let arguments = crate::InvocationArguments::literals(&words).with_dialect(dialect);
            assert!(
                facts
                    .successful_handler_effects(arguments, crate::VariableAliasFrame::Procedure)
                    .is_some(),
                "{head}"
            );
            assert!(facts.successful_handler.is_some(), "{head}");
            if head != "IP::client_addr" {
                assert!(
                    facts
                        .successful_handler_effects(arguments, crate::VariableAliasFrame::Unknown)
                        .is_none(),
                    "{head}: unknown frame cannot license address transfer"
                );
            }
        }
    }

    #[test]
    fn catch_output_binding_phase_and_legacy_arity_stay_explicit() {
        assert_eq!(
            SuccessfulHandlerSpec::CatchOutputs.variable_binding_phase(),
            VariableOperandBindingPhase::NormalContinuation
        );
        let registry = crate::model::ingress::static_context_for("f5-irules").commands();
        let words = ["set x 1", "::result", "::options"];
        let facts = registry
            .resolve_invocation("catch", &words, registry.own_surface_query())
            .unwrap()
            .facts();
        let args = crate::InvocationArguments::literals(&words).with_dialect(
            crate::InvocationDialect::of_profile(tcl_dialect::DialectProfile::irules()),
        );
        assert!(
            facts
                .successful_handler_effects(args, crate::VariableAliasFrame::Procedure)
                .is_none()
        );
    }

    fn context(frame: NativeCompilationFrame) -> NativeCompilationContext {
        NativeCompilationContext {
            mode: NativeCompilationMode::BytecodeObject,
            frame,
            loop_depth: 0,
            catch_depth: Some(0),
        }
    }

    #[test]
    fn uplevel_compiler_preserves_original_frame_probe_and_release_floor() {
        let spec = NativeCompilationSpec {
            grammar: NativeCompilationGrammar::Uplevel,
            operation: SemanticOperationId::StructuredLowering(LoweringHookId::Uplevel),
            body: NativeBodyCompilation::Uplevel,
        };
        for version in TclVersion::ALL {
            let dialect = InvocationDialect::for_version(version);
            for (first, valid) in [
                ("#0", true),
                ("set", true),
                ("", true),
                ("-0", true),
                ("0d0", true),
                ("1.0", true),
                ("-1", false),
                ("2147483648", false),
                ("#2147483648", false),
            ] {
                let selection = spec.select(
                    InvocationWords::literals("uplevel", &[first, "set ::x RIGHT"]),
                    &[NativeCompilationWordShape::BracedLiteral; 2],
                    Some(dialect),
                    context(NativeCompilationFrame::ProcedureCode),
                );
                assert_eq!(
                    matches!(selection, NativeCompilationSelection::Inline { .. }),
                    version == TclVersion::V9_1 && valid,
                    "{version:?} first={first:?}",
                );
            }
            for (frame, first_shape) in [
                (
                    NativeCompilationFrame::ScriptCode,
                    NativeCompilationWordShape::Literal,
                ),
                (
                    NativeCompilationFrame::ProcedureCode,
                    NativeCompilationWordShape::Substituted,
                ),
            ] {
                assert_eq!(
                    spec.select(
                        InvocationWords::literals("uplevel", &["#0", "set ::x RIGHT"]),
                        &[first_shape, NativeCompilationWordShape::Substituted],
                        Some(dialect),
                        context(frame),
                    ),
                    NativeCompilationSelection::Generic
                );
            }
        }
    }

    #[test]
    fn checked_arity_retains_native_entry_error_and_argument_timing() {
        let registry = crate::model::ingress::static_context_for("tcl8.6").commands();
        let facts = registry
            .resolve_invocation("llength", &["items"], registry.own_surface_query())
            .unwrap()
            .facts();
        let spec = facts.native_compilation.unwrap();
        for version in TclVersion::ALL {
            let dialect = InvocationDialect::for_version(version);
            let entry = context(NativeCompilationFrame::ScriptCode);
            assert!(matches!(
                spec.select(
                    InvocationWords::literals("renamed", &["items"]),
                    &[NativeCompilationWordShape::Substituted],
                    Some(dialect),
                    entry,
                ),
                NativeCompilationSelection::Inline { .. }
            ));
            let selection = spec.select(
                InvocationWords::literals("renamed", &["items", "extra"]),
                &[NativeCompilationWordShape::Substituted; 2],
                Some(dialect),
                entry,
            );
            assert_eq!(
                selection,
                if version == TclVersion::V8_4 {
                    NativeCompilationSelection::CompileError
                } else {
                    NativeCompilationSelection::Generic
                }
            );
            assert_eq!(
                spec.failure_for_selection(
                    selection,
                    InvocationWords::literals("renamed", &["items", "extra"]),
                    &facts,
                    Some(dialect),
                )
                .is_some(),
                version == TclVersion::V8_4,
            );
        }
    }

    #[test]
    fn string_equal_retains_raw_member_and_versioned_private_fallback() {
        let spec = NativeCompilationSpec {
            grammar: NativeCompilationGrammar::StringEqual(
                crate::native_scalar_compilation::NativeScalarScope::PublicMember,
            ),
            operation: SemanticOperationId::Intrinsic(crate::IntrinsicId::StringEqual),
            body: NativeBodyCompilation::Inherit,
        };
        for version in TclVersion::ALL {
            let dialect = InvocationDialect::for_version(version);
            for shape in [
                NativeCompilationWordShape::Literal,
                NativeCompilationWordShape::QuotedLiteral,
                NativeCompilationWordShape::BracedLiteral,
            ] {
                let selection = spec.select(
                    InvocationWords::literals("string", &["equal", "a", "a"]),
                    &[
                        shape,
                        NativeCompilationWordShape::Substituted,
                        NativeCompilationWordShape::Substituted,
                    ],
                    Some(dialect),
                    context(NativeCompilationFrame::ScriptCode),
                );
                assert_eq!(
                    matches!(selection, NativeCompilationSelection::Inline { .. }),
                    version > TclVersion::V8_4 || shape == NativeCompilationWordShape::Literal,
                );
            }
            let options = spec.select(
                InvocationWords::literals("string", &["equal", "-nocase", "a", "A"]),
                &[NativeCompilationWordShape::Literal; 4],
                Some(dialect),
                context(NativeCompilationFrame::ScriptCode),
            );
            assert_eq!(
                matches!(options, NativeCompilationSelection::NamedInvocation { .. }),
                version >= TclVersion::V8_6,
            );
            assert_eq!(
                spec.implementation_lookup(dialect).is_some(),
                version >= TclVersion::V8_5
            );
        }
    }

    #[test]
    fn string_length_requires_exact_member_shape_and_retains_native_arity_fallback() {
        let spec = NativeCompilationSpec {
            grammar: NativeCompilationGrammar::StringLength(
                crate::native_scalar_compilation::NativeScalarScope::PublicMember,
            ),
            operation: SemanticOperationId::Intrinsic(crate::IntrinsicId::StringLength),
            body: NativeBodyCompilation::Inherit,
        };
        for version in TclVersion::ALL {
            let dialect = InvocationDialect::for_version(version);
            for shape in [
                NativeCompilationWordShape::Literal,
                NativeCompilationWordShape::QuotedLiteral,
                NativeCompilationWordShape::BracedLiteral,
            ] {
                let selected = spec.select(
                    InvocationWords::literals("string", &["length", "value"]),
                    &[shape, NativeCompilationWordShape::Substituted],
                    Some(dialect),
                    context(NativeCompilationFrame::ScriptCode),
                );
                assert_eq!(
                    matches!(selected, NativeCompilationSelection::Inline { .. }),
                    version > TclVersion::V8_4 || shape == NativeCompilationWordShape::Literal
                );
            }
            let selected = spec.select(
                InvocationWords::literals("string", &["length", "a", "b"]),
                &[NativeCompilationWordShape::Literal; 3],
                Some(dialect),
                context(NativeCompilationFrame::ProcedureCode),
            );
            assert_eq!(
                matches!(
                    selected,
                    NativeCompilationSelection::NamedInvocation {
                        protocol: NativeNamedInvocationProtocol::EnsembleRewrite,
                        ..
                    }
                ),
                version >= TclVersion::V8_6
            );
            assert_eq!(
                spec.implementation_lookup(dialect).is_some(),
                version >= TclVersion::V8_5
            );
        }
    }

    #[test]
    fn global_alias_compilation_requires_a_local_table_and_known_scalar_tail() {
        let spec = NativeCompilationSpec {
            grammar: NativeCompilationGrammar::GlobalBindings,
            operation: SemanticOperationId::StructuredLowering(LoweringHookId::Global),
            body: NativeBodyCompilation::Inherit,
        };
        for version in TclVersion::ALL {
            for frame in [
                NativeCompilationFrame::ScriptCode,
                NativeCompilationFrame::ProcedureCode,
            ] {
                for name in ["g", "::g", "N::g", "", "g(k)"] {
                    let selection = spec.select(
                        InvocationWords::literals("global", &[name]),
                        &[NativeCompilationWordShape::BracedLiteral],
                        Some(InvocationDialect::for_version(version)),
                        context(frame),
                    );
                    assert_eq!(
                        matches!(selection, NativeCompilationSelection::Inline { .. }),
                        version >= TclVersion::V8_5
                            && frame == NativeCompilationFrame::ProcedureCode
                            && !name.ends_with(')'),
                        "{version:?} {frame:?} {name:?}"
                    );
                }
            }
        }
        assert_eq!(
            spec.select(
                InvocationWords::literals("global", &["N::g"]),
                &[NativeCompilationWordShape::Substituted],
                Some(InvocationDialect::for_version(TclVersion::V9_1)),
                context(NativeCompilationFrame::ProcedureCode),
            ),
            NativeCompilationSelection::Unknown
        );
    }

    #[test]
    fn insertion_compiler_moves_dynamic_index_acceptance_at_c9() {
        let spec = NativeCompilationSpec {
            grammar: NativeCompilationGrammar::ListInsertion,
            operation: SemanticOperationId::Intrinsic(crate::IntrinsicId::ListInsert),
            body: NativeBodyCompilation::Inherit,
        };
        for version in TclVersion::ALL {
            for shape in [
                NativeCompilationWordShape::Literal,
                NativeCompilationWordShape::Substituted,
            ] {
                let selection = spec.select(
                    InvocationWords::literals("linsert", &["a b", "1", "X"]),
                    &[
                        NativeCompilationWordShape::BracedLiteral,
                        shape,
                        NativeCompilationWordShape::Substituted,
                    ],
                    Some(InvocationDialect::for_version(version)),
                    context(NativeCompilationFrame::ScriptCode),
                );
                assert_eq!(
                    matches!(selection, NativeCompilationSelection::Inline { .. }),
                    version >= TclVersion::V9_0
                        || (version == TclVersion::V8_6
                            && shape == NativeCompilationWordShape::Literal)
                );
            }
        }
    }

    #[test]
    fn native_list_operands_keep_compilation_and_store_timing_separate() {
        let range = NativeCompilationSpec {
            grammar: NativeCompilationGrammar::ListRange,
            operation: SemanticOperationId::Intrinsic(crate::IntrinsicId::ListRange),
            body: NativeBodyCompilation::Inherit,
        };
        let assign = NativeCompilationSpec {
            grammar: NativeCompilationGrammar::ListAssignment,
            operation: SemanticOperationId::Intrinsic(crate::IntrinsicId::ListAssign),
            body: NativeBodyCompilation::Inherit,
        };
        for version in TclVersion::ALL {
            let dialect = Some(InvocationDialect::for_version(version));
            let entered = context(NativeCompilationFrame::ProcedureCode);
            let selected = range.select(
                InvocationWords::literals("lrange", &["a b", "0", "end"]),
                &[NativeCompilationWordShape::Literal; 3],
                dialect,
                entered,
            );
            assert_eq!(
                matches!(selected, NativeCompilationSelection::Inline { .. }),
                version >= TclVersion::V8_6
            );
            let dynamic_index = range.select(
                InvocationWords::literals("lrange", &["a b", "0", "end"]),
                &[
                    NativeCompilationWordShape::Literal,
                    NativeCompilationWordShape::Substituted,
                    NativeCompilationWordShape::Literal,
                ],
                dialect,
                entered,
            );
            assert_eq!(dynamic_index, NativeCompilationSelection::Generic);
            let selected = assign.select(
                InvocationWords::literals("lassign", &["a b", "x"]),
                &[
                    NativeCompilationWordShape::Substituted,
                    NativeCompilationWordShape::QuotedLiteral,
                ],
                dialect,
                entered,
            );
            assert_eq!(
                matches!(selected, NativeCompilationSelection::Inline { .. }),
                version >= TclVersion::V8_5
            );
            let dynamic_name = assign.select(
                InvocationWords::literals("lassign", &["a b", "x"]),
                &[
                    NativeCompilationWordShape::Literal,
                    NativeCompilationWordShape::Substituted,
                ],
                dialect,
                entered,
            );
            assert_eq!(
                dynamic_name,
                if version >= TclVersion::V8_5 {
                    NativeCompilationSelection::Unknown
                } else {
                    NativeCompilationSelection::Generic
                }
            );
        }
    }

    #[test]
    fn list_index_compilation_requires_the_native_hook_and_original_word_count() {
        let spec = NativeCompilationSpec {
            grammar: NativeCompilationGrammar::ListIndex,
            operation: SemanticOperationId::Intrinsic(crate::IntrinsicId::ListIndex),
            body: NativeBodyCompilation::Inherit,
        };
        for version in TclVersion::ALL {
            let dialect = Some(InvocationDialect::for_version(version));
            for frame in [
                NativeCompilationFrame::ScriptCode,
                NativeCompilationFrame::ProcedureCode,
            ] {
                for arguments in [vec!["a b"], vec!["a b", "1"], vec!["a b", "0", "end"]] {
                    let shapes = vec![NativeCompilationWordShape::Substituted; arguments.len()];
                    let selection = spec.select(
                        InvocationWords::literals("lindex", &arguments),
                        &shapes,
                        dialect,
                        context(frame),
                    );
                    assert_eq!(
                        selection,
                        NativeCompilationSelection::Inline {
                            operation: spec.operation,
                            guard: if version == TclVersion::V8_4 {
                                NativeCompilationGuard::ChunkEntry
                            } else {
                                NativeCompilationGuard::BeforeArguments
                            },
                        },
                    );
                }
                assert_eq!(
                    spec.select(
                        InvocationWords::literals("lindex", &[]),
                        &[],
                        dialect,
                        context(frame)
                    ),
                    NativeCompilationSelection::Generic,
                );
                assert_eq!(
                    spec.select(
                        InvocationWords::literals("lindex", &["a b", "0"]),
                        &[
                            NativeCompilationWordShape::Expanded,
                            NativeCompilationWordShape::Literal
                        ],
                        dialect,
                        context(frame),
                    ),
                    NativeCompilationSelection::Generic,
                );
            }
        }
        let jim = crate::model::ingress::resolve_environment("jim").analyser_profile();
        assert_eq!(
            spec.select(
                InvocationWords::literals("lindex", &["a b", "0"]),
                &[NativeCompilationWordShape::Literal; 2],
                Some(InvocationDialect::of_profile(jim)),
                context(NativeCompilationFrame::ProcedureCode),
            ),
            NativeCompilationSelection::Generic,
        );
    }

    #[test]
    fn known_return_options_have_their_actual_versioned_compiler_protocol() {
        let registry = crate::CommandRegistry::build_default();
        let spec = registry.get("return").unwrap().native_compilation.unwrap();
        for version in [
            TclVersion::V8_4,
            TclVersion::V8_5,
            TclVersion::V8_6,
            TclVersion::V9_0,
            TclVersion::V9_1,
        ] {
            let dialect = InvocationDialect::for_version(version);
            let selected = spec.select(
                InvocationWords::literals("return", &["-code", "error", "absent"]),
                &[NativeCompilationWordShape::Literal; 3],
                Some(dialect),
                context(NativeCompilationFrame::ProcedureCode),
            );
            if version == TclVersion::V8_4 {
                assert_eq!(selected, NativeCompilationSelection::Generic);
            } else {
                assert!(matches!(
                    selected,
                    NativeCompilationSelection::Inline { .. }
                ));
            }
            assert_eq!(
                spec.select(
                    InvocationWords::literals("return", &["-level", "-1", "absent"]),
                    &[NativeCompilationWordShape::Literal; 3],
                    Some(dialect),
                    context(NativeCompilationFrame::ProcedureCode),
                ),
                NativeCompilationSelection::Generic
            );
        }
    }

    #[test]
    fn concat_compiler_accepts_retained_variadic_operands_from_c86() {
        let registry = crate::CommandRegistry::build_default();
        let spec = registry.get("concat").unwrap().native_compilation.unwrap();
        for version in [
            TclVersion::V8_4,
            TclVersion::V8_5,
            TclVersion::V8_6,
            TclVersion::V9_0,
            TclVersion::V9_1,
        ] {
            for args in [vec![], vec!["a"], vec!["a", "b"]] {
                let words = InvocationWords::literals("concat", &args);
                let shapes = vec![NativeCompilationWordShape::Substituted; args.len()];
                let dialect = InvocationDialect::for_version(version);
                let selected = spec.select(
                    words,
                    &shapes,
                    Some(dialect),
                    context(NativeCompilationFrame::ScriptCode),
                );
                assert_eq!(
                    spec.compiler_hook_presence(dialect),
                    Some(version >= TclVersion::V8_6)
                );
                assert_eq!(
                    matches!(selected, NativeCompilationSelection::Inline { .. }),
                    version >= TclVersion::V8_6
                );
                assert_eq!(
                    matches!(selected, NativeCompilationSelection::Generic),
                    version < TclVersion::V8_6
                );
            }
        }
    }

    #[test]
    fn sequence_normal_effects_require_no_native_expression_callback() {
        let registry = crate::CommandRegistry::build_default();
        for version in [TclVersion::V9_0, TclVersion::V9_1] {
            let dialect = InvocationDialect::for_version(version);
            for (operands, closed) in [
                (vec!["3"], true),
                (vec!["1", "to", "5", "by", "2"], true),
                (vec!["1+2"], false),
                (vec!["[set ::changed yes]"], false),
                (vec!["1", "count"], false),
            ] {
                let words = InvocationWords::literals("lseq", &operands).with_dialect(dialect);
                let resolution =
                    registry.resolve_structured_invocation(words, dialect.authoring_query());
                let facts = resolution.resolved().unwrap().facts();
                assert_eq!(
                    facts
                        .successful_value_leaf_world(words.arguments())
                        .is_some(),
                    closed,
                    "{version:?}: {operands:?}"
                );
                if facts.arity_accepts_frozen_arguments() == Some(true) {
                    assert!(
                        facts
                            .normal_list_method_provider(words.arguments())
                            .is_some()
                    );
                }
                let spec = facts.native_compilation.unwrap();
                assert_eq!(
                    spec.select(
                        words,
                        &vec![NativeCompilationWordShape::Literal; operands.len()],
                        Some(dialect),
                        context(NativeCompilationFrame::ProcedureCode)
                    ),
                    if version == TclVersion::V9_0 {
                        NativeCompilationSelection::Generic
                    } else {
                        NativeCompilationSelection::Unknown
                    }
                );
            }
        }
    }

    #[test]
    fn successful_handler_effects_do_not_select_compiler_or_body_protocol() {
        let registry = crate::CommandRegistry::build_default();
        let arguments = crate::InvocationArguments::literals(&["x", "value"]);
        for name in ["set", "incr", "list"] {
            let spec = registry.get(name).unwrap().native_compilation.unwrap();
            let effects = spec
                .successful_handler_effects(arguments, crate::VariableAliasFrame::Procedure)
                .expect("normative normal handler effects");
            assert_eq!(effects.operation, spec.operation);
            if name != "list" {
                assert!(
                    spec.successful_handler_effects(arguments, crate::VariableAliasFrame::Unknown)
                        .is_none()
                );
            }
        }
        for name in ["expr", "if", "while", "return", "proc"] {
            let spec = registry.get(name).unwrap().native_compilation.unwrap();
            assert!(
                spec.successful_handler_effects(arguments, crate::VariableAliasFrame::Procedure)
                    .is_none(),
                "{name}: body/control protocol must remain unproved"
            );
        }
        let spec = registry.get("set").unwrap().native_compilation.unwrap();
        assert!(
            spec.successful_handler_effects(
                crate::InvocationArguments::structured(&[
                    crate::InvocationWord::Dynamic,
                    crate::InvocationWord::Literal("value"),
                ]),
                crate::VariableAliasFrame::Procedure,
            )
            .is_none()
        );
        assert!(
            spec.successful_handler_effects(
                crate::InvocationArguments::literals(&["x", "value", "extra"]),
                crate::VariableAliasFrame::Procedure,
            )
            .is_none()
        );
    }

    #[test]
    fn expanded_no_body_compilation_is_independent_of_runtime_argv_length() {
        use crate::InvocationWord::{Expanded, Literal};
        let registry = crate::model::ingress::static_context_for("tcl9.1").commands();
        let dialect = InvocationDialect::for_version(TclVersion::V9_1);
        let arguments = [Literal("x"), Expanded];
        let shapes = [
            NativeCompilationWordShape::Literal,
            NativeCompilationWordShape::Expanded,
        ];
        for head in ["lappend", "proc", "foreach", "eval", "try"] {
            let words =
                InvocationWords::structured(Literal(head), &arguments).with_dialect(dialect);
            let resolution =
                registry.resolve_structured_invocation(words, dialect.authoring_query());
            let facts = resolution
                .resolved()
                .expect("actual stock invocation")
                .facts();
            let spec = facts.native_compilation.expect("authored native compiler");
            let selected = spec.select(
                words,
                &shapes,
                Some(dialect),
                context(NativeCompilationFrame::ProcedureCode),
            );
            assert_eq!(words.arguments().exact_argv_len(), None);
            if head == "lappend" {
                assert!(matches!(
                    selected,
                    NativeCompilationSelection::Inline { .. }
                ));
                assert!(facts.arg_roles_complete);
                assert_eq!(
                    spec.compilation_steps(selected, words, &shapes, &facts, Some(dialect)),
                    NativeCompilationSteps::Known(Vec::new())
                );
                let mut incomplete = facts.clone();
                incomplete.arg_roles_complete = false;
                assert_eq!(
                    spec.compiled_bodies(selected, words, &shapes, &incomplete, Some(dialect)),
                    NativeCompiledBodies::Unknown
                );
            } else {
                assert!(
                    !matches!(selected, NativeCompilationSelection::Inline { .. }),
                    "{head}: expansion cannot select an original inline body"
                );
            }
            assert_eq!(
                spec.compiled_bodies(
                    NativeCompilationSelection::Unknown,
                    words,
                    &shapes,
                    &facts,
                    Some(dialect)
                ),
                NativeCompiledBodies::Unknown,
                "{head}: no-body metadata cannot close unknown compiler selection"
            );
        }
    }

    fn compiled_arguments(head: &str, arguments: &[&str]) -> NativeCompiledBodies {
        let registry = crate::cache::registry_for_profile(
            tcl_dialect::DialectProfile::find("tcl8.4").unwrap(),
        );
        let facts = registry
            .resolve_invocation(head, arguments, registry.own_surface_query())
            .expect("stock native invocation")
            .facts();
        let spec = registry.get(head).unwrap().native_compilation.unwrap();
        let words = InvocationWords::literals(head, arguments);
        let shapes = vec![NativeCompilationWordShape::BracedLiteral; arguments.len()];
        let dialect = Some(InvocationDialect::for_version(TclVersion::V8_4));
        let selected = spec.select(
            words,
            &shapes,
            dialect,
            context(NativeCompilationFrame::ProcedureCode),
        );
        assert!(matches!(
            selected,
            NativeCompilationSelection::Inline { .. }
        ));
        spec.compiled_bodies(selected, words, &shapes, &facts, dialect)
    }

    #[test]
    fn compiled_bodies_use_literal_boolean_pruning_instead_of_runtime_folding() {
        use NativeCompilationFailureScope::EnclosingChunk;
        let known =
            |indices: &[usize], loops: &[usize], annotations: &[NativeCompiledBodyErrorContext]| {
                NativeCompiledBodies::Known(
                    indices
                        .iter()
                        .zip(annotations)
                        .map(|(argument, annotation)| NativeCompiledBodyOperand {
                            error_context: *annotation,
                            argument: *argument,
                            context: if loops.contains(argument) {
                                NativeCompiledBodyContext::Loop
                            } else {
                                NativeCompiledBodyContext::Inherit
                            },
                            failure_scope: EnclosingChunk,
                        })
                        .collect(),
                )
            };
        for condition in ["0", "00", "0.0", "off", "false", "0x0"] {
            assert_eq!(
                compiled_arguments("if", &[condition, "bad", "else", "good"]),
                known(&[3], &[], &[NativeCompiledBodyErrorContext::IfElse])
            );
            assert_eq!(
                compiled_arguments("while", &[condition, "bad"]),
                known(&[], &[], &[])
            );
        }
        for condition in ["0+0", "2-2", "$flag"] {
            assert_eq!(
                compiled_arguments("if", &[condition, "bad", "else", "good"]),
                known(
                    &[1, 3],
                    &[],
                    &[
                        NativeCompiledBodyErrorContext::IfThen,
                        NativeCompiledBodyErrorContext::IfElse
                    ]
                )
            );
            assert_eq!(
                compiled_arguments("while", &[condition, "bad"]),
                known(&[1], &[1], &[NativeCompiledBodyErrorContext::WhileBody])
            );
        }
        assert_eq!(
            compiled_arguments("if", &["1", "good", "else", "bad"]),
            known(&[1], &[], &[NativeCompiledBodyErrorContext::IfThen])
        );
        assert_eq!(
            compiled_arguments("for", &["init", "0", "next", "body"]),
            known(
                &[0, 3, 2],
                &[3, 2],
                &[
                    NativeCompiledBodyErrorContext::ForInitial,
                    NativeCompiledBodyErrorContext::ForBody,
                    NativeCompiledBodyErrorContext::ForNext
                ]
            )
        );
        assert_eq!(
            compiled_arguments("foreach", &["x", "", "body"]),
            known(&[2], &[2], &[NativeCompiledBodyErrorContext::ForeachBody])
        );
        assert_eq!(
            compiled_arguments("catch", &["body"]),
            NativeCompiledBodies::Known(vec![NativeCompiledBodyOperand {
                argument: 0,
                context: NativeCompiledBodyContext::Inherit,
                error_context: NativeCompiledBodyErrorContext::None,
                failure_scope: NativeCompilationFailureScope::FallbackToGeneric
            }])
        );
    }

    #[test]
    fn compiler_rejection_keeps_native_usage_after_command_renaming() {
        let registry = crate::cache::registry_for_profile(
            tcl_dialect::DialectProfile::find("tcl8.4").unwrap(),
        );
        let arguments = ["x", "extra", "bad"];
        let facts = registry
            .resolve_invocation("set", &arguments, registry.own_surface_query())
            .unwrap()
            .facts();
        let spec = registry.get("set").unwrap().native_compilation.unwrap();
        let words = InvocationWords::literals("renamed", &arguments);
        let dialect = Some(InvocationDialect::for_version(TclVersion::V8_4));
        let selected = spec.select(
            words,
            &[NativeCompilationWordShape::Literal; 3],
            dialect,
            context(NativeCompilationFrame::ProcedureCode),
        );
        assert_eq!(selected, NativeCompilationSelection::CompileError);
        assert_eq!(
            spec.failure_for_selection(selected, words, &facts, dialect),
            Some(NativeCompilationFailure {
                message: Some("wrong # args: should be \"set varName ?newValue?\"".into()),
                error_code: Some("NONE".into()),
                error_info: None,
            })
        );
        assert_eq!(
            spec.compiled_bodies(
                NativeCompilationSelection::Unknown,
                words,
                &[],
                &facts,
                dialect
            ),
            NativeCompiledBodies::Unknown
        );
    }

    #[test]
    fn upvar_compiler_uses_literal_probe_and_local_scalar_destinations() {
        let registry = crate::CommandRegistry::build_default();
        let spec = registry.get("upvar").unwrap().native_compilation.unwrap();
        for version in TclVersion::ALL {
            let select = |values: &[&str], shapes: &[NativeCompilationWordShape]| {
                spec.select(
                    InvocationWords::literals("upvar", values),
                    shapes,
                    Some(InvocationDialect::for_version(version)),
                    context(NativeCompilationFrame::ProcedureCode),
                )
            };
            let literals = [NativeCompilationWordShape::Literal; 3];
            for values in [["0", "x", "a"], ["#0", "x", "a"], ["1", "x", "a"]] {
                let selection = select(&values, &literals);
                if version == TclVersion::V8_4 {
                    assert_eq!(selection, NativeCompilationSelection::Generic);
                } else {
                    assert!(matches!(
                        selection,
                        NativeCompilationSelection::Inline { .. }
                    ));
                }
            }
            assert_eq!(
                select(&["1", "a"], &literals[..2]),
                NativeCompilationSelection::Generic,
                "numeric otherVar is a runtime pair in modern Tcl but not a compiled pair"
            );
            assert_eq!(
                select(&["0", "x", "a(k)"], &literals),
                NativeCompilationSelection::Generic
            );
            assert_eq!(
                select(
                    &["0", "x", "a"],
                    &[
                        NativeCompilationWordShape::Substituted,
                        literals[1],
                        literals[2]
                    ]
                ),
                NativeCompilationSelection::Generic
            );
        }
    }

    #[test]
    fn inline_exception_ranges_preserve_unknown_depth_and_checked_overflow() {
        let entry = context(NativeCompilationFrame::ProcedureCode);
        assert_eq!(entry.with_inline_exception_range().catch_depth, Some(1));
        let child = NativeCompiledBodyOperand {
            error_context: NativeCompiledBodyErrorContext::None,
            argument: 0,
            context: NativeCompiledBodyContext::Loop,
            failure_scope: NativeCompilationFailureScope::EnclosingChunk,
        };
        assert_eq!(child.entered_context(entry).unwrap().loop_depth, 1);
        assert_eq!(
            child.entered_context(NativeCompilationContext {
                loop_depth: u32::MAX,
                ..entry
            }),
            None
        );
        for depth in [None, Some(u32::MAX)] {
            let context = NativeCompilationContext {
                catch_depth: depth,
                ..entry
            };
            assert_eq!(context.with_inline_exception_range().catch_depth, None);
        }
    }

    #[test]
    fn every_sampled_stock_registration_has_a_matching_availability_descriptor() {
        let registry = crate::CommandRegistry::build_default();
        let mut observed = 0;
        for row in include_str!("native_compilation_hooks.tsv")
            .lines()
            .filter(|row| !row.starts_with('#'))
        {
            let cells: Vec<_> = row.split('\t').collect();
            assert_eq!(cells.len(), 6);
            for (version, registration) in TclVersion::ALL.into_iter().zip(&cells[1..]) {
                if *registration == "ABSENT" {
                    continue;
                }
                let dialect = InvocationDialect::for_version(version);
                let spec = registry
                    .get_for_surface(cells[0], dialect.authoring_query())
                    .unwrap_or_else(|| panic!("missing stock command {}", cells[0]));
                let native = spec
                    .native_compilation
                    .unwrap_or_else(|| panic!("missing native contract {}", cells[0]));
                let hook = native
                    .compiler_hook_presence(InvocationDialect::of_profile(
                        tcl_dialect::DialectProfile::find(version.dialect_profile_name()).unwrap(),
                    ))
                    .expect("pinned native C registration");
                assert_eq!(
                    hook,
                    *registration != "NULL",
                    "{} {version:?}: {registration}",
                    cells[0]
                );
                observed += 1;
            }
        }
        assert_eq!(observed, 421);
    }

    #[test]
    fn stock_tcloo_commands_have_generic_dispatch_without_native_compiler_hooks() {
        let registry = crate::CommandRegistry::build_default();
        for version in [TclVersion::V8_6, TclVersion::V9_0, TclVersion::V9_1] {
            for name in [
                "oo::class",
                "oo::object",
                "oo::define",
                "oo::objdefine",
                "oo::copy",
            ] {
                let native = registry.get(name).unwrap().native_compilation.unwrap();
                let words = InvocationWords::literals(name, &["create", "C", "{}"]);
                assert_eq!(
                    native.select(
                        words,
                        &[NativeCompilationWordShape::Literal; 3],
                        Some(InvocationDialect::for_version(version)),
                        context(NativeCompilationFrame::ProcedureCode),
                    ),
                    NativeCompilationSelection::Generic,
                    "{name} {version:?}",
                );
            }
        }
    }

    #[test]
    fn error_hook_accepts_retained_operands_only_from_its_authored_release() {
        let registry = crate::CommandRegistry::build_default();
        for version in TclVersion::ALL {
            let native = registry
                .get_for_surface(
                    "error",
                    InvocationDialect::for_version(version).authoring_query(),
                )
                .unwrap()
                .native_compilation
                .unwrap();
            for frame in [
                NativeCompilationFrame::ScriptCode,
                NativeCompilationFrame::ProcedureCode,
            ] {
                for count in 0..=4 {
                    let arguments = vec!["message"; count];
                    let shapes = vec![NativeCompilationWordShape::Substituted; count];
                    let selected = native.select(
                        InvocationWords::literals("error", &arguments),
                        &shapes,
                        Some(InvocationDialect::for_version(version)),
                        context(frame),
                    );
                    assert_eq!(
                        selected,
                        if version >= TclVersion::V8_6 && (1..=3).contains(&count) {
                            NativeCompilationSelection::Inline {
                                operation: native.operation,
                                guard: NativeCompilationGuard::BeforeArguments,
                            }
                        } else {
                            NativeCompilationSelection::Generic
                        }
                    );
                }
            }
        }
    }

    #[test]
    fn regexp_capture_and_option_forms_decline_native_compilation() {
        let registry = crate::CommandRegistry::build_default();
        let native = registry.get("regexp").unwrap().native_compilation.unwrap();
        for version in TclVersion::ALL {
            for arguments in [
                &[][..],
                &["pattern"][..],
                &["pattern", "text", "capture"][..],
                &["-nocase", "pattern", "text", "capture"][..],
                &["--", "pattern", "text", "capture"][..],
                &["-expanded", "pattern", "text"][..],
                &["-start", "0", "pattern", "text"][..],
            ] {
                assert_eq!(
                    native.select(
                        InvocationWords::literals("regexp", arguments),
                        &vec![NativeCompilationWordShape::Literal; arguments.len()],
                        Some(InvocationDialect::for_version(version)),
                        context(NativeCompilationFrame::ProcedureCode),
                    ),
                    NativeCompilationSelection::Generic,
                    "{version:?} {arguments:?}",
                );
            }
            for arguments in [
                &["pattern", "text"][..],
                &["-nocase", "pattern", "text"][..],
                &["-n", "--", "pattern", "text"][..],
            ] {
                assert_eq!(
                    native.select(
                        InvocationWords::literals("regexp", arguments),
                        &vec![NativeCompilationWordShape::Literal; arguments.len()],
                        Some(InvocationDialect::for_version(version)),
                        context(NativeCompilationFrame::ProcedureCode),
                    ),
                    NativeCompilationSelection::Unknown,
                );
            }
        }
    }

    #[test]
    fn regexp_operand_layout_is_independent_of_unresolved_operation_selection() {
        let registry = crate::CommandRegistry::build_default();
        let native = registry.get("regexp").unwrap().native_compilation.unwrap();
        let arguments = [
            crate::InvocationWord::Dynamic,
            crate::InvocationWord::Dynamic,
        ];
        let words =
            InvocationWords::structured(crate::InvocationWord::Literal("regexp"), &arguments);
        let shapes = [NativeCompilationWordShape::Substituted; 2];
        let compilation = context(NativeCompilationFrame::ProcedureCode);
        for version in TclVersion::ALL {
            let dialect = Some(InvocationDialect::for_version(version));
            assert_eq!(
                native.select(words, &shapes, dialect, compilation),
                NativeCompilationSelection::Unknown
            );
            assert_eq!(
                native.operand_layout(words, &shapes, dialect, compilation),
                (version >= TclVersion::V8_5).then_some(NativeCompilationOperandLayout::Pattern {
                    argument_index: 0,
                    guard: NativeCompilationGuard::BeforeArguments,
                })
            );
        }
        let dialect = Some(InvocationDialect::for_version(TclVersion::V9_1));
        for compilation in [
            NativeCompilationContext {
                mode: NativeCompilationMode::Direct,
                ..compilation
            },
            NativeCompilationContext {
                frame: NativeCompilationFrame::Unknown,
                ..compilation
            },
        ] {
            assert_eq!(
                native.operand_layout(words, &shapes, dialect, compilation),
                None
            );
        }
        assert_eq!(
            native.operand_layout(words, &shapes, None, compilation),
            None
        );
        assert_eq!(
            native.operand_layout(
                words,
                &[
                    NativeCompilationWordShape::Expanded,
                    NativeCompilationWordShape::Substituted
                ],
                dialect,
                compilation
            ),
            None
        );
    }

    #[test]
    fn regexp_options_preserve_original_operand_layout() {
        let registry = crate::CommandRegistry::build_default();
        let native = registry.get("regexp").unwrap().native_compilation.unwrap();
        let dialect = Some(InvocationDialect::for_version(TclVersion::V9_1));
        let compilation = context(NativeCompilationFrame::ProcedureCode);
        let options = [
            crate::InvocationWord::Literal("-nocase"),
            crate::InvocationWord::Dynamic,
            crate::InvocationWord::Dynamic,
        ];
        assert_eq!(
            native.operand_layout(
                InvocationWords::structured(crate::InvocationWord::Literal("regexp"), &options),
                &[
                    NativeCompilationWordShape::Literal,
                    NativeCompilationWordShape::Substituted,
                    NativeCompilationWordShape::Substituted
                ],
                dialect,
                compilation
            ),
            Some(NativeCompilationOperandLayout::Pattern {
                argument_index: 1,
                guard: NativeCompilationGuard::BeforeArguments
            })
        );
        let unknown_option = [crate::InvocationWord::Dynamic; 3];
        assert_eq!(
            native.operand_layout(
                InvocationWords::structured(
                    crate::InvocationWord::Literal("regexp"),
                    &unknown_option
                ),
                &[
                    NativeCompilationWordShape::Literal,
                    NativeCompilationWordShape::Substituted,
                    NativeCompilationWordShape::Substituted
                ],
                dialect,
                compilation,
            ),
            None,
        );
        let generic = [
            crate::InvocationWord::Literal("-about"),
            crate::InvocationWord::Dynamic,
        ];
        assert_eq!(
            native.operand_layout(
                InvocationWords::structured(crate::InvocationWord::Literal("regexp"), &generic),
                &[
                    NativeCompilationWordShape::Literal,
                    NativeCompilationWordShape::Substituted
                ],
                dialect,
                compilation
            ),
            Some(NativeCompilationOperandLayout::Pattern {
                argument_index: 0,
                guard: NativeCompilationGuard::BeforeArguments
            })
        );
    }

    #[test]
    fn actual_stock_hook_availability_and_return_catch_context_are_version_aware() {
        let registry = crate::CommandRegistry::build_default();
        for version in TclVersion::ALL {
            let dialect = Some(InvocationDialect::for_version(version));
            let empty = InvocationWords::literals("puts", &[]);
            assert_eq!(
                registry
                    .get("puts")
                    .unwrap()
                    .native_compilation
                    .unwrap()
                    .select(
                        empty,
                        &[],
                        dialect,
                        context(NativeCompilationFrame::ProcedureCode)
                    ),
                NativeCompilationSelection::Generic
            );
            let upvar = registry.get("upvar").unwrap().native_compilation.unwrap();
            assert_eq!(
                upvar.select(
                    InvocationWords::literals("upvar", &[]),
                    &[],
                    dialect,
                    context(NativeCompilationFrame::ProcedureCode)
                ),
                NativeCompilationSelection::Generic
            );
            let spec = registry.get("return").unwrap().native_compilation.unwrap();
            let words = InvocationWords::literals("return", &[]);
            assert!(matches!(
                spec.select(
                    words,
                    &[],
                    dialect,
                    context(NativeCompilationFrame::ProcedureCode)
                ),
                NativeCompilationSelection::Inline { .. }
            ));
            let mut protected = context(NativeCompilationFrame::ProcedureCode);
            protected.catch_depth = Some(1);
            assert_eq!(
                matches!(
                    spec.select(words, &[], dialect, protected),
                    NativeCompilationSelection::Generic
                ),
                version == TclVersion::V8_4
            );
            let uplevel = registry.get("uplevel").unwrap().native_compilation.unwrap();
            assert_eq!(
                uplevel.select(
                    InvocationWords::literals("uplevel", &[]),
                    &[],
                    dialect,
                    context(NativeCompilationFrame::ProcedureCode)
                ),
                NativeCompilationSelection::Generic
            );
        }
    }

    #[test]
    fn unset_hook_proves_literal_operands_and_retains_dynamic_order_uncertainty() {
        let spec = NativeCompilationSpec {
            grammar: NativeCompilationGrammar::LiteralUnset,
            operation: SemanticOperationId::Invoke,
            body: NativeBodyCompilation::Inherit,
        };
        for version in TclVersion::ALL {
            for words in [
                &[][..],
                &["x"][..],
                &["-nocomplain", "--", "a(k)", "::g"][..],
            ] {
                let shapes = vec![NativeCompilationWordShape::Literal; words.len()];
                assert_eq!(
                    spec.select(
                        InvocationWords::literals("unset", words),
                        &shapes,
                        Some(InvocationDialect::for_version(version)),
                        context(NativeCompilationFrame::ScriptCode)
                    ),
                    if version < TclVersion::V8_6 {
                        NativeCompilationSelection::Generic
                    } else {
                        NativeCompilationSelection::Inline {
                            operation: spec.operation,
                            guard: NativeCompilationGuard::BeforeArguments,
                        }
                    }
                );
            }
            assert_eq!(
                spec.select(
                    InvocationWords::literals("unset", &["x"]),
                    &[NativeCompilationWordShape::Substituted],
                    Some(InvocationDialect::for_version(version)),
                    context(NativeCompilationFrame::ProcedureCode)
                ),
                if version < TclVersion::V8_6 {
                    NativeCompilationSelection::Generic
                } else {
                    NativeCompilationSelection::Unknown
                }
            );
        }
    }

    #[test]
    fn expression_compilation_does_not_invent_a_legacy_math_function_table() {
        let operand = NativeCompiledExpressionOperand {
            argument: 0,
            error_context: NativeCompiledExpressionErrorContext::None,
        };
        for version in TclVersion::ALL {
            let profile =
                tcl_dialect::DialectProfile::find(version.dialect_profile_name()).unwrap();
            let context = tcl_syntax::expr::parser::ExprParseContext::for_profile(profile);
            assert_eq!(
                operand.validate("1 + 2", &context),
                NativeExpressionCompilation::Accepted
            );
            for expression in ["future_function(1)", "abs()", "int(1)"] {
                assert_eq!(
                    operand.validate(expression, &context),
                    if version == TclVersion::V8_4 {
                        NativeExpressionCompilation::Unknown
                    } else {
                        NativeExpressionCompilation::Accepted
                    }
                );
            }
            assert!(
                matches!(operand.validate("+", &context),
                NativeExpressionCompilation::Rejected(_) if version == TclVersion::V8_4)
                    || operand.validate("+", &context) == NativeExpressionCompilation::Accepted
            );
        }
    }

    #[test]
    fn procedure_only_variable_hook_declines_script_frames() {
        let registry = crate::CommandRegistry::build_default();
        let spec = registry
            .get("variable")
            .unwrap()
            .native_compilation
            .unwrap();
        for version in TclVersion::ALL {
            let dialect = InvocationDialect::for_version(version);
            assert_eq!(
                spec.compiler_hook_presence(dialect),
                Some(version >= TclVersion::V8_5)
            );
            for frame in [
                NativeCompilationFrame::ScriptCode,
                NativeCompilationFrame::ProcedureCode,
                NativeCompilationFrame::Unknown,
            ] {
                let selection = spec.select(
                    InvocationWords::literals("variable", &["x", "10"]),
                    &[NativeCompilationWordShape::Literal; 2],
                    Some(dialect),
                    context(frame),
                );
                assert_eq!(
                    selection,
                    if version < TclVersion::V8_5 || frame == NativeCompilationFrame::ScriptCode {
                        NativeCompilationSelection::Generic
                    } else {
                        NativeCompilationSelection::Unknown
                    }
                );
            }
        }
    }

    #[test]
    fn namespace_code_distinguishes_original_literal_builder_and_worker_fallback() {
        let spec = NativeCompilationSpec {
            grammar: NativeCompilationGrammar::NamespaceCode,
            operation: SemanticOperationId::Intrinsic(crate::IntrinsicId::NamespaceCode),
            body: NativeBodyCompilation::Inherit,
        };
        let context = context(NativeCompilationFrame::ProcedureCode);
        for version in TclVersion::ALL {
            let dialect = InvocationDialect::for_version(version);
            let modern = version >= TclVersion::V8_6;
            assert_eq!(spec.compiler_hook_presence(dialect), Some(modern));
            assert_eq!(spec.implementation_lookup(dialect).is_some(), modern);
            for shape in [
                NativeCompilationWordShape::Literal,
                NativeCompilationWordShape::QuotedLiteral,
                NativeCompilationWordShape::BracedLiteral,
            ] {
                for value in ["my tick", "::namespace inscope "] {
                    assert_eq!(
                        spec.select(
                            InvocationWords::literals("::tcl::namespace::code", &[value]),
                            &[shape],
                            Some(dialect),
                            context,
                        ),
                        if modern {
                            NativeCompilationSelection::Inline {
                                operation: spec.operation,
                                guard: NativeCompilationGuard::BeforeArguments,
                            }
                        } else {
                            NativeCompilationSelection::Generic
                        }
                    );
                }
            }
            for (value, shape) in [
                ("my tick", NativeCompilationWordShape::Substituted),
                ("my tick", NativeCompilationWordShape::BackslashLiteral),
                (
                    "::namespace inscope :: my",
                    NativeCompilationWordShape::BracedLiteral,
                ),
            ] {
                assert_eq!(
                    spec.select(
                        InvocationWords::literals("::tcl::namespace::code", &[value]),
                        &[shape],
                        Some(dialect),
                        context,
                    ),
                    NativeCompilationSelection::Generic,
                );
                let selection = spec.select(
                    InvocationWords::literals("namespace", &[value]),
                    &[shape],
                    Some(dialect),
                    context,
                );
                assert_eq!(
                    matches!(
                        selection,
                        NativeCompilationSelection::NamedInvocation {
                            protocol: NativeNamedInvocationProtocol::EnsembleRewrite,
                            ..
                        }
                    ),
                    modern,
                );
            }
        }
        let jim = InvocationDialect::of_point(tcl_dialect::model::DialectPoint::canonical(
            tcl_dialect::model::Release::JIM_0_84,
        ));
        assert_eq!(spec.compiler_hook_presence(jim), Some(false));
        assert_eq!(
            spec.select(
                InvocationWords::literals("::tcl::namespace::code", &["my tick"]),
                &[NativeCompilationWordShape::Literal],
                Some(jim),
                context,
            ),
            NativeCompilationSelection::Generic,
        );
    }

    #[test]
    fn namespace_origin_compilation_keeps_original_opcode_and_operand_layout() {
        let spec = NativeCompilationSpec {
            grammar: NativeCompilationGrammar::NamespaceOrigin,
            operation: SemanticOperationId::Intrinsic(crate::IntrinsicId::NamespaceOrigin),
            body: NativeBodyCompilation::Inherit,
        };
        let registry = crate::CommandRegistry::build_default();
        let context = context(NativeCompilationFrame::ProcedureCode);
        for version in TclVersion::ALL {
            let dialect = InvocationDialect::for_version(version);
            let modern = version >= TclVersion::V8_6;
            assert_eq!(spec.compiler_hook_presence(dialect), Some(modern));
            assert_eq!(spec.implementation_lookup(dialect).is_some(), modern);
            let arguments = ["origin", "target"];
            let facts = registry
                .resolve_invocation("namespace", &arguments, registry.own_surface_query())
                .unwrap()
                .facts();
            assert_eq!(
                spec.select_for_facts(
                    InvocationWords::literals("namespace", &arguments),
                    &[
                        NativeCompilationWordShape::Literal,
                        NativeCompilationWordShape::Substituted
                    ],
                    &facts,
                    Some(dialect),
                    context
                ),
                if modern {
                    NativeCompilationSelection::Inline {
                        operation: spec.operation,
                        guard: NativeCompilationGuard::BeforeArguments,
                    }
                } else {
                    NativeCompilationSelection::Generic
                }
            );
            for count in [0, 2] {
                let arguments = vec!["target"; count];
                assert_eq!(
                    spec.select(
                        InvocationWords::literals("::tcl::namespace::origin", &arguments),
                        &vec![NativeCompilationWordShape::Literal; count],
                        Some(dialect),
                        context
                    ),
                    NativeCompilationSelection::Generic
                );
            }
            assert_eq!(
                spec.select(
                    InvocationWords::literals("::tcl::namespace::origin", &["target"]),
                    &[NativeCompilationWordShape::Opaque],
                    Some(dialect),
                    context
                ),
                NativeCompilationSelection::Unknown
            );
        }
        let jim = InvocationDialect::of_point(tcl_dialect::model::DialectPoint::canonical(
            tcl_dialect::model::Release::JIM_0_84,
        ));
        assert_eq!(spec.compiler_hook_presence(jim), Some(false));
        assert_eq!(
            spec.select(
                InvocationWords::literals("namespace", &["origin", "target"]),
                &[NativeCompilationWordShape::Literal; 2],
                Some(jim),
                context
            ),
            NativeCompilationSelection::Generic
        );
    }

    #[test]
    fn current_namespace_compilation_preserves_operand_offsets_and_release_floor() {
        let spec = NativeCompilationSpec {
            grammar: NativeCompilationGrammar::NamespaceCurrent,
            operation: SemanticOperationId::Intrinsic(crate::IntrinsicId::NamespaceCurrent),
            body: NativeBodyCompilation::Inherit,
        };
        let registry = crate::CommandRegistry::build_default();
        let context = NativeCompilationContext {
            mode: NativeCompilationMode::BytecodeObject,
            frame: NativeCompilationFrame::ProcedureCode,
            ..Default::default()
        };
        for version in TclVersion::ALL {
            let dialect = InvocationDialect::for_version(version);
            let modern = version >= TclVersion::V8_6;
            assert_eq!(spec.compiler_hook_presence(dialect), Some(modern));
            assert_eq!(spec.implementation_lookup(dialect).is_some(), modern);
            for arguments in [vec!["current"], vec!["current", "extra"]] {
                let facts = registry
                    .resolve_invocation("namespace", &arguments, registry.own_surface_query())
                    .unwrap()
                    .facts();
                let selection = spec.select_for_facts(
                    InvocationWords::literals("namespace", &arguments),
                    &vec![NativeCompilationWordShape::Literal; arguments.len()],
                    &facts,
                    Some(dialect),
                    context,
                );
                if !modern {
                    assert_eq!(selection, NativeCompilationSelection::Generic);
                } else if arguments.len() == 1 {
                    assert_eq!(
                        selection,
                        NativeCompilationSelection::Inline {
                            operation: spec.operation,
                            guard: NativeCompilationGuard::BeforeArguments,
                        }
                    );
                } else {
                    assert!(matches!(selection,
                        NativeCompilationSelection::NamedInvocation { lookup, arguments_from: 1, protocol: NativeNamedInvocationProtocol::EnsembleRewrite }
                        if lookup.slot == "::tcl::namespace::current"));
                }
            }
            assert_eq!(
                spec.select(
                    InvocationWords::literals("::tcl::namespace::current", &["extra"]),
                    &[NativeCompilationWordShape::Literal],
                    Some(dialect),
                    context,
                ),
                NativeCompilationSelection::Generic
            );
        }
    }

    #[test]
    fn stack_level_compilation_retains_original_operands_and_release_floor() {
        let spec = NativeCompilationSpec {
            grammar: NativeCompilationGrammar::InfoLevel,
            operation: SemanticOperationId::Intrinsic(crate::IntrinsicId::InfoLevel),
            body: NativeBodyCompilation::Inherit,
        };
        let context = NativeCompilationContext {
            mode: NativeCompilationMode::BytecodeObject,
            frame: NativeCompilationFrame::ProcedureCode,
            catch_depth: Some(0),
            ..Default::default()
        };
        for version in TclVersion::ALL {
            let dialect = InvocationDialect::for_version(version);
            for shapes in [vec![], vec![NativeCompilationWordShape::Substituted]] {
                let arguments = vec!["0"; shapes.len()];
                assert_eq!(
                    spec.select(
                        InvocationWords::literals("info", &arguments),
                        &shapes,
                        Some(dialect),
                        context
                    ),
                    if version < TclVersion::V8_6 {
                        NativeCompilationSelection::Generic
                    } else {
                        NativeCompilationSelection::Inline {
                            operation: spec.operation,
                            guard: NativeCompilationGuard::BeforeArguments,
                        }
                    }
                );
            }
            let shapes = [NativeCompilationWordShape::Literal; 2];
            assert_eq!(
                matches!(
                    spec.select(
                        InvocationWords::literals("info", &["extra", "args"]),
                        &shapes,
                        Some(dialect),
                        context
                    ),
                    NativeCompilationSelection::Generic
                ),
                (version < TclVersion::V8_6)
            );
            assert_eq!(
                spec.select(
                    InvocationWords::literals("::tcl::info::level", &["extra", "args"]),
                    &shapes,
                    Some(dialect),
                    context
                ),
                NativeCompilationSelection::Generic
            );
            assert_eq!(
                spec.compiler_hook_presence(dialect),
                Some(version >= TclVersion::V8_6)
            );
            assert_eq!(
                spec.implementation_lookup(dialect).is_some(),
                version >= TclVersion::V8_5
            );
        }
    }

    #[test]
    fn command_enumeration_distinguishes_named_calls_from_absolute_resolution() {
        let spec = NativeCompilationSpec {
            grammar: NativeCompilationGrammar::InfoCommands,
            operation: SemanticOperationId::Intrinsic(crate::IntrinsicId::InfoCommandsResolve),
            body: NativeBodyCompilation::Inherit,
        };
        let context = NativeCompilationContext {
            mode: NativeCompilationMode::BytecodeObject,
            frame: NativeCompilationFrame::ScriptCode,
            catch_depth: Some(0),
            loop_depth: 0,
        };
        for version in TclVersion::ALL {
            for (args, shapes, resolves) in [
                (
                    vec!["commands"],
                    vec![NativeCompilationWordShape::Literal],
                    false,
                ),
                (
                    vec!["commands", "::x"],
                    vec![NativeCompilationWordShape::Literal; 2],
                    true,
                ),
                (
                    vec!["commands", "::x*"],
                    vec![NativeCompilationWordShape::Literal; 2],
                    false,
                ),
                (
                    vec!["commands", "::x\\y"],
                    vec![NativeCompilationWordShape::Literal; 2],
                    false,
                ),
                (
                    vec!["commands", "$pattern"],
                    vec![
                        NativeCompilationWordShape::Literal,
                        NativeCompilationWordShape::Substituted,
                    ],
                    false,
                ),
            ] {
                let selected = spec.select(
                    InvocationWords::literals("info", &args),
                    &shapes,
                    Some(InvocationDialect::for_version(version)),
                    context,
                );
                if version < TclVersion::V8_6 {
                    assert_eq!(selected, NativeCompilationSelection::Generic);
                } else if resolves {
                    assert!(
                        matches!(selected, NativeCompilationSelection::Inline { operation, .. }
                        if operation == spec.operation)
                    );
                } else {
                    assert!(
                        matches!(selected, NativeCompilationSelection::NamedInvocation { arguments_from: 1, lookup, protocol: NativeNamedInvocationProtocol::Direct }
                        if lookup.slot == "::tcl::info::commands")
                    );
                }
            }
        }
    }

    #[test]
    fn existence_compiler_requires_original_private_implementation_on_modern_c() {
        let spec = NativeCompilationSpec {
            grammar: NativeCompilationGrammar::InfoExists,
            operation: SemanticOperationId::Intrinsic(crate::IntrinsicId::InfoExists),
            body: NativeBodyCompilation::Inherit,
        };
        for version in TclVersion::ALL {
            let dialect = InvocationDialect::for_version(version);
            let lookup = spec.implementation_lookup(dialect);
            if version == TclVersion::V8_4 {
                assert!(lookup.is_none());
            } else {
                let lookup = lookup.unwrap();
                assert_eq!(lookup.ensemble, "::info");
                assert_eq!(lookup.member, "exists");
                assert_eq!(lookup.slot, "::tcl::info::exists");
                assert_eq!(lookup.command, "info");
                assert_eq!(lookup.prepended, &["exists"]);
                let registry = crate::CommandRegistry::build_default();
                assert!(
                    registry
                        .native_compiler_implementation_slots(dialect)
                        .contains(&lookup)
                );
                assert!(
                    registry
                        .effective_semantics_for_dialect(dialect)
                        .binding_names()
                        .contains(lookup.slot)
                );
            }
        }
        let profile = tcl_dialect::DialectProfile::projected_from_point(
            "jim",
            &[],
            "Jim",
            tcl_dialect::model::DialectPoint::canonical(tcl_dialect::model::Release::JIM_0_84),
        );
        let jim = InvocationDialect::of_profile(&profile);
        assert!(spec.implementation_lookup(jim).is_none());
        assert!(
            crate::CommandRegistry::build_default()
                .native_compiler_implementation_slots(jim)
                .is_empty()
        );
    }

    #[test]
    fn existence_hook_accepts_substituted_names_without_a_local_frame() {
        let spec = NativeCompilationSpec {
            grammar: NativeCompilationGrammar::InfoExists,
            operation: SemanticOperationId::Intrinsic(crate::IntrinsicId::InfoExists),
            body: NativeBodyCompilation::Inherit,
        };
        for version in TclVersion::ALL {
            for frame in [
                NativeCompilationFrame::ScriptCode,
                NativeCompilationFrame::ProcedureCode,
            ] {
                for shape in [
                    NativeCompilationWordShape::Literal,
                    NativeCompilationWordShape::Substituted,
                ] {
                    assert_eq!(
                        spec.select(
                            InvocationWords::literals("info", &["a(k)"]),
                            &[shape],
                            Some(InvocationDialect::for_version(version)),
                            context(frame)
                        ),
                        if version == TclVersion::V8_4 {
                            NativeCompilationSelection::Generic
                        } else {
                            NativeCompilationSelection::Inline {
                                operation: spec.operation,
                                guard: NativeCompilationGuard::BeforeArguments,
                            }
                        }
                    );
                }
                assert_eq!(
                    spec.select(
                        InvocationWords::literals("info", &[]),
                        &[],
                        Some(InvocationDialect::for_version(version)),
                        context(frame)
                    ),
                    NativeCompilationSelection::Generic
                );
            }
        }
    }

    #[test]
    fn selected_opcode_guard_precedes_substituted_arguments() {
        let spec = NativeCompilationSpec {
            grammar: NativeCompilationGrammar::VariableLoadStore,
            operation: SemanticOperationId::StructuredLowering(LoweringHookId::Set),
            body: NativeBodyCompilation::Inherit,
        };
        assert!(spec.requires_original_word_preparation());
        let arguments = [
            crate::InvocationWord::Literal("x"),
            crate::InvocationWord::Dynamic,
        ];
        let words = InvocationWords::structured(crate::InvocationWord::Literal("set"), &arguments);
        let shapes = [
            NativeCompilationWordShape::Literal,
            NativeCompilationWordShape::Substituted,
        ];
        for version in TclVersion::ALL {
            assert_eq!(
                spec.select(
                    words,
                    &shapes,
                    Some(InvocationDialect::for_version(version)),
                    context(NativeCompilationFrame::ScriptCode)
                ),
                NativeCompilationSelection::Inline {
                    operation: spec.operation,
                    guard: if version == TclVersion::V8_4 {
                        NativeCompilationGuard::ChunkEntry
                    } else {
                        NativeCompilationGuard::BeforeArguments
                    },
                }
            );
        }
    }

    #[test]
    fn physical_caller_frame_does_not_invent_a_compiler_local_table() {
        let spec = NativeCompilationSpec {
            grammar: NativeCompilationGrammar::Catch,
            operation: SemanticOperationId::StructuredLowering(LoweringHookId::Catch),
            body: NativeBodyCompilation::Inherit,
        };
        let words = InvocationWords::literals("catch", &["set x 1", "result"]);
        let shapes = [
            NativeCompilationWordShape::BracedLiteral,
            NativeCompilationWordShape::Literal,
        ];
        let dialect = Some(InvocationDialect::for_version(TclVersion::V8_6));
        assert_eq!(
            spec.select(
                words,
                &shapes,
                dialect,
                context(NativeCompilationFrame::ScriptCode)
            ),
            NativeCompilationSelection::Generic
        );
        let selection = spec.select(
            words,
            &shapes,
            dialect,
            context(NativeCompilationFrame::ProcedureCode),
        );
        assert!(matches!(
            selection,
            NativeCompilationSelection::Inline { .. }
        ));
        assert_eq!(
            spec.body_context_for_operand(
                dialect,
                context(NativeCompilationFrame::ProcedureCode),
                selection,
                NativeCompilationWordShape::BracedLiteral
            ),
            context(NativeCompilationFrame::ProcedureCode).with_inline_exception_range()
        );
        assert_eq!(
            spec.body_context_for_operand(
                dialect,
                context(NativeCompilationFrame::ProcedureCode),
                NativeCompilationSelection::Generic,
                NativeCompilationWordShape::BracedLiteral
            ),
            context(NativeCompilationFrame::ScriptCode)
        );
    }

    #[test]
    fn selected_interpreter_controls_independent_body_boundaries() {
        let spec = NativeCompilationSpec {
            grammar: NativeCompilationGrammar::NoHook,
            operation: SemanticOperationId::Invoke,
            body: NativeBodyCompilation::Uplevel,
        };
        let enclosing = context(NativeCompilationFrame::ProcedureCode);
        assert_eq!(
            spec.body_context(
                Some(InvocationDialect::for_version(TclVersion::V8_4)),
                enclosing
            )
            .mode,
            NativeCompilationMode::Direct
        );
        assert_eq!(
            spec.body_context(
                Some(InvocationDialect::for_version(TclVersion::V8_5)),
                enclosing
            ),
            context(NativeCompilationFrame::ScriptCode)
        );
        let profile = crate::model::ingress::resolve_environment("jim").unit_profile();
        assert_eq!(
            spec.body_context(Some(InvocationDialect::of_profile(profile)), enclosing)
                .mode,
            NativeCompilationMode::Direct
        );
        assert_eq!(
            spec.body_context(None, enclosing),
            NativeCompilationContext::default()
        );
    }
    #[test]
    fn fixed_math_compiler_visits_required_children_before_argument_count_errors() {
        let profile = crate::model::ingress::resolve_environment("tcl8.4").unit_profile();
        let context = tcl_syntax::expr::parser::ExprParseContext::for_profile(profile);
        let operand = NativeCompiledExpressionOperand {
            argument: 0,
            error_context: NativeCompiledExpressionErrorContext::None,
        };
        for (text, script_count, message) in [
            (
                "future_function([set x extra bad])",
                0,
                "unknown math function \"future_function\"",
            ),
            (
                "abs([set x extra bad],2)",
                1,
                "too many arguments for math function",
            ),
            (
                "pow([set x extra bad])",
                1,
                "too few arguments for math function",
            ),
            (
                "abs(1,[set x extra bad])",
                0,
                "too many arguments for math function",
            ),
            ("abs(unknown(1),2)", 0, "unknown math function \"unknown\""),
        ] {
            let steps = operand.compiler_steps(text, &context, |name| match name {
                "abs" => NativeMathFunctionResolution::Known { arity: 1 },
                "pow" => NativeMathFunctionResolution::Known { arity: 2 },
                _ => NativeMathFunctionResolution::Absent,
            });
            assert_eq!(
                steps
                    .iter()
                    .filter(|step| matches!(step, NativeExpressionCompilerStep::Script(_)))
                    .count(),
                script_count,
                "{text}"
            );
            assert!(
                matches!(steps.last(), Some(NativeExpressionCompilerStep::Failure(failure)) if failure.message.as_deref()==Some(message)),
                "{text}: {steps:?}"
            );
        }
        assert_eq!(
            operand.compiler_steps("abs(1)", &context, |_| {
                NativeMathFunctionResolution::Unknown
            }),
            vec![NativeExpressionCompilerStep::Unknown]
        );
    }
}

/// Analysis topology of possible caller-frame body entries. It does not prove
/// that the native compiler selected an opcode or entered any body.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum PossibleBodyTopology {
    /// Reached bodies execute in sequence.
    Sequence(Vec<usize>),
    /// The selected capture consumes ordinary script completions. Native
    /// process termination remains outside the capture boundary.
    Captured(crate::catch_invocation::CatchInvocation),
    /// At most one body is selected.
    Alternatives(Vec<usize>),
    /// Native conditional tests retain their operand and associated body.
    /// A missing test operand is the final unconditional alternative.
    Conditional(Vec<(Option<usize>, usize)>),
    /// Case bodies retain their true list element/argument locations.
    CaseAlternatives(Vec<crate::body_execution::BodyOperand>),
    /// Initial bodies execute once; repeated bodies follow a possible loop edge.
    Loop {
        /// Once-only entry operands.
        initial: Vec<usize>,
        /// Repeated body and update operands.
        repeated: Vec<usize>,
        /// Operands reached after a continue completion.
        continued: Vec<usize>,
    },
}

impl PossibleHandlerBodyFlow {
    /// Project only the authored body topology; unsupported execution protocols
    /// remain absent rather than inheriting a sequence from role order.
    ///
    /// ```
    /// use tcl_registry::native_compilation::{PossibleBodyTopology, PossibleHandlerBodyFlow};
    /// use tcl_registry::{VariableAliasFrame, script_body_flow::ScriptBodyFlow};
    /// let selected = PossibleHandlerBodyFlow {
    ///     flow: ScriptBodyFlow::Conditional(vec![(Some(0), 1), (None, 3)]),
    ///     frame: VariableAliasFrame::Procedure,
    /// };
    /// assert_eq!(selected.topology(), Some(PossibleBodyTopology::Conditional(vec![(Some(0), 1), (None, 3)])));
    /// // This topology supplies possible entries; it grants no compiler opcode.
    /// ```
    #[must_use]
    pub fn topology(&self) -> Option<PossibleBodyTopology> {
        use crate::script_body_flow::ScriptBodyFlow;
        match &self.flow {
            ScriptBodyFlow::Capture(crate::catch_invocation::CatchInvocationSelection::Valid(
                selected,
            )) if selected.ignored_codes == 0 => Some(PossibleBodyTopology::Captured(*selected)),
            ScriptBodyFlow::Sequence(indices) => {
                Some(PossibleBodyTopology::Sequence(indices.clone()))
            }
            ScriptBodyFlow::ConcatenatedScript { argument_offset } => {
                // The source inventory attaches the complete evaluated concat
                // to each contributing operand. Select its first operand once;
                // the topology must not evaluate the constituent scripts again.
                Some(PossibleBodyTopology::Sequence(vec![*argument_offset]))
            }
            ScriptBodyFlow::Alternatives(indices) => {
                Some(PossibleBodyTopology::Alternatives(indices.clone()))
            }
            ScriptBodyFlow::CaseBodies(selection) => Some(PossibleBodyTopology::CaseAlternatives(
                selection.bodies.clone(),
            )),
            ScriptBodyFlow::Conditional(branches) => {
                Some(PossibleBodyTopology::Conditional(branches.clone()))
            }
            ScriptBodyFlow::Loop {
                initial,
                repeated,
                continued,
                ..
            } => Some(PossibleBodyTopology::Loop {
                initial: initial.clone(),
                repeated: repeated.clone(),
                continued: continued.clone(),
            }),
            _ => None,
        }
    }
}

#[cfg(test)]
mod possible_capture_tests {
    use super::{PossibleBodyTopology, PossibleHandlerBodyFlow};
    use crate::catch_invocation::{CatchInvocation, CatchInvocationSelection};
    use crate::{VariableAliasFrame, script_body_flow::ScriptBodyFlow};

    #[test]
    fn possible_case_bodies_keep_unknown_subjects_without_accepting_bad_lists() {
        use crate::InvocationWord::{Dynamic, Literal};
        let registry = crate::model::ingress::static_context_for("f5-irules").commands();
        let dialect = crate::InvocationDialect::of_profile(
            crate::model::ingress::resolve_environment("f5-irules").unit_profile(),
        );
        for (cases, possible) in [
            ("loud {set debug 1} default {set debug 0}", true),
            ("loud", false),
            ("{", false),
        ] {
            let arguments = [Dynamic, Literal(cases)];
            let words = crate::InvocationWords::structured(Literal("switch"), &arguments)
                .with_dialect(dialect);
            let resolution = registry.resolve_structured_invocation(words, None);
            let facts = resolution.resolved().unwrap().facts();
            assert_eq!(
                facts
                    .possible_handler_body_flow(
                        registry,
                        words.arguments(),
                        VariableAliasFrame::Procedure
                    )
                    .is_some(),
                possible,
                "{cases}"
            );
        }
    }

    #[test]
    fn possible_capture_retains_selected_filter_without_licensing_filtered_propagation() {
        let selected = CatchInvocation::CAPTURE_TCL_PHASE;
        let mut flow = PossibleHandlerBodyFlow {
            flow: ScriptBodyFlow::Capture(CatchInvocationSelection::Valid(selected)),
            frame: VariableAliasFrame::Procedure,
        };
        assert_eq!(
            flow.topology(),
            Some(PossibleBodyTopology::Captured(selected))
        );
        flow.flow = ScriptBodyFlow::Capture(CatchInvocationSelection::Valid(CatchInvocation {
            ignored_codes: 1 << 2,
            ..selected
        }));
        assert_eq!(flow.topology(), None);
    }
}

#[cfg(test)]
mod tailcall_tests {
    use super::*;

    #[test]
    fn native_tailcall_selection_and_namespace_stack_follow_the_actual_release() {
        let spec = NativeCompilationSpec {
            grammar: NativeCompilationGrammar::Tailcall,
            operation: SemanticOperationId::Invoke,
            body: NativeBodyCompilation::Inherit,
        };
        let context = NativeCompilationContext {
            mode: NativeCompilationMode::BytecodeObject,
            frame: NativeCompilationFrame::ProcedureCode,
            ..Default::default()
        };
        for version in TclVersion::ALL {
            let dialect = InvocationDialect::for_version(version);
            let arguments = ["target"];
            let words = InvocationWords::literals("tailcall", &arguments);
            let selected = spec.select(
                words,
                &[NativeCompilationWordShape::Literal],
                Some(dialect),
                context,
            );
            assert_eq!(
                matches!(selected, NativeCompilationSelection::Inline { .. }),
                version >= TclVersion::V8_6
            );
            let script = spec.select(
                words,
                &[NativeCompilationWordShape::Literal],
                Some(dialect),
                NativeCompilationContext {
                    frame: NativeCompilationFrame::ScriptCode,
                    ..context
                },
            );
            assert_eq!(script, NativeCompilationSelection::Generic);
            let expanded = spec.select(
                words,
                &[NativeCompilationWordShape::Expanded],
                Some(dialect),
                context,
            );
            assert_eq!(
                matches!(expanded, NativeCompilationSelection::Inline { .. }),
                version == TclVersion::V9_1
            );
            let empty = spec.select(
                InvocationWords::literals("tailcall", &[]),
                &[],
                Some(dialect),
                context,
            );
            assert_eq!(
                matches!(empty, NativeCompilationSelection::Inline { .. }),
                version == TclVersion::V9_1
            );
            let stack = NativeTailcallStack::for_invocation(dialect, 1, false);
            assert_eq!(
                stack,
                match version {
                    TclVersion::V8_4 | TclVersion::V8_5 => None,
                    TclVersion::V8_6 | TclVersion::V9_0 =>
                        Some(NativeTailcallStack::NamespaceAfterArguments),
                    TclVersion::V9_1 => Some(NativeTailcallStack::NamespaceBeforeArguments),
                }
            );
        }
        assert_eq!(
            NativeTailcallStack::for_invocation(
                InvocationDialect::for_version(TclVersion::V9_1),
                0,
                false
            ),
            Some(NativeTailcallStack::NamespacePrefixedList)
        );
    }
    #[test]
    fn nested_named_invocation_retains_only_written_selectors() {
        static LOOKUP: NativeCompilerImplementationLookup = NativeCompilerImplementationLookup {
            ensemble: "::tcl::binary::encode",
            member: "hex",
            slot: "::tcl::binary::encode::hex",
            command: "binary",
            prepended: &["encode", "hex"],
        };
        for (head, args, from) in [
            ("binary", vec!["encode", "hex", "DATA"], 2),
            ("::tcl::binary::encode", vec!["hex", "DATA"], 1),
            ("::tcl::binary::encode::hex", vec!["DATA"], 0),
        ] {
            let shapes = vec![NativeCompilationWordShape::Literal; args.len()];
            assert!(matches!(
                named_ensemble_grammar(&LOOKUP, TclVersion::V8_6, crate::Arity::exact(1),
                    InvocationWords::literals(head, &args), &shapes, TclVersion::V8_6),
                NativeCompilationSelection::NamedInvocation { arguments_from, .. } if arguments_from == from
            ));
            assert_eq!(
                named_ensemble_grammar(
                    &LOOKUP,
                    TclVersion::V8_6,
                    crate::Arity::exact(1),
                    InvocationWords::literals(head, &args),
                    &shapes,
                    TclVersion::V8_5
                ),
                NativeCompilationSelection::Generic
            );
            let missing = &args[..args.len() - 1];
            assert_eq!(
                named_ensemble_grammar(
                    &LOOKUP,
                    TclVersion::V8_6,
                    crate::Arity::exact(1),
                    InvocationWords::literals(head, missing),
                    &shapes[..shapes.len() - 1],
                    TclVersion::V8_6
                ),
                NativeCompilationSelection::Generic
            );
        }
    }
}
