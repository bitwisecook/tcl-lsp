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

//! A small target-neutral executable semantic IR.
//!
//! This is intentionally not a backend IR and does not enable code
//! generation or specialisation.  It records the Tcl-visible sequencing that
//! precedes a generic invocation: words are evaluated left-to-right, argument
//! expansion happens before command resolution, and every potentially failing
//! operation yields one [`CompletionId`] representing its code, result, and
//! return-options triple.  Future common analyses can add state/effect facts
//! to this IR without teaching target emitters about Tcl command names.

use std::collections::BTreeMap;
#[cfg(test)]
use std::collections::BTreeSet;

use tcl_core_types::Code as CompletionCode;
use tcl_registry::hooks::LoweringHookId;
use tcl_registry::model::semantic::SemanticContext;
use tcl_registry::{CommandRegistry, SemanticOperationId};

use crate::expr_ast::ExprNode;
use crate::ir::Script;
use crate::ir::Statement;
use crate::ir::SwitchMode;
use crate::registry_invocation::InvocationMetadataContext;
pub use crate::registry_invocation::{
    OwnedInvocationResolutionUnresolved, RegistryInvocationResolution as InvocationResolution,
};
use tcl_source::words::NodeId;
use tcl_source::words::SourceSite;
use tcl_source::words::WordExpr;

mod builder;
mod dominance;
mod footprints;
mod region_descriptors;
mod validation;

#[cfg(test)]
use builder::original_list_iterator_names;
use builder::{ControlContext, FunctionBuilder, ScriptTail};
#[cfg(test)]
use footprints::lowered_operation_footprint;
#[cfg(test)]
pub(crate) use region_descriptors::try_handler_code_in;
pub use validation::ExecutableIrValidationError;

/// Identity of one independently executable function body.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ExecutableFunctionId(usize);

impl ExecutableFunctionId {
    /// Construct a function identity owned by the caller's compilation unit.
    #[must_use]
    pub const fn new(index: usize) -> Self {
        Self(index)
    }

    /// Return the caller-owned function index.
    #[must_use]
    pub const fn index(self) -> usize {
        self.0
    }
}

/// Identity of a basic block owned by an [`ExecutableFunctionId`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ExecutableBlockId {
    function: ExecutableFunctionId,
    index: usize,
}

impl ExecutableBlockId {
    /// Construct a block identity for `function`.
    #[must_use]
    pub const fn new(function: ExecutableFunctionId, index: usize) -> Self {
        Self { function, index }
    }

    /// Return the function that owns this block.
    #[must_use]
    pub const fn function(self) -> ExecutableFunctionId {
        self.function
    }

    /// Return the deterministic block position within its function.
    #[must_use]
    pub const fn index(self) -> usize {
        self.index
    }
}

/// Identity of an evaluated Tcl value owned by an [`ExecutableFunctionId`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ExecutableValueId {
    function: ExecutableFunctionId,
    index: usize,
}

impl ExecutableValueId {
    /// Construct a value identity for `function`.
    #[must_use]
    pub const fn new(function: ExecutableFunctionId, index: usize) -> Self {
        Self { function, index }
    }

    /// Return the function that owns this value.
    #[must_use]
    pub const fn function(self) -> ExecutableFunctionId {
        self.function
    }

    /// Return the caller-assigned value index.
    #[must_use]
    pub const fn index(self) -> usize {
        self.index
    }
}

/// Identity of one constructed argv vector owned by an [`ExecutableFunctionId`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ExecutableArgvId {
    function: ExecutableFunctionId,
    index: usize,
}

impl ExecutableArgvId {
    /// Construct an argv identity for `function`.
    #[must_use]
    pub const fn new(function: ExecutableFunctionId, index: usize) -> Self {
        Self { function, index }
    }

    /// Return the function that owns this argv vector.
    #[must_use]
    pub const fn function(self) -> ExecutableFunctionId {
        self.function
    }

    /// Return the caller-assigned argv index.
    #[must_use]
    pub const fn index(self) -> usize {
        self.index
    }
}

/// Identity of a Tcl completion triple owned by an [`ExecutableFunctionId`].
///
/// A completion is deliberately one first-class value: it contains the Tcl
/// completion code, result, and return-options dictionary together.  This
/// initial IR never permits a transform to accidentally forward the code
/// while dropping the result or options payload.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CompletionId {
    function: ExecutableFunctionId,
    index: usize,
}

impl CompletionId {
    /// Construct a completion identity for `function`.
    #[must_use]
    pub const fn new(function: ExecutableFunctionId, index: usize) -> Self {
        Self { function, index }
    }

    /// Return the function that owns this completion triple.
    #[must_use]
    pub const fn function(self) -> ExecutableFunctionId {
        self.function
    }

    /// Return the caller-assigned completion index.
    #[must_use]
    pub const fn index(self) -> usize {
        self.index
    }
}

/// An argv entry after word evaluation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ArgvEntry {
    /// One ordinary Tcl word contributes one argv element.
    Value(ExecutableValueId),
    /// A `{*}` word contributes the elements of its evaluated Tcl list.
    Expanded(ExecutableValueId),
}

impl ArgvEntry {
    fn value(self) -> ExecutableValueId {
        match self {
            Self::Value(value) | Self::Expanded(value) => value,
        }
    }
}

/// A generic Tcl invocation after all argv words have been evaluated.
///
/// `original_words` is deliberately retained even though `argv` has already
/// been constructed. They are provenance for diagnostics and source
/// reconstruction only. A later guarded fast path must pass the already-built
/// `argv` to its generic slow path; it must never use these words as executable
/// fallback input or re-run their substitutions.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GenericInvoke {
    /// The first-class completion produced by the invocation.
    pub completion: CompletionId,
    /// Fully evaluated argv vector supplied to runtime resolution.
    pub argv: ExecutableArgvId,
    /// Registry facts, or the typed reason they could not be selected. The
    /// runtime invocation remains generic in both cases.
    pub resolution: InvocationResolution,
    /// Original source words, including the command head and `{*}` markers.
    pub original_words: Vec<WordExpr>,
    /// Point-specific dispatch proof after original argument substitution.
    /// Explicit unknown/absence is never replaced by written-head metadata.
    pub source_binding: Option<crate::command_binding::SourceInvocationBinding>,
    /// Post-binding words and origins for semantic argument-role projection.
    /// These are facts, not a second argv evaluation sequence.
    pub effective_words: Option<crate::registry_invocation::EffectiveCommandWords>,
    /// Stable source-semantic node that originated this command.
    pub node: NodeId,
    /// Full command source site and provenance.
    pub source: SourceSite,
}

impl GenericInvoke {
    /// Whether registry argument positions map exactly to runtime written argv.
    /// This is an argument-correspondence condition, not a dispatch proof.
    /// A backend can specialise an alias prefix only after explicitly composing
    /// that prefix into its executable argv; ordinary runtime dispatch remains
    /// valid with the original argv regardless of this answer.
    #[must_use]
    pub fn registry_specialisation_arguments_exact(&self) -> bool {
        if self.source_binding.as_ref().is_some_and(|binding| {
            binding
                .proved_execution_target()
                .is_none_or(|target| !target.registry_backed || !target.prepended.is_empty())
        }) {
            return false;
        }
        self.effective_words.as_ref().is_none_or(|words| {
            words.words.len() == self.original_words.len()
                && words.origins.len() == words.words.len()
                && words
                    .origins
                    .iter()
                    .enumerate()
                    .skip(1)
                    .all(|(index, origin)| {
                        *origin == crate::registry_invocation::InvocationWordOrigin::Written(index)
                    })
        })
    }
}

/// A source statement whose registry-selected structural lowering has already
/// happened.
///
/// `descriptor` is the registry-owned semantic identity that authorised the
/// lowering.  It deliberately carries no command spelling, command binding,
/// or dispatch proof: the compatibility source IR does not retain those facts,
/// and manufacturing them here would be unsound in the presence of aliases,
/// namespaces, or `rename`.  `statement` is retained as an exact executable
/// payload for consumers still reading the compatibility IR.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoweredOperation {
    /// Completion produced by executing the operation.
    pub completion: CompletionId,
    /// Registry-owned target-neutral lowering descriptor.
    pub descriptor: LoweringHookId,
    /// Exact cell and completion footprint projected from the descriptor and
    /// the statement's retained operands.
    pub footprint: LoweredFootprint,
    /// Exact already-lowered source operation and operands.
    pub statement: Statement,
    /// Whole original command words retained by the actual Script statement.
    /// Missing source ownership cannot be reconstructed from the name field.
    pub original_tokens: Option<std::sync::Arc<crate::ir::CommandTokens>>,
    /// Stable source-semantic node that originated the operation.
    pub node: NodeId,
    /// Full source site and provenance.
    pub source: SourceSite,
}

impl LoweredOperation {
    /// Return the target-neutral operation identity selected by the retained
    /// registry descriptor.
    #[must_use]
    pub const fn semantic_operation(&self) -> SemanticOperationId {
        SemanticOperationId::StructuredLowering(self.descriptor)
    }
}

/// A bounded region whose internal control cannot yet be expressed exactly by
/// this executable IR.
///
/// The region remains one executable operation, so statements before and after
/// it keep their registry and completion facts.  A known structural descriptor
/// is retained when the source-IR shape proves one; `None` records that the
/// earlier lowering discarded even that identity.  Common passes must treat a
/// region as a conservative world barrier and must not inspect `statement` to
/// invent command-specific semantics.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OpaqueRegion {
    /// Completion produced by executing the whole region.
    pub completion: CompletionId,
    /// Registry-owned structural identity, when it survived lowering.
    pub descriptor: Option<LoweringHookId>,
    /// Exact structured source-IR payload for a later common CFG lowering.
    pub statement: Statement,
    /// Stable source-semantic node that originated the region.
    pub node: NodeId,
    /// Full source site and provenance.
    pub source: SourceSite,
}

impl OpaqueRegion {
    /// Return the retained target-neutral operation identity, when lowering
    /// preserved one. `None` is an explicit compatibility gap, not an
    /// invitation to infer a command from the region payload.
    #[must_use]
    pub const fn semantic_operation(&self) -> Option<SemanticOperationId> {
        match self.descriptor {
            Some(descriptor) => Some(SemanticOperationId::StructuredLowering(descriptor)),
            None => None,
        }
    }
}

/// One Tcl variable cell named by source-faithful lowering.
///
/// This is deliberately *not* a [`crate::place::Place`]: binding a name to a
/// place requires the scope declarations (`global`, `variable`, `upvar`,
/// instance variables) that a `ResolveContext` carries, and the executable
/// builder is handed a [`Script`] and a registry context only.  A cell
/// reference is therefore the exact retained name plus the one distinction the
/// name text itself proves — whether it is statically spellable at all — and a
/// consumer that does own a scope context binds it to a `Place` itself.
///
/// The base name is the world subject in both cases: `a(k)` and `$a` name the
/// same subject, so an element write is correctly seen by a whole-array read.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum CellReference {
    /// A cell whose name is statically known, resolved in the scope current at
    /// the site.  `element` records that only one element of an array was
    /// touched.
    Named {
        /// Base variable name, without any array index.
        name: String,
        /// Whether the reference named one array element.
        element: bool,
    },
    /// A cell whose identity is computed at run time (`set $name …`, a
    /// substituted array index).  Every access must be treated as touching an
    /// unbounded set of cells.
    Computed,
}

impl CellReference {
    /// Project a completed variable-name datum without interpreting it as source.
    /// Empty and sigil bytes remain name data; shared element partitioning keeps
    /// an array root distinct from its element. No scope or storage is selected.
    #[must_use]
    pub fn from_name_data(name: &str) -> Self {
        let (name, element) = tcl_syntax::naming::split_element_ref(name)
            .map_or((name, false), |(root, _)| (root, true));
        Self::Named {
            name: name.to_owned(),
            element,
        }
    }

    /// Project one retained variable-name word into the cell vocabulary.
    ///
    /// `braced` is the source token's brace-literal flag: a braced name word
    /// suppresses substitution, so `{a($k)}` names a literal element of `a`
    /// rather than a computed one.  Array-name splitting is the shared
    /// `tcl_syntax` rule, never a local re-parse.
    #[must_use]
    pub fn from_name(name: &str, braced: bool) -> Self {
        if name.is_empty() {
            return Self::Computed;
        }
        let (base, index) = tcl_syntax::naming::split_array_name_braced(name, braced);
        if base.is_empty() || base.contains('$') || base.contains('[') {
            return Self::Computed;
        }
        if !braced && index.is_some_and(|index| index.contains('$') || index.contains('[')) {
            // A substituted index still touches only this array, but the
            // element is unknown; the base subject already covers that.
            return Self::Named {
                name: base.to_owned(),
                element: true,
            };
        }
        Self::Named {
            name: base.to_owned(),
            element: index.is_some(),
        }
    }

    /// The statically known base name, when there is one.
    #[must_use]
    pub fn name(&self) -> Option<&str> {
        match self {
            Self::Named { name, .. } => Some(name),
            Self::Computed => None,
        }
    }
}

/// Which part of a completion triple a handler cell receives.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum CompletionPayload {
    /// The Tcl completion code.
    Code,
    /// The completion result value.
    Result,
    /// The return-options dictionary.
    Options,
}

/// One structured-control operand that produces a Tcl value.
///
/// Structured lowering keeps conditions as parsed expressions and list or
/// subject words as exact retained text, so these operands cannot travel
/// through [`ExecutableInstruction::EvaluateWord`], which validates a
/// [`WordExpr`] against its own source site.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExecutableExpr {
    /// A parsed Tcl expression retained by structured lowering: an `if`,
    /// `while`, or `for` condition.
    Condition {
        /// Parsed expression AST.
        expr: Box<ExprNode>,
        /// Absolute source offset of the expression text's first byte, when
        /// the text handed to the expression parser was a verbatim slice.
        base: Option<u32>,
    },
    /// An exact retained operand word: a `foreach` list, a `switch` subject.
    Operand {
        /// Exact retained operand text, with any braces already stripped.
        text: String,
        /// Whether the source word was a brace literal, which suppresses
        /// substitution.
        braced: bool,
    },
    /// The `-errorcode` prefix test of a `try … trap` handler against the
    /// return options of the completion the handler joined.
    TrapPrefix {
        /// Completion whose `-errorcode` entry is tested.
        completion: CompletionId,
        /// The `-errorcode` prefix elements, as the registry-owned handler
        /// parse produced them.
        prefix: Vec<String>,
    },
}

/// One `switch` arm comparison against the evaluated subject.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SwitchPattern {
    /// Exact retained pattern text.
    pub text: String,
    /// Match mode selected by the registry-owned `switch` option parse.
    pub mode: SwitchMode,
    /// Whether matching is case-insensitive.
    pub nocase: bool,
    /// Whether the pattern arrived as a literal list element of a single
    /// braced `{pat body …}` block, so no substitution applies to it.
    pub literal: bool,
}

/// One `foreach`/`lmap` iterator group in an executable list-cursor loop.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IteratorGroup {
    /// Evaluated Tcl list this group iterates.
    pub list: ExecutableValueId,
    /// Loop-variable cells written once per iteration, in list order.
    pub variables: Vec<CellReference>,
}

/// The executable shape a structured control region was projected into.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum StructuredRegionKind {
    /// `if`/`elseif`/`else` decision edges.
    Conditional,
    /// A `while` or `for` loop header with an explicit back edge.
    ConditionLoop,
    /// A `foreach`/`lmap` list-cursor loop.
    CursorLoop,
    /// A `catch` region with one completion-joining handler edge.
    Catch,
    /// A `try` region with completion-class handler edges and `finally`.
    Try,
    /// A `switch` decision tree over registry-parsed pattern/body arms.
    Switch,
}

impl StructuredRegionKind {
    /// Stable Explorer/API spelling.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Conditional => "conditional",
            Self::ConditionLoop => "condition-loop",
            Self::CursorLoop => "cursor-loop",
            Self::Catch => "catch",
            Self::Try => "try",
            Self::Switch => "switch",
        }
    }
}

/// A structured control region whose interior is now executable edges.
///
/// The instruction carrying this sits where the region's interior edges join,
/// because that is where the region's completion becomes available: every
/// operand evaluation, condition test, handler edge, and body statement is a
/// separate instruction reached before it.  It performs no work of its own; it
/// is the region's stable identity for plans, provenance, and diagnostics.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StructuredRegion {
    /// Completion produced by the region as a whole.
    pub completion: CompletionId,
    /// Registry-owned structural identity.
    pub descriptor: LoweringHookId,
    /// Which executable projection this region received.
    pub kind: StructuredRegionKind,
    /// Exact structured source-IR payload, retained as provenance only.
    /// Shared, because it holds every nested body: a copy per clone of the
    /// function grows with the square of the nesting depth.
    pub statement: std::sync::Arc<Statement>,
    /// Stable source-semantic node that originated the region.
    pub node: NodeId,
    /// Full source site and provenance.
    pub source: SourceSite,
}

impl StructuredRegion {
    /// Return the target-neutral operation identity of this region.
    #[must_use]
    pub const fn semantic_operation(&self) -> SemanticOperationId {
        SemanticOperationId::StructuredLowering(self.descriptor)
    }
}

/// The exact cell and completion footprint of an already-lowered operation.
///
/// This replaces the conservative all-world default for the assignment,
/// increment, expression, and return operations that
/// [`ExecutableInstruction::ExecuteLowered`] carries.  It is derived from the
/// registry-owned [`LoweringHookId`] the statement already holds and from the
/// statement's own retained operands — never from a command spelling.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoweredFootprint {
    /// Cells the operation writes.
    pub writes: Vec<CellReference>,
    /// Cells the operation reads.
    pub reads: Vec<CellReference>,
    /// `true` when the operation reads cells this footprint does not bound.
    pub reads_unbounded: bool,
    /// `true` when evaluating the operands can run arbitrary Tcl commands, so
    /// the cell footprint bounds only this operation's own accesses.
    pub runs_commands: bool,
    /// Completion codes the operation can produce, in ascending code order.
    pub completion: Vec<CompletionCode>,
}

impl LoweredFootprint {
    /// The conservative footprint: unbounded reads, unbounded writes, and an
    /// unknown completion set.
    #[must_use]
    pub fn conservative() -> Self {
        Self {
            writes: vec![CellReference::Computed],
            reads: Vec::new(),
            reads_unbounded: true,
            runs_commands: true,
            completion: Vec::new(),
        }
    }

    /// Whether this footprint bounds the operation's cell accesses at all.
    #[must_use]
    pub fn is_bounded(&self) -> bool {
        !self.runs_commands
            && !self.reads_unbounded
            && self
                .writes
                .iter()
                .chain(&self.reads)
                .all(|cell| cell.name().is_some())
    }
}

/// Residual wrapper completion after its semantic script phases have executed.
///
/// `invocation` retains evaluated argv and original dispatch for a backend that
/// declines the entire region. It must never invoke that wrapper after running
/// the expanded phases: that would execute the scripts twice.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EvaluatedRegionCompletion {
    /// Original invocation and wrapper-owned effects/completion contract.
    pub invocation: GenericInvoke,
    /// Proved shared phase/completion plan.
    pub region: crate::execution_region::EvaluatedBodyRegion,
    /// First phase block; targets must decline before entering this block.
    pub entry: ExecutableBlockId,
}

/// One operation in an executable semantic block.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExecutableInstruction {
    /// Evaluate one non-expanded Tcl word.
    ///
    /// Word evaluation may perform variable or command substitution and may
    /// therefore return a non-OK completion before the command is resolved.
    EvaluateWord {
        /// Value available only on normal completion.
        value: ExecutableValueId,
        /// Completion of word evaluation.
        completion: CompletionId,
        /// Structured source word evaluated by this operation.
        word: WordExpr,
        /// Command node containing the word.
        node: NodeId,
        /// Exact word source site.
        source: SourceSite,
    },
    /// Expand one already-evaluated `{*}` word into argv elements.
    ///
    /// Parsing the value as a Tcl list can itself fail, so it has an explicit
    /// completion which must be dispatched before a following invocation.
    ExpandWord {
        /// Expanded argv fragment available only on normal completion.
        value: ExecutableValueId,
        /// Completion of Tcl list expansion.
        completion: CompletionId,
        /// Evaluated inner word to parse as a Tcl list.
        input: ExecutableValueId,
        /// Original `{*}` word retained for source provenance and diagnostics.
        original: WordExpr,
        /// Command node containing the word.
        node: NodeId,
        /// Exact expansion source site.
        source: SourceSite,
    },
    /// Assemble an argv vector after every word and expansion succeeded.
    BuildArgv {
        /// Constructed argv vector.
        argv: ExecutableArgvId,
        /// Completion of argv allocation and assembly.
        ///
        /// This remains explicit because allocation or list construction can
        /// fail before runtime command resolution begins.
        completion: CompletionId,
        /// Entries in Tcl left-to-right word order.
        entries: Vec<ArgvEntry>,
    },
    /// Execute the registry-resolved operation through the generic Tcl
    /// invocation path.
    ///
    /// This is intentionally still generic. It records semantic-operation
    /// metadata for common analyses but does not select native, bytecode,
    /// WASM, BPF, or any other target implementation.
    Invoke(GenericInvoke),
    /// Execute an already-lowered target-neutral structural operation.
    ///
    /// This is not a specialisation.  It preserves the registry descriptor
    /// and exact source-IR operands while all backends continue to decline the
    /// shape explicitly.
    ExecuteLowered(LoweredOperation),
    /// Execute one structured region through a conservative compatibility
    /// boundary without declining the containing function.
    ExecuteOpaqueRegion(OpaqueRegion),
    /// Evaluate one structured-control operand into a Tcl value.
    ///
    /// A condition or list operand may substitute variables and run commands,
    /// so it can complete abnormally before the control decision is taken;
    /// its completion must therefore be dispatched before the value is used.
    EvaluateExpr {
        /// Value available only on normal completion.
        value: ExecutableValueId,
        /// Completion of operand evaluation.
        completion: CompletionId,
        /// The retained structured operand.
        expr: ExecutableExpr,
        /// Structured statement containing the operand.
        node: NodeId,
        /// Exact operand source site.
        source: SourceSite,
    },
    /// Compare one `switch` arm pattern against the evaluated subject.
    ///
    /// A dynamic or regular-expression pattern stays one opaque comparison,
    /// but the arm bodies it selects between are real blocks.
    MatchPattern {
        /// Boolean match result, available only on normal completion.
        value: ExecutableValueId,
        /// Completion of the comparison, which a bad pattern can fail.
        completion: CompletionId,
        /// Evaluated `switch` subject.
        subject: ExecutableValueId,
        /// The arm's retained pattern and match mode.
        pattern: SwitchPattern,
        /// Structured statement containing the arm.
        node: NodeId,
        /// Exact pattern source site.
        source: SourceSite,
    },
    /// Step a `foreach`/`lmap` list cursor and bind this iteration's variables.
    ///
    /// This is the loop header: it consumes the evaluated per-group lists,
    /// writes each group's loop-variable cells for the iteration about to run,
    /// and produces the boolean the following `Branch` tests.  Parsing an
    /// operand as a Tcl list and writing a loop variable can both fail, so it
    /// carries a completion.
    IterateLists {
        /// True when another iteration exists, available on normal completion.
        has_iteration: ExecutableValueId,
        /// Completion of the list parse and the per-iteration cell writes.
        completion: CompletionId,
        /// Iterator groups in source order.
        groups: Vec<IteratorGroup>,
        /// Structured statement that owns the loop.
        node: NodeId,
        /// Exact loop source site.
        source: SourceSite,
    },
    /// Receive the completion triple that transferred control to this handler.
    ///
    /// This is the completion analogue of a φ: `catch` and `try` join every
    /// abrupt edge out of their body, so the handler names the joined triple
    /// rather than any one producer's.
    JoinCompletion {
        /// The joined completion observed by this handler block.
        completion: CompletionId,
        /// Structured statement that established the handler.
        node: NodeId,
        /// Exact handler source site.
        source: SourceSite,
    },
    /// Write one part of an already-dispatched completion into a Tcl cell.
    ///
    /// `catch`'s result and options variables, and a `try` handler's bindings,
    /// are ordinary cell writes of a completion payload — and a cell write can
    /// itself fail, so it has its own completion.
    WriteCompletionCell {
        /// Completion of the cell write.
        completion: CompletionId,
        /// Completion whose payload is written.
        payload_of: CompletionId,
        /// Which part of that completion is written.
        payload: CompletionPayload,
        /// Destination cell.
        cell: CellReference,
        /// Structured statement that owns the binding.
        node: NodeId,
        /// Exact variable-word source site.
        source: SourceSite,
    },
    /// Produce the completion of a structured control region whose interior is
    /// now executable edges.
    CompleteStructuredRegion(StructuredRegion),
    /// Complete a proved evaluated-body wrapper without invoking its scripts again.
    CompleteEvaluatedRegion(EvaluatedRegionCompletion),
}

/// One branch of a completion-code dispatch.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CompletionCase {
    /// Tcl completion code selected by this branch.
    pub code: CompletionCode,
    /// Target block for this code.
    pub target: ExecutableBlockId,
}

/// The unique terminator of an executable semantic block.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExecutableTerminator {
    /// Continue unconditionally at another block.
    Goto(ExecutableBlockId),
    /// Continue according to a previously evaluated Tcl truth value.
    Branch {
        /// Value used as the condition.
        condition: ExecutableValueId,
        /// Target when the condition is true.
        then_target: ExecutableBlockId,
        /// Target when the condition is false.
        else_target: ExecutableBlockId,
    },
    /// External wrapper state selects whether its script phases run.
    /// Both normal alternatives remain possible until that state is resolved.
    RegionChoice {
        /// Entry of the script phases.
        enter: ExecutableBlockId,
        /// Continuation when the wrapper omits all phases.
        skip: ExecutableBlockId,
    },
    /// Dispatch the complete Tcl completion triple by its code.
    CompletionSwitch {
        /// Completion whose code selects a successor.
        completion: CompletionId,
        /// Explicit code branches, in ascending integer-code order.
        cases: Vec<CompletionCase>,
        /// Successor for every code not named in `cases`.
        default: ExecutableBlockId,
    },
    /// Return a complete Tcl completion triple from this function.
    ReturnCompletion(CompletionId),
}

/// One basic block in deterministic function order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExecutableBlock {
    /// Block identity. Its index must equal the block's position in the
    /// containing function's `blocks` vector.
    pub id: ExecutableBlockId,
    /// Instructions in execution order.
    pub instructions: Vec<ExecutableInstruction>,
    /// The block's one terminator. `None` is representable only so validation
    /// can diagnose incomplete construction before a backend ever sees it.
    pub terminator: Option<ExecutableTerminator>,
}

impl ExecutableBlock {
    /// Create an empty unterminated block for IR construction.
    #[must_use]
    pub fn new(id: ExecutableBlockId) -> Self {
        Self {
            id,
            instructions: Vec::new(),
            terminator: None,
        }
    }
}

// Immutable original argv geometry shared by the actual evaluation values.
// This owner supplies source effects only, never a handler or entered cell.
#[derive(Debug, Clone, PartialEq, Eq)]
struct OriginalExecutableArgvSource {
    tokens: crate::ir::CommandTokens,
    words: Vec<tcl_lexer::NativeWord>,
    metadata: crate::registry_invocation::OwnedInvocationMetadataContext,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct OriginalExecutableWordSource {
    owner: std::sync::Arc<OriginalExecutableArgvSource>,
    ordinal: usize,
    evaluated: WordExpr,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum OriginalExpressionEffectOwner {
    Source(std::sync::Arc<crate::ir::CommandTokens>),
    TrapPrefix,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct OriginalExpressionCommandEffect {
    expression: ExecutableExpr,
    owner: Option<OriginalExpressionEffectOwner>,
    runs_commands: Option<bool>,
}

/// One executable semantic function.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExecutableFunction {
    /// Native compilation boundary shared with the source and CFG artifacts.
    pub native_compilation_admission:
        Option<std::sync::Arc<crate::native_compilation_admission::NativeCompilationAdmission>>,
    /// Function identity that owns every block and value ID in this function.
    pub id: ExecutableFunctionId,
    /// First block to execute.
    pub entry: ExecutableBlockId,
    /// Basic blocks in deterministic ID order.
    pub blocks: Vec<ExecutableBlock>,
    // Outer None belongs only to the explicit carrierless constructor. A
    // source-built function owns Some(map), even when it has no word stages;
    // missing entries and present None both mean unavailable source geometry.
    original_word_sources:
        Option<BTreeMap<ExecutableValueId, Option<OriginalExecutableWordSource>>>,
    // Outer None is the explicit carrierless compatibility constructor. A
    // production function retains every EvaluateExpr value, with inner None
    // distinguishing a missing source owner from a complete no-command proof.
    original_expression_command_effects:
        Option<BTreeMap<ExecutableValueId, OriginalExpressionCommandEffect>>,
}

pub(crate) enum OriginalCommandEffect {
    Unavailable,
    Known(bool),
}

impl OriginalCommandEffect {
    fn from_known(effect: Option<bool>) -> Self {
        effect.map_or(Self::Unavailable, Self::Known)
    }

    pub(crate) fn into_known(self) -> Option<bool> {
        match self {
            Self::Unavailable => None,
            Self::Known(effect) => Some(effect),
        }
    }
}

impl ExecutableFunction {
    /// Build an executable function without changing existing CFG consumers.
    #[must_use]
    pub fn new(
        id: ExecutableFunctionId,
        entry: ExecutableBlockId,
        blocks: Vec<ExecutableBlock>,
    ) -> Self {
        Self {
            id,
            entry,
            blocks,
            native_compilation_admission: None,
            original_word_sources: None,
            original_expression_command_effects: None,
        }
    }

    /// Original word evaluation geometry under the retained source owner.
    /// Outer None is explicit carrierless compatibility; Unavailable is a
    /// missing/changed source owner and cannot establish effect absence.
    pub(crate) fn original_word_command_effects(
        &self,
        value: ExecutableValueId,
        word: &WordExpr,
    ) -> Option<OriginalCommandEffect> {
        let retained = self.original_word_sources.as_ref()?.get(&value);
        Some(OriginalCommandEffect::from_known((|| {
            let retained = retained?.as_ref()?;
            (retained.evaluated == *word).then_some(())?;
            let owner = &retained.owner;
            if matches!(
                owner.metadata,
                crate::registry_invocation::OwnedInvocationMetadataContext::Unavailable
            ) {
                return None;
            }
            let original = owner.words.get(retained.ordinal)?;
            let written = owner.tokens.words().get(retained.ordinal)?;
            let evaluated = if let WordExpr::Expand { word, .. } = written {
                word.as_ref()
            } else {
                written
            };
            (evaluated == word).then_some(())?;
            Some(original.executable_parts().all_parts().any(|part| {
                matches!(
                    part.part,
                    tcl_lexer::ExecutablePart::Command { .. }
                        | tcl_lexer::ExecutablePart::Expression { .. }
                        | tcl_lexer::ExecutablePart::ParseError(_)
                )
            }))
        })()))
    }

    /// Source-backed command effects for one `EvaluateExpr` value. Outer None
    /// denotes an explicitly carrierless compatibility function; Unavailable
    /// denotes a missing/changed production operand owner.
    pub(crate) fn original_expression_command_effects(
        &self,
        value: ExecutableValueId,
        expression: &ExecutableExpr,
    ) -> Option<OriginalCommandEffect> {
        let effects = self.original_expression_command_effects.as_ref()?;
        let Some(retained) = effects.get(&value) else {
            return Some(OriginalCommandEffect::Unavailable);
        };
        if retained.expression != *expression {
            return Some(OriginalCommandEffect::Unavailable);
        }
        match (&retained.owner, retained.runs_commands) {
            (Some(OriginalExpressionEffectOwner::Source(tokens)), Some(effect))
                if tokens.source_binding.is_some() =>
            {
                Some(OriginalCommandEffect::Known(effect))
            }
            (Some(OriginalExpressionEffectOwner::TrapPrefix), Some(false))
                if matches!(expression, ExecutableExpr::TrapPrefix { .. }) =>
            {
                Some(OriginalCommandEffect::Known(false))
            }
            _ => Some(OriginalCommandEffect::Unavailable),
        }
    }
}

/// Why the deliberately small source-IR compatibility builder declined input.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SourceCompatibilityDecline {
    /// The source script has no command completion to return.
    EmptyScript,
    /// A source statement had no exact executable payload and could not be
    /// isolated as an opaque region.
    UnsupportedStatement {
        /// Position in the source script.
        statement_index: usize,
        /// Source-IR statement shape.
        kind: &'static str,
    },
    /// A call or barrier lacked exact segmented command tokens.
    MissingCommandTokens {
        /// Position in the source script.
        statement_index: usize,
    },
    /// The compatibility token arrays did not agree with the source statement.
    InconsistentCommandTokens {
        /// Position in the source script.
        statement_index: usize,
    },
    /// A segmented command-token snapshot did not contain a command head.
    MissingCommandHead {
        /// Position in the source script.
        statement_index: usize,
    },
    /// The registry returned neither a resolved invocation nor a typed
    /// unresolved reason, violating its structured-resolution contract.
    IncompleteRegistryResolution {
        /// Position in the source script.
        statement_index: usize,
    },
}

/// Build executable semantic IR from the source-faithful compatibility IR.
///
/// Calls with exact segmented words retain the complete generic invocation
/// sequence. Already-lowered assignment, expression, increment, and return
/// statements become registry-identified structural operations carrying their
/// exact cell and completion footprint. Structured control — `if`, `while`,
/// `for`, `foreach`/`lmap`, `catch`, `try`, and `switch` — becomes real
/// executable blocks with branch, loop back, handler, and completion edges;
/// only the inlined-body regions (`Block`, `UpFrame`) and the structured
/// shapes explicitly listed in [`structured_region_projection`] remain typed
/// opaque regions. No optimisation or backend specialisation is enabled by
/// this adapter.
pub fn build_linear_executable_ir(
    registry: &CommandRegistry,
    context: Option<SemanticContext>,
    function: ExecutableFunctionId,
    script: &Script,
) -> Result<ExecutableFunction, SourceCompatibilityDecline> {
    build_executable_ir_with_metadata_context(registry, context.map(Into::into), function, script)
}

/// Why executable metadata could not be built from the supplied context.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExecutableMetadataBuildDecline {
    /// Missing or foreign source availability supplies no registry facts.
    ContextUnavailable,
    /// The retained source lies outside the executable compatibility subset.
    Source(SourceCompatibilityDecline),
}

/// Build under the exact supplied availability generation. Missing and
/// foreign input refuse before visiting source; nested bodies retain the same
/// owner. This supplies no Native invocation or compilation authority.
pub fn build_linear_executable_ir_with_metadata_context(
    registry: &CommandRegistry,
    context: Option<InvocationMetadataContext<'_>>,
    function: ExecutableFunctionId,
    script: &Script,
) -> Result<ExecutableFunction, ExecutableMetadataBuildDecline> {
    let context = context
        .filter(|context| context.matches_registry(registry))
        .ok_or(ExecutableMetadataBuildDecline::ContextUnavailable)?;
    build_executable_ir_with_metadata_context(registry, Some(context), function, script)
        .map_err(ExecutableMetadataBuildDecline::Source)
}

fn build_executable_ir_with_metadata_context(
    registry: &CommandRegistry,
    context: Option<InvocationMetadataContext<'_>>,
    function: ExecutableFunctionId,
    script: &Script,
) -> Result<ExecutableFunction, SourceCompatibilityDecline> {
    if script.statements.is_empty() {
        return Err(SourceCompatibilityDecline::EmptyScript);
    }
    let mut builder = FunctionBuilder::new(function);
    let entry = builder.new_block();
    // The function body's own last statement returns the function completion
    // directly, so a straight-line body keeps exactly the block shape it had
    // before structured control became executable.
    let tail = builder.emit_script(
        registry,
        context,
        script,
        &[],
        entry,
        ControlContext::FUNCTION_BODY,
        true,
    )?;
    match tail {
        ScriptTail::Returns => {}
        ScriptTail::Falls {
            block,
            completion: Some(completion),
        } => builder.terminate(block, ExecutableTerminator::ReturnCompletion(completion)),
        ScriptTail::Falls {
            completion: None, ..
        } => return Err(SourceCompatibilityDecline::EmptyScript),
    }
    let mut executable = ExecutableFunction::new(function, entry, builder.blocks);
    // This is unconditional at production function assembly, including when
    // the builder emitted only non-word operations and the map is empty.
    executable.original_word_sources = Some(builder.original_word_sources);
    executable.original_expression_command_effects =
        Some(builder.original_expression_command_effects);
    executable.native_compilation_admission =
        crate::native_compilation_admission::retained_script_admission(script);
    debug_assert!(
        executable.validate().is_ok(),
        "compatibility builder emitted invalid executable IR: {:?}",
        executable.validate()
    );
    Ok(executable)
}

#[cfg(test)]
mod tests;
