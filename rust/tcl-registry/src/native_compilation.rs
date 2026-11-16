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

//! Native script compilation boundaries and operation selection before argv.
//!
//! These descriptors supplement runtime dispatch; a terminal runtime command
//! target cannot prove that an interpreter previously compiled an operation.

use crate::{InvocationDialect, InvocationWords, SemanticOperationId};
use tcl_dialect::TclVersion;
use tcl_dialect::model::Family;

mod compiled_bodies;
mod expression_compilation;
mod grammar_selection;
mod implementation_lookup;
mod introspection_grammars;
mod name_effects;
mod original_word_selection;
mod string_grammars;
mod successful_handler;
mod variable_and_list_grammars;
mod word_selection;

pub use compiled_bodies::{
    NativeCompilationStep, NativeCompilationSteps, NativeCompiledBodies, NativeCompiledBodyContext,
    NativeCompiledBodyErrorContext, NativeCompiledBodyOperand,
};
pub(crate) use expression_compilation::expression_compiler_visits;
pub use expression_compilation::{
    NativeCompiledExpressionErrorContext, NativeCompiledExpressionOperand,
    NativeExpressionCompilation, NativeExpressionCompilerStep, NativeMathFunctionResolution,
};
pub(crate) use grammar_selection::clock_reading_value;
pub use implementation_lookup::NativeCompilerImplementationLookup;
pub use name_effects::NativeOriginalCompilerNameEffects;
#[cfg(test)]
use string_grammars::named_ensemble_grammar;
use string_grammars::regexp_compiler_pattern_index;
pub use string_grammars::{NamedCompilerOperandView, named_arity_at, splits_into_text_tokens};
pub use successful_handler::{
    NormalHandlerImplementationLookup, NormalValueLeafProvider, PossibleBodyTopology,
    PossibleHandlerBodyFlow, SuccessfulHandlerEffects, SuccessfulHandlerSpec,
    UserProcedureInvocation, VariableOperandBindingPhase, VariableOutputLookup,
};
pub use variable_and_list_grammars::original_upvar_level_width;

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

impl NativeCompilationGuard {
    /// Actual native C command-entry ordering. A profile-only or foreign engine
    /// supplies no guard, and the guard itself authenticates no captured opcode.
    #[must_use]
    pub fn for_native_dialect(dialect: crate::InvocationDialect) -> Option<Self> {
        let tcl_syntax::naming::NativeNameProtocol::C(version) = dialect.native_name_protocol()?
        else {
            return None;
        };
        Some(Self::for_c_version(version))
    }

    /// Original C release's compiler currency boundary, separate from command
    /// identity, metadata selection and any argument execution.
    #[must_use]
    pub const fn for_c_version(version: TclVersion) -> Self {
        match version {
            TclVersion::V8_4 => Self::ChunkEntry,
            _ => Self::BeforeArguments,
        }
    }

    /// Whether this command boundary consumes current compiler/resolver epochs.
    /// Chunk-entry selection remains captured while its operands run; it needs
    /// no newly available epoch from mutations reached during those operands.
    #[must_use]
    pub const fn requires_current_epochs(self) -> bool {
        matches!(self, Self::BeforeArguments)
    }

    /// A retained command must reacquire current source dispatch before its
    /// operands when the actual compiler or namespace resolver epoch changed.
    #[must_use]
    pub const fn revalidate_source(self, captured: (u64, u64), current: (u64, u64)) -> bool {
        self.requires_current_epochs() && (captured.0 != current.0 || captured.1 != current.1)
    }
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

/// Syntax validation boundary of an actual selected Direct script entry.
/// This supplies no command lookup, frame, naming or successful entry proof.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum NativeDirectScriptParseEntry {
    /// Supported C Tcl entries validate each command before its effects.
    CommandAtTime,
    /// Jim 0.84 validates the complete script before its first command.
    WholeScript,
}

impl NativeDirectScriptParseEntry {
    fn for_point(point: tcl_dialect::model::DialectPoint) -> Option<Self> {
        use tcl_dialect::model::Release;
        match point.release() {
            Release::TCL_8_4
            | Release::TCL_8_5
            | Release::TCL_8_6
            | Release::TCL_9_0
            | Release::TCL_9_1 => Some(Self::CommandAtTime),
            Release::JIM_0_84 => Some(Self::WholeScript),
            _ => None,
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
    /// Select parse ordering from this actual Direct entry's physical engine.
    /// Assistance, handler simulation and lexical grammar cannot supply it.
    /// Unsupported or missing physical points remain unavailable; bytecode
    /// entry keeps its independent compiler traversal protocol.
    #[must_use]
    pub fn direct_script_parse_entry(
        self,
        entry: &tcl_runtime_api::NativeCompilationEntry,
    ) -> Option<NativeDirectScriptParseEntry> {
        if self.mode != NativeCompilationMode::Direct {
            return None;
        }
        NativeDirectScriptParseEntry::for_point(entry.execution_point?)
    }

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

/// Registered native clock reading compiler, independent of command spelling.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum NativeClockReading {
    /// Clicks and its microseconds / milliseconds options.
    Clicks,
    /// Microsecond wall clock.
    Microseconds,
    /// Millisecond wall clock.
    Milliseconds,
    /// Second wall clock.
    Seconds,
}

/// Native compiler-hook grammar, separate from a front-end lowering hook.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum NativeCompilationGrammar {
    /// No native compiler hook is licensed by this descriptor.
    NoHook,
    /// Native hook availability or syntax is not yet an authored closed contract.
    Unresolved,
    /// `TclCompileThrowCmd`: exactly two arguments from 8.6, compiled whether
    /// the type is a literal or checked at run time.
    Throw,
    /// `TclCompileConstCmd`: exactly two arguments from 9.0, a scalar name
    /// only; a literal array-element name declines to an invocation.
    Constant,
    /// Independently authored ordered prerequisites of a named-invocation or
    /// absent-hook compiler. This does not wrap body or opcode grammars.
    WithImplementationPath {
        /// Actual terminal compiler grammar; this carries no handler semantics.
        compiler: &'static NativeCompilationSpec,
        /// Original ensemble mappings and worker identities, outermost first.
        lookups: &'static [NativeCompilerImplementationLookup],
        /// First native release installing this exact path.
        implementation_from: TclVersion,
        /// Earlier C releases use the same explicitly authored absent-hook
        /// member in their monolithic handler. This cannot license an earlier
        /// private worker, opcode, or a non-NULL terminal compiler.
        monolithic_no_hook_before: bool,
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
    /// C8.6+ dynamic `%s`/`%%` concatenation and exact C9.0 `%x` generic decline.
    /// Constant format evaluation and other conversion paths remain unresolved.
    FormatStringConcat,
    /// Native clock reading operation.
    ClockReading(NativeClockReading),
    /// No hook before this C release; an actual hook with unmodelled grammar thereafter.
    HookFrom(TclVersion),
    /// C8.5's monolithic namespace compiler only attempts original upvar syntax.
    /// Other namespace operations decline before preparing any operands.
    NamespaceLegacy,
    /// C8.6+ original private namespace-upvar worker compiler.
    NamespaceUpvarBindings,
    /// C8.6+ namespace command-query hook; variable queries decline its opcode.
    NamespaceWhich,
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
        /// A later release's own count, when its compiler accepts more.
        later_arity: Option<(TclVersion, crate::Arity)>,
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
    /// C8.6+ namespace string arithmetic over one original evaluated operand.
    NamespaceString(crate::native_namespace_string_compilation::NativeNamespaceStringOperation),
    /// Native string equality; unsupported options use a private invocation from 8.6.
    StringEqual(crate::native_scalar_compilation::NativeScalarScope),
    /// C8.6+ exact non-strict ASCII class test; other forms remain unclosed.
    StringIsAscii(crate::native_scalar_compilation::NativeScalarScope),
    /// C8.4+ string length accepts one evaluated operand; refused modern
    /// public forms retain the selected private worker invocation.
    StringLength(crate::native_scalar_compilation::NativeScalarScope),
    /// A fixed-operand String member instruction (`strcmp`, `strindex`,
    /// `strfind`, `strrfind` and the case conversions), with the release and
    /// refused-count fallbacks of its actual compiler.
    StringOperand {
        /// Public member or actual private worker operand layout.
        scope: crate::native_scalar_compilation::NativeScalarScope,
        /// Selected instruction, independent of the current command spelling.
        operation: crate::native_string_operand_compilation::NativeStringOperandOperation,
    },
    /// Original C string-match hook and its actual registration operand scope.
    StringMatch(crate::native_string_compilation::NativeStringMatchScope),
    /// C9 `Basic2Arg` fallback after a private compile-known non-pair map probe.
    StringMap(crate::native_scalar_compilation::NativeScalarScope),
    /// Original C9 range compiler with represented compile-known indices.
    StringRange(crate::native_scalar_compilation::NativeScalarScope),
    /// `TclCompileStringReplaceCmd` over its original words.
    StringReplace(crate::native_scalar_compilation::NativeScalarScope),
    /// `TclCompileStringInsertCmd` over its original words.
    StringInsert(crate::native_scalar_compilation::NativeScalarScope),
    /// `TclCompileLreplaceCmd` over its original words.
    ListReplacement,
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
    /// C8.6+ original zero-operand coroutine-name instruction.
    InfoCoroutine,
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
    /// C8.6+ regsub compiler: only `-all ?--? simpleRE string simpleReplacement`
    /// becomes a string map; every other form invokes the command.
    Regsub,
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

impl NativeCompilationSpec {
    /// [`Self::requires_original_word_preparation`] for one command whose
    /// operands include an expanded word when `expanded` is set.
    ///
    /// `TclCompileListCmd` sees a literal `{*}{a b}` only after the parser
    /// has flattened it, a projection the List recipe does not take yet; such
    /// a `list` keeps its expanded emission.
    #[must_use]
    pub const fn requires_original_word_preparation_of(self, expanded: bool) -> bool {
        self.requires_original_word_preparation()
            && !(expanded && matches!(self.grammar, NativeCompilationGrammar::ArgumentList))
    }

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
            | NativeCompilationGrammar::ListRange
            | NativeCompilationGrammar::ListAssignment
            | NativeCompilationGrammar::LiteralUnset
            | NativeCompilationGrammar::Upvar
            | NativeCompilationGrammar::InfoExists
            | NativeCompilationGrammar::Array { .. }
            | NativeCompilationGrammar::InfoLevel
            | NativeCompilationGrammar::ArgumentList
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
            | NativeCompilationGrammar::NamespaceString(_)
            | NativeCompilationGrammar::StringTrim { .. }
            | NativeCompilationGrammar::StringMatch(_)
            | NativeCompilationGrammar::StringEqual(_)
            | NativeCompilationGrammar::StringLength(_)
            | NativeCompilationGrammar::StringIsAscii(_)
            | NativeCompilationGrammar::StringMap(_)
            | NativeCompilationGrammar::StringRange(_)
            | NativeCompilationGrammar::StringReplace(_)
            | NativeCompilationGrammar::StringInsert(_)
            | NativeCompilationGrammar::ListReplacement
            | NativeCompilationGrammar::StringOperand { .. }
            | NativeCompilationGrammar::ListLength
            | NativeCompilationGrammar::Tailcall
            | NativeCompilationGrammar::CoroutineYield
            | NativeCompilationGrammar::InfoCoroutine
            | NativeCompilationGrammar::CoroutineRelay
            | NativeCompilationGrammar::FormatStringConcat
            | NativeCompilationGrammar::SubstitutionTemplate
            | NativeCompilationGrammar::Error
            | NativeCompilationGrammar::Throw => true,
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
            NativeCompilationGrammar::InfoCommands => Some(Kind::InfoCommands),
            NativeCompilationGrammar::NamespaceCurrent => Some(Kind::NamespaceCurrent),
            NativeCompilationGrammar::NamespaceOrigin => Some(Kind::NamespaceOrigin),
            NativeCompilationGrammar::NamespaceWhich => Some(Kind::NamespaceWhich),
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

    /// A list-construction intrinsic cannot authorise another compiler's stack.
    /// Generic Invoke descriptors retain their separately selected grammar.
    fn operation_matches_grammar(self) -> bool {
        if let NativeCompilationGrammar::WithImplementationPath { compiler, .. } = self.grammar {
            return compiler.operation_matches_grammar() && self.operation == compiler.operation;
        }
        if self.grammar == NativeCompilationGrammar::FormatStringConcat
            || matches!(self.grammar, NativeCompilationGrammar::ClockReading(_))
        {
            return self.operation == SemanticOperationId::Invoke;
        }
        if matches!(
            self.grammar,
            NativeCompilationGrammar::StringMap(_)
                | NativeCompilationGrammar::StringRange(_)
                | NativeCompilationGrammar::StringReplace(_)
                | NativeCompilationGrammar::StringInsert(_)
                | NativeCompilationGrammar::ListReplacement
        ) {
            return self.operation == SemanticOperationId::Invoke
                && self.body == NativeBodyCompilation::Inherit;
        }
        if matches!(self.grammar, NativeCompilationGrammar::StringIsAscii(_)) {
            return self.operation == SemanticOperationId::Intrinsic(crate::IntrinsicId::StringIs);
        }
        self.operation != SemanticOperationId::Intrinsic(crate::IntrinsicId::ListConstruct)
            || matches!(
                self.grammar,
                NativeCompilationGrammar::ArgumentList
                    | NativeCompilationGrammar::ArgumentListFrom(_)
            )
    }
}

fn static_value_shape(shape: NativeCompilationWordShape) -> bool {
    literal_shape(shape) || shape == NativeCompilationWordShape::BackslashLiteral
}

#[cfg(test)]
mod append_compilation_tests;

fn literal_shape(shape: NativeCompilationWordShape) -> bool {
    matches!(
        shape,
        NativeCompilationWordShape::Literal
            | NativeCompilationWordShape::QuotedLiteral
            | NativeCompilationWordShape::BracedLiteral
    )
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod possible_capture_tests;

#[cfg(test)]
mod tailcall_tests;

#[cfg(test)]
mod native_command_currency_tests;

#[cfg(test)]
#[path = "native_compilation/namespace_which_tests.rs"]
mod namespace_which_tests;

#[cfg(test)]
mod named_operand_layout_tests;

#[cfg(test)]
mod simple_word_split_tests;

#[cfg(test)]
mod clock_reading_compiler_tests;
