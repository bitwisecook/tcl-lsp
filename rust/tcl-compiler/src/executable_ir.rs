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

use std::collections::{BTreeMap, BTreeSet};

use tcl_core_types::Code as CompletionCode;
use tcl_registry::hooks::LoweringHookId;
use tcl_registry::model::semantic::SemanticContext;
use tcl_registry::{CommandRegistry, SemanticOperationId};

use crate::expr_ast::ExprNode;
use crate::ir::{NodeId, Script, SourceSite, Statement, SwitchMode, WordExpr};
pub use crate::registry_invocation::{
    OwnedInvocationResolutionUnresolved, RegistryInvocationResolution as InvocationResolution,
};
use crate::registry_invocation::{RegistryInvocationDecline, resolve_word_exprs};

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
    /// Stable source-semantic node that originated this command.
    pub node: NodeId,
    /// Full command source site and provenance.
    pub source: SourceSite,
}

/// A source statement whose registry-selected structural lowering has already
/// happened.
///
/// `descriptor` is the registry-owned semantic identity that authorised the
/// lowering.  It deliberately carries no command spelling, command binding,
/// or dispatch proof: the legacy source IR no longer retains those facts, and
/// manufacturing them here would be unsound in the presence of aliases,
/// namespaces, or `rename`.  `statement` is retained as an exact executable
/// payload until all consumers have migrated off the compatibility IR.
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
    pub statement: Statement,
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

/// One executable semantic function.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExecutableFunction {
    /// Function identity that owns every block and value ID in this function.
    pub id: ExecutableFunctionId,
    /// First block to execute.
    pub entry: ExecutableBlockId,
    /// Basic blocks in deterministic ID order.
    pub blocks: Vec<ExecutableBlock>,
}

impl ExecutableFunction {
    /// Build an executable function without changing existing CFG consumers.
    #[must_use]
    pub fn new(
        id: ExecutableFunctionId,
        entry: ExecutableBlockId,
        blocks: Vec<ExecutableBlock>,
    ) -> Self {
        Self { id, entry, blocks }
    }

    /// Validate the bounded executable-IR invariants.
    pub fn validate(&self) -> Result<(), ExecutableIrValidationError> {
        let blocks = self.validate_block_layout()?;
        let definitions = self.collect_definitions()?;
        let dominance = Dominance::compute(self);
        self.validate_uses_and_terminators(&blocks, &definitions, &dominance)
    }

    fn validate_block_layout(
        &self,
    ) -> Result<BTreeSet<ExecutableBlockId>, ExecutableIrValidationError> {
        let mut blocks = BTreeSet::new();
        for (index, block) in self.blocks.iter().enumerate() {
            let expected = ExecutableBlockId::new(self.id, index);
            if block.id != expected {
                return Err(ExecutableIrValidationError::NonDeterministicBlockOrder {
                    expected,
                    actual: block.id,
                });
            }
            blocks.insert(block.id);
        }
        if !blocks.contains(&self.entry) {
            return Err(ExecutableIrValidationError::UnknownEntryBlock(self.entry));
        }
        for block in &self.blocks {
            let Some(terminator) = &block.terminator else {
                return Err(ExecutableIrValidationError::MissingTerminator(block.id));
            };
            validate_terminator_targets(terminator, self.id, &blocks)?;
        }
        Ok(blocks)
    }

    fn collect_definitions(&self) -> Result<DefinitionTables, ExecutableIrValidationError> {
        let mut definitions = DefinitionTables::default();
        for (block_index, block) in self.blocks.iter().enumerate() {
            for (instruction_index, instruction) in block.instructions.iter().enumerate() {
                validate_instruction_owners(instruction, self.id)?;
                collect_instruction_definition(
                    &mut definitions,
                    instruction,
                    InstructionPosition {
                        block: block_index,
                        instruction: instruction_index,
                    },
                )?;
            }
        }
        Ok(definitions)
    }

    fn validate_uses_and_terminators(
        &self,
        blocks: &BTreeSet<ExecutableBlockId>,
        definitions: &DefinitionTables,
        dominance: &Dominance,
    ) -> Result<(), ExecutableIrValidationError> {
        for (block_index, block) in self.blocks.iter().enumerate() {
            let Some(terminator) = block.terminator.as_ref() else {
                return Err(ExecutableIrValidationError::MissingTerminator(block.id));
            };
            for (instruction_index, instruction) in block.instructions.iter().enumerate() {
                let position = InstructionPosition {
                    block: block_index,
                    instruction: instruction_index,
                };
                validate_instruction_uses(
                    self,
                    instruction,
                    position,
                    &definitions.values,
                    &definitions.argvs,
                    dominance,
                )?;
            }
            validate_terminator(
                terminator,
                self,
                blocks,
                &definitions.values,
                &definitions.completions,
                dominance,
                InstructionPosition {
                    block: block_index,
                    instruction: block.instructions.len(),
                },
            )?;
        }
        Ok(())
    }
}

fn argv_entry_source_word<'f>(
    function: &'f ExecutableFunction,
    entry: ArgvEntry,
    values: &BTreeMap<ExecutableValueId, ValueDefinition>,
) -> Option<&'f WordExpr> {
    let definition = values.get(&entry.value())?;
    let instruction = function
        .blocks
        .get(definition.position().block)?
        .instructions
        .get(definition.position().instruction)?;
    match (entry, instruction) {
        (ArgvEntry::Value(_), ExecutableInstruction::EvaluateWord { word, .. }) => Some(word),
        (ArgvEntry::Expanded(_), ExecutableInstruction::ExpandWord { original, .. }) => {
            Some(original)
        }
        (ArgvEntry::Value(_), _) | (ArgvEntry::Expanded(_), _) => None,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
struct InstructionPosition {
    block: usize,
    instruction: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ValueDefinition {
    Word {
        position: InstructionPosition,
        completion: CompletionId,
    },
    Expansion {
        position: InstructionPosition,
        completion: CompletionId,
    },
    /// A value computed by structured control — a condition, a pattern match,
    /// or a loop cursor step.  It is never a source word, so it can never be
    /// an argv entry.
    Computed {
        position: InstructionPosition,
        completion: CompletionId,
    },
}

impl ValueDefinition {
    const fn position(self) -> InstructionPosition {
        match self {
            Self::Word { position, .. }
            | Self::Expansion { position, .. }
            | Self::Computed { position, .. } => position,
        }
    }

    const fn completion(self) -> CompletionId {
        match self {
            Self::Word { completion, .. }
            | Self::Expansion { completion, .. }
            | Self::Computed { completion, .. } => completion,
        }
    }
}

#[derive(Debug, Clone)]
struct ArgvDefinition {
    position: InstructionPosition,
    completion: CompletionId,
    entries: Vec<ArgvEntry>,
}

#[derive(Default)]
struct DefinitionTables {
    values: BTreeMap<ExecutableValueId, ValueDefinition>,
    argvs: BTreeMap<ExecutableArgvId, ArgvDefinition>,
    completions: BTreeMap<CompletionId, InstructionPosition>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct DefinedWord {
    word: WordExpr,
}

fn defined_word_at(
    function: &ExecutableFunction,
    position: InstructionPosition,
) -> Option<DefinedWord> {
    let instruction = function
        .blocks
        .get(position.block)?
        .instructions
        .get(position.instruction)?;
    let ExecutableInstruction::EvaluateWord { word, .. } = instruction else {
        return None;
    };
    Some(DefinedWord { word: word.clone() })
}

fn collect_instruction_definition(
    definitions: &mut DefinitionTables,
    instruction: &ExecutableInstruction,
    position: InstructionPosition,
) -> Result<(), ExecutableIrValidationError> {
    match instruction {
        ExecutableInstruction::EvaluateWord {
            value,
            completion,
            word,
            source,
            ..
        } => {
            if matches!(word, WordExpr::Expand { .. }) {
                return Err(ExecutableIrValidationError::ExpandedWordEvaluatedDirectly {
                    value: *value,
                });
            }
            if source != word.source() {
                return Err(ExecutableIrValidationError::WordSourceMismatch { value: *value });
            }
            insert_value_definition(
                &mut definitions.values,
                *value,
                ValueDefinition::Word {
                    position,
                    completion: *completion,
                },
            )?;
            insert_completion_definition(&mut definitions.completions, *completion, position)
        }
        ExecutableInstruction::ExpandWord {
            value,
            completion,
            original,
            source,
            ..
        } => {
            let WordExpr::Expand {
                source: original_source,
                ..
            } = original
            else {
                return Err(
                    ExecutableIrValidationError::ExpansionOriginalIsNotExpanded { value: *value },
                );
            };
            if source != original_source {
                return Err(ExecutableIrValidationError::ExpansionSourceMismatch { value: *value });
            }
            insert_value_definition(
                &mut definitions.values,
                *value,
                ValueDefinition::Expansion {
                    position,
                    completion: *completion,
                },
            )?;
            insert_completion_definition(&mut definitions.completions, *completion, position)
        }
        ExecutableInstruction::BuildArgv {
            argv,
            completion,
            entries,
        } => {
            insert_argv_definition(
                &mut definitions.argvs,
                *argv,
                position,
                *completion,
                entries.clone(),
            )?;
            insert_completion_definition(&mut definitions.completions, *completion, position)
        }
        ExecutableInstruction::Invoke(invoke) => {
            insert_completion_definition(&mut definitions.completions, invoke.completion, position)
        }
        ExecutableInstruction::ExecuteLowered(operation) => {
            validate_lowered_operation(operation)?;
            insert_completion_definition(
                &mut definitions.completions,
                operation.completion,
                position,
            )
        }
        ExecutableInstruction::ExecuteOpaqueRegion(region) => {
            validate_opaque_region(region)?;
            insert_completion_definition(&mut definitions.completions, region.completion, position)
        }
        ExecutableInstruction::EvaluateExpr {
            value, completion, ..
        }
        | ExecutableInstruction::MatchPattern {
            value, completion, ..
        } => {
            insert_value_definition(
                &mut definitions.values,
                *value,
                ValueDefinition::Computed {
                    position,
                    completion: *completion,
                },
            )?;
            insert_completion_definition(&mut definitions.completions, *completion, position)
        }
        ExecutableInstruction::IterateLists {
            has_iteration,
            completion,
            ..
        } => {
            insert_value_definition(
                &mut definitions.values,
                *has_iteration,
                ValueDefinition::Computed {
                    position,
                    completion: *completion,
                },
            )?;
            insert_completion_definition(&mut definitions.completions, *completion, position)
        }
        ExecutableInstruction::JoinCompletion { completion, .. }
        | ExecutableInstruction::WriteCompletionCell { completion, .. } => {
            insert_completion_definition(&mut definitions.completions, *completion, position)
        }
        ExecutableInstruction::CompleteStructuredRegion(region) => {
            validate_structured_region(region)?;
            insert_completion_definition(&mut definitions.completions, region.completion, position)
        }
    }
}

fn validate_structured_region(
    region: &StructuredRegion,
) -> Result<(), ExecutableIrValidationError> {
    if region.source.span != region.statement.span() {
        return Err(ExecutableIrValidationError::OperationSourceMismatch {
            completion: region.completion,
        });
    }
    if structured_region_projection(&region.statement).map(|(descriptor, kind)| (descriptor, kind))
        != Some((region.descriptor, region.kind))
    {
        return Err(ExecutableIrValidationError::OperationDescriptorMismatch {
            completion: region.completion,
        });
    }
    Ok(())
}

fn validate_lowered_operation(
    operation: &LoweredOperation,
) -> Result<(), ExecutableIrValidationError> {
    if operation.source.span != operation.statement.span() {
        return Err(ExecutableIrValidationError::OperationSourceMismatch {
            completion: operation.completion,
        });
    }
    if lowered_operation_descriptor(&operation.statement) != Some(operation.descriptor) {
        return Err(ExecutableIrValidationError::OperationDescriptorMismatch {
            completion: operation.completion,
        });
    }
    if operation.footprint != lowered_operation_footprint(&operation.statement) {
        return Err(ExecutableIrValidationError::OperationFootprintMismatch {
            completion: operation.completion,
        });
    }
    Ok(())
}

fn validate_opaque_region(region: &OpaqueRegion) -> Result<(), ExecutableIrValidationError> {
    if region.source.span != region.statement.span() {
        return Err(ExecutableIrValidationError::OperationSourceMismatch {
            completion: region.completion,
        });
    }
    let actual = region.descriptor.map_or(
        OpaqueRegionDescriptor::Unidentified,
        OpaqueRegionDescriptor::Identified,
    );
    if opaque_region_descriptor(&region.statement) != Some(actual) {
        return Err(ExecutableIrValidationError::OperationDescriptorMismatch {
            completion: region.completion,
        });
    }
    Ok(())
}

fn insert_value_definition(
    values: &mut BTreeMap<ExecutableValueId, ValueDefinition>,
    value: ExecutableValueId,
    definition: ValueDefinition,
) -> Result<(), ExecutableIrValidationError> {
    if values.insert(value, definition).is_some() {
        return Err(ExecutableIrValidationError::DuplicateValueDefinition(value));
    }
    Ok(())
}

fn insert_argv_definition(
    argvs: &mut BTreeMap<ExecutableArgvId, ArgvDefinition>,
    argv: ExecutableArgvId,
    position: InstructionPosition,
    completion: CompletionId,
    entries: Vec<ArgvEntry>,
) -> Result<(), ExecutableIrValidationError> {
    if argvs
        .insert(
            argv,
            ArgvDefinition {
                position,
                completion,
                entries,
            },
        )
        .is_some()
    {
        return Err(ExecutableIrValidationError::DuplicateArgvDefinition(argv));
    }
    Ok(())
}

fn insert_completion_definition(
    completions: &mut BTreeMap<CompletionId, InstructionPosition>,
    completion: CompletionId,
    position: InstructionPosition,
) -> Result<(), ExecutableIrValidationError> {
    if completions.insert(completion, position).is_some() {
        return Err(ExecutableIrValidationError::DuplicateCompletionDefinition(
            completion,
        ));
    }
    Ok(())
}

fn validate_instruction_uses(
    function: &ExecutableFunction,
    instruction: &ExecutableInstruction,
    position: InstructionPosition,
    values: &BTreeMap<ExecutableValueId, ValueDefinition>,
    argvs: &BTreeMap<ExecutableArgvId, ArgvDefinition>,
    dominance: &Dominance,
) -> Result<(), ExecutableIrValidationError> {
    match instruction {
        ExecutableInstruction::EvaluateWord { .. }
        | ExecutableInstruction::ExecuteLowered(_)
        | ExecutableInstruction::ExecuteOpaqueRegion(_)
        | ExecutableInstruction::EvaluateExpr { .. }
        | ExecutableInstruction::JoinCompletion { .. }
        | ExecutableInstruction::CompleteStructuredRegion(_) => Ok(()),
        ExecutableInstruction::MatchPattern { subject, .. } => {
            require_available_value(function, values, *subject, position, dominance).map(drop)
        }
        ExecutableInstruction::IterateLists { groups, .. } => {
            for group in groups {
                require_available_value(function, values, group.list, position, dominance)?;
            }
            Ok(())
        }
        ExecutableInstruction::WriteCompletionCell { .. } => Ok(()),
        ExecutableInstruction::ExpandWord {
            value,
            input,
            original,
            ..
        } => validate_expansion_use(
            function, *value, *input, original, position, values, dominance,
        ),
        ExecutableInstruction::BuildArgv { argv, entries, .. } => {
            validate_argv_build(function, *argv, entries, position, values, dominance)
        }
        ExecutableInstruction::Invoke(invoke) => {
            validate_invoke_use(function, invoke, position, values, argvs, dominance)
        }
    }
}

fn validate_expansion_use(
    function: &ExecutableFunction,
    value: ExecutableValueId,
    input: ExecutableValueId,
    original: &WordExpr,
    position: InstructionPosition,
    values: &BTreeMap<ExecutableValueId, ValueDefinition>,
    dominance: &Dominance,
) -> Result<(), ExecutableIrValidationError> {
    let definition = require_available_value(function, values, input, position, dominance)?;
    let ValueDefinition::Word {
        position: input_position,
        ..
    } = definition
    else {
        return Err(ExecutableIrValidationError::ExpansionInputIsNotWord { input });
    };
    let WordExpr::Expand { word: inner, .. } = original else {
        return Err(ExecutableIrValidationError::ExpansionOriginalIsNotExpanded { value });
    };
    let Some(DefinedWord { word, .. }) = defined_word_at(function, input_position) else {
        return Err(ExecutableIrValidationError::ExpansionInputIsNotWord { input });
    };
    if &word != inner.as_ref() {
        return Err(ExecutableIrValidationError::ExpansionInnerWordMismatch { input });
    }
    Ok(())
}

fn validate_argv_build(
    function: &ExecutableFunction,
    argv: ExecutableArgvId,
    entries: &[ArgvEntry],
    position: InstructionPosition,
    values: &BTreeMap<ExecutableValueId, ValueDefinition>,
    dominance: &Dominance,
) -> Result<(), ExecutableIrValidationError> {
    let mut previous = None;
    let mut seen_entries = BTreeSet::new();
    for entry in entries {
        let value = entry.value();
        if !seen_entries.insert(value) {
            return Err(ExecutableIrValidationError::RepeatedArgvValue { argv, value });
        }
        let definition = require_available_value(function, values, value, position, dominance)?;
        validate_argv_entry(argv, *entry, value, definition)?;
        if let Some(previous) = previous
            && !definition_precedes(previous, definition.position(), dominance)
        {
            return Err(ExecutableIrValidationError::ArgvEvaluationOutOfOrder { argv, value });
        }
        previous = Some(definition.position());
    }
    Ok(())
}

fn validate_argv_entry(
    argv: ExecutableArgvId,
    entry: ArgvEntry,
    value: ExecutableValueId,
    definition: ValueDefinition,
) -> Result<(), ExecutableIrValidationError> {
    match (entry, definition) {
        (ArgvEntry::Value(_), ValueDefinition::Word { .. })
        | (ArgvEntry::Expanded(_), ValueDefinition::Expansion { .. }) => Ok(()),
        (ArgvEntry::Value(_), ValueDefinition::Expansion { .. }) => {
            Err(ExecutableIrValidationError::ExpandedValueUsedAsWord { argv, value })
        }
        (ArgvEntry::Expanded(_), ValueDefinition::Word { .. }) => {
            Err(ExecutableIrValidationError::WordValueUsedAsExpansion { argv, value })
        }
        (_, ValueDefinition::Computed { .. }) => {
            Err(ExecutableIrValidationError::ComputedValueUsedInArgv { argv, value })
        }
    }
}

fn validate_invoke_use(
    function: &ExecutableFunction,
    invoke: &GenericInvoke,
    position: InstructionPosition,
    values: &BTreeMap<ExecutableValueId, ValueDefinition>,
    argvs: &BTreeMap<ExecutableArgvId, ArgvDefinition>,
    dominance: &Dominance,
) -> Result<(), ExecutableIrValidationError> {
    let argv = require_available_argv(function, argvs, invoke.argv, position, dominance)?;
    if argv.entries.is_empty() {
        return Err(ExecutableIrValidationError::InvocationHasEmptyArgv {
            completion: invoke.completion,
        });
    }
    if argv.entries.len() != invoke.original_words.len() {
        return Err(ExecutableIrValidationError::InvocationWordsDoNotMatchArgv {
            completion: invoke.completion,
            words: invoke.original_words.len(),
            entries: argv.entries.len(),
        });
    }
    for (word_index, (entry, original)) in
        argv.entries.iter().zip(&invoke.original_words).enumerate()
    {
        if argv_entry_source_word(function, *entry, values) != Some(original) {
            return Err(
                ExecutableIrValidationError::InvocationWordDoesNotMatchArgv {
                    completion: invoke.completion,
                    word_index,
                },
            );
        }
    }
    Ok(())
}

fn require_available_value(
    function: &ExecutableFunction,
    values: &BTreeMap<ExecutableValueId, ValueDefinition>,
    value: ExecutableValueId,
    use_position: InstructionPosition,
    dominance: &Dominance,
) -> Result<ValueDefinition, ExecutableIrValidationError> {
    let Some(definition) = values.get(&value).copied() else {
        return Err(ExecutableIrValidationError::UndefinedValue(value));
    };
    require_normal_availability(
        function,
        definition.position(),
        definition.completion(),
        use_position,
        dominance,
        ExecutableIrValidationError::ValueUsedBeforeDefinition(value),
        ExecutableIrValidationError::ValueNotAvailableOnAllPaths(value),
    )?;
    Ok(definition)
}

fn require_available_argv<'a>(
    function: &ExecutableFunction,
    argvs: &'a BTreeMap<ExecutableArgvId, ArgvDefinition>,
    argv: ExecutableArgvId,
    use_position: InstructionPosition,
    dominance: &Dominance,
) -> Result<&'a ArgvDefinition, ExecutableIrValidationError> {
    let Some(definition) = argvs.get(&argv) else {
        return Err(ExecutableIrValidationError::UndefinedArgv(argv));
    };
    require_normal_availability(
        function,
        definition.position,
        definition.completion,
        use_position,
        dominance,
        ExecutableIrValidationError::ArgvUsedBeforeDefinition(argv),
        ExecutableIrValidationError::ArgvNotAvailableOnAllPaths(argv),
    )?;
    Ok(definition)
}

fn require_normal_availability(
    function: &ExecutableFunction,
    definition: InstructionPosition,
    completion: CompletionId,
    use_position: InstructionPosition,
    dominance: &Dominance,
    same_block_error: ExecutableIrValidationError,
    unavailable_error: ExecutableIrValidationError,
) -> Result<(), ExecutableIrValidationError> {
    if definition.block == use_position.block {
        return Err(same_block_error);
    }
    let Some(normal_target) = normal_successor(function, definition, completion) else {
        return Err(ExecutableIrValidationError::MissingNormalCompletionEdge { completion });
    };
    if !dominance.dominates(normal_target.index(), use_position.block) {
        return Err(unavailable_error);
    }
    Ok(())
}

fn normal_successor(
    function: &ExecutableFunction,
    definition: InstructionPosition,
    completion: CompletionId,
) -> Option<ExecutableBlockId> {
    let block = function.blocks.get(definition.block)?;
    if definition.instruction + 1 != block.instructions.len() {
        return None;
    }
    let ExecutableTerminator::CompletionSwitch {
        completion: dispatched,
        cases,
        ..
    } = block.terminator.as_ref()?
    else {
        return None;
    };
    (*dispatched == completion)
        .then(|| cases.iter().find(|case| case.code == CompletionCode::Ok))
        .flatten()
        .map(|case| case.target)
}

fn definition_precedes(
    earlier: InstructionPosition,
    later: InstructionPosition,
    dominance: &Dominance,
) -> bool {
    if earlier.block == later.block {
        earlier.instruction < later.instruction
    } else {
        dominance.dominates(earlier.block, later.block)
    }
}

/// Reachability and dominators for the executable CFG.
///
/// Definition availability must use graph structure, not the incidental order
/// in the `blocks` vector: deterministic IDs are for reproducibility, not an
/// execution schedule.
///
/// Dominators are held as the dominator **tree** — one immediate dominator per
/// block, plus a DFS entry/exit numbering of that tree — rather than as a
/// per-block `BTreeSet` of every dominator. The set form is O(blocks²) in
/// memory and clones an O(blocks)-sized set per block per fixpoint round, which
/// is fine for the handful of blocks a single proc lowers to but not for the
/// unit's *top level*: a file of several hundred procedures flattens to
/// thousands of blocks there, and the set fixpoint then dominated whole-file
/// analysis time (tens of seconds on a ~5k-line file, growing quadratically).
/// The tree is built by the Cooper/Harvey/Kennedy iterative algorithm, which is
/// near-linear in practice, and [`Self::dominates`] becomes an O(1) interval
/// test. The answers are identical — only the representation changed.
struct Dominance {
    reachable: BTreeSet<usize>,
    /// Dominator-tree DFS entry number and subtree-maximum entry number per
    /// block. `None` for a block unreachable from the entry: it dominates
    /// nothing and is dominated by nothing.
    intervals: Vec<Option<(u32, u32)>>,
}

impl Dominance {
    fn compute(function: &ExecutableFunction) -> Self {
        let count = function.blocks.len();
        let mut successors = vec![Vec::new(); count];
        let mut predecessors = vec![Vec::new(); count];
        for (index, block) in function.blocks.iter().enumerate() {
            let Some(terminator) = block.terminator.as_ref() else {
                continue;
            };
            successors[index] = terminator_successors(terminator)
                .into_iter()
                .map(ExecutableBlockId::index)
                .collect();
            for successor in &successors[index] {
                predecessors[*successor].push(index);
            }
        }

        let entry = function.entry.index();
        // Reverse postorder is both the reachable set and the visit order the
        // dominator iteration below needs to converge in few rounds.
        let order = reverse_postorder(entry, &successors, count);
        let reachable: BTreeSet<usize> = order.iter().copied().collect();
        let idom = immediate_dominators(entry, &order, &predecessors, count);
        let intervals = dominator_tree_intervals(entry, &idom, &order, count);
        Self {
            reachable,
            intervals,
        }
    }

    fn dominates(&self, definition: usize, use_block: usize) -> bool {
        if !self.reachable.contains(&use_block) {
            return definition == use_block;
        }
        // `definition` dominates `use_block` exactly when `use_block` lies in
        // `definition`'s dominator-tree subtree — a block's own interval
        // contains its entry number, so a block still dominates itself.
        let (Some((definition_in, definition_out)), Some((use_in, _))) = (
            self.intervals.get(definition).copied().flatten(),
            self.intervals.get(use_block).copied().flatten(),
        ) else {
            return false;
        };
        definition_in <= use_in && use_in <= definition_out
    }
}

/// Blocks reachable from `entry`, in reverse postorder (an iterative DFS, so a
/// deeply nested CFG cannot overflow the stack).
fn reverse_postorder(entry: usize, successors: &[Vec<usize>], count: usize) -> Vec<usize> {
    if entry >= count {
        return Vec::new();
    }
    let mut visited = vec![false; count];
    let mut postorder = Vec::new();
    visited[entry] = true;
    // Each frame is `(block, index of the next successor to walk)`.
    let mut stack = vec![(entry, 0_usize)];
    while let Some(frame) = stack.last_mut() {
        let block = frame.0;
        let next = frame.1;
        if let Some(successor) = successors[block].get(next).copied() {
            frame.1 += 1;
            if !visited[successor] {
                visited[successor] = true;
                stack.push((successor, 0));
            }
        } else {
            postorder.push(block);
            stack.pop();
        }
    }
    postorder.reverse();
    postorder
}

/// The immediate dominator of every block in `order`, by the Cooper/Harvey/
/// Kennedy iterative algorithm. `idom[entry] == Some(entry)`; a block that is
/// unreachable (absent from `order`) keeps `None`.
fn immediate_dominators(
    entry: usize,
    order: &[usize],
    predecessors: &[Vec<usize>],
    count: usize,
) -> Vec<Option<usize>> {
    let mut idom: Vec<Option<usize>> = vec![None; count];
    if entry >= count || order.is_empty() {
        return idom;
    }
    let mut rpo_number = vec![usize::MAX; count];
    for (position, block) in order.iter().enumerate() {
        rpo_number[*block] = position;
    }
    idom[entry] = Some(entry);
    let mut changed = true;
    while changed {
        changed = false;
        for block in order.iter().copied().filter(|block| *block != entry) {
            let mut candidate: Option<usize> = None;
            for predecessor in predecessors[block].iter().copied() {
                // A predecessor with no dominator yet is unreachable, or not
                // visited in this round; either way it contributes nothing.
                if idom[predecessor].is_none() {
                    continue;
                }
                candidate = Some(match candidate {
                    None => predecessor,
                    Some(current) => {
                        nearest_common_dominator(predecessor, current, &idom, &rpo_number)
                    }
                });
            }
            if candidate.is_some() && idom[block] != candidate {
                idom[block] = candidate;
                changed = true;
            }
        }
    }
    idom
}

/// Walk two blocks up the partially-built dominator tree to their nearest
/// common ancestor — the Cooper/Harvey/Kennedy `intersect`. Both walks strictly
/// decrease the reverse-postorder number and the entry is its own dominator, so
/// this always terminates.
fn nearest_common_dominator(
    mut a: usize,
    mut b: usize,
    idom: &[Option<usize>],
    rpo_number: &[usize],
) -> usize {
    while a != b {
        while rpo_number[a] > rpo_number[b] {
            match idom[a] {
                Some(parent) if parent != a => a = parent,
                // Reached the entry (or an unparented block): it is the only
                // common ancestor available.
                _ => return b,
            }
        }
        while rpo_number[b] > rpo_number[a] {
            match idom[b] {
                Some(parent) if parent != b => b = parent,
                _ => return a,
            }
        }
    }
    a
}

/// DFS entry number and subtree-maximum entry number for every block of the
/// dominator tree rooted at `entry`, so that "does `a` dominate `b`?" is the
/// interval test `in[a] <= in[b] <= out[a]`.
fn dominator_tree_intervals(
    entry: usize,
    idom: &[Option<usize>],
    order: &[usize],
    count: usize,
) -> Vec<Option<(u32, u32)>> {
    let mut intervals = vec![None; count];
    if entry >= count || order.is_empty() {
        return intervals;
    }
    let mut children: Vec<Vec<usize>> = vec![Vec::new(); count];
    for block in order.iter().copied().filter(|block| *block != entry) {
        if let Some(parent) = idom[block] {
            children[parent].push(block);
        }
    }
    // Every non-entry node has exactly one parent, strictly earlier in reverse
    // postorder, so the tree is acyclic and this DFS visits each block once.
    let mut entry_number = vec![0_u32; count];
    let mut counter: u32 = 0;
    let mut stack = vec![(entry, 0_usize)];
    counter += 1;
    while let Some(frame) = stack.last_mut() {
        let block = frame.0;
        let next = frame.1;
        if let Some(child) = children[block].get(next).copied() {
            frame.1 += 1;
            entry_number[child] = counter;
            counter += 1;
            stack.push((child, 0));
        } else {
            intervals[block] = Some((entry_number[block], counter - 1));
            stack.pop();
        }
    }
    intervals
}

fn terminator_successors(terminator: &ExecutableTerminator) -> Vec<ExecutableBlockId> {
    match terminator {
        ExecutableTerminator::Goto(target) => vec![*target],
        ExecutableTerminator::Branch {
            then_target,
            else_target,
            ..
        } => vec![*then_target, *else_target],
        ExecutableTerminator::CompletionSwitch { cases, default, .. } => cases
            .iter()
            .map(|case| case.target)
            .chain(std::iter::once(*default))
            .collect(),
        ExecutableTerminator::ReturnCompletion(_) => Vec::new(),
    }
}

fn validate_instruction_owners(
    instruction: &ExecutableInstruction,
    function: ExecutableFunctionId,
) -> Result<(), ExecutableIrValidationError> {
    match instruction {
        ExecutableInstruction::EvaluateWord {
            value, completion, ..
        } => {
            require_value_owner(*value, function)?;
            require_completion_owner(*completion, function)?;
        }
        ExecutableInstruction::ExpandWord {
            value,
            completion,
            input,
            ..
        } => {
            require_value_owner(*value, function)?;
            require_completion_owner(*completion, function)?;
            require_value_owner(*input, function)?;
        }
        ExecutableInstruction::BuildArgv {
            argv,
            completion,
            entries,
        } => {
            require_argv_owner(*argv, function)?;
            require_completion_owner(*completion, function)?;
            for entry in entries {
                require_value_owner(entry.value(), function)?;
            }
        }
        ExecutableInstruction::Invoke(invoke) => {
            require_completion_owner(invoke.completion, function)?;
            require_argv_owner(invoke.argv, function)?;
        }
        ExecutableInstruction::ExecuteLowered(operation) => {
            require_completion_owner(operation.completion, function)?;
        }
        ExecutableInstruction::ExecuteOpaqueRegion(region) => {
            require_completion_owner(region.completion, function)?;
        }
        ExecutableInstruction::EvaluateExpr {
            value,
            completion,
            expr,
            ..
        } => {
            require_value_owner(*value, function)?;
            require_completion_owner(*completion, function)?;
            if let ExecutableExpr::TrapPrefix {
                completion: tested, ..
            } = expr
            {
                require_completion_owner(*tested, function)?;
            }
        }
        ExecutableInstruction::MatchPattern {
            value,
            completion,
            subject,
            ..
        } => {
            require_value_owner(*value, function)?;
            require_completion_owner(*completion, function)?;
            require_value_owner(*subject, function)?;
        }
        ExecutableInstruction::IterateLists {
            has_iteration,
            completion,
            groups,
            ..
        } => {
            require_value_owner(*has_iteration, function)?;
            require_completion_owner(*completion, function)?;
            for group in groups {
                require_value_owner(group.list, function)?;
            }
        }
        ExecutableInstruction::JoinCompletion { completion, .. } => {
            require_completion_owner(*completion, function)?;
        }
        ExecutableInstruction::WriteCompletionCell {
            completion,
            payload_of,
            ..
        } => {
            require_completion_owner(*completion, function)?;
            require_completion_owner(*payload_of, function)?;
        }
        ExecutableInstruction::CompleteStructuredRegion(region) => {
            require_completion_owner(region.completion, function)?;
        }
    }
    Ok(())
}

fn validate_terminator(
    terminator: &ExecutableTerminator,
    function: &ExecutableFunction,
    blocks: &BTreeSet<ExecutableBlockId>,
    values: &BTreeMap<ExecutableValueId, ValueDefinition>,
    completions: &BTreeMap<CompletionId, InstructionPosition>,
    dominance: &Dominance,
    position: InstructionPosition,
) -> Result<(), ExecutableIrValidationError> {
    match terminator {
        ExecutableTerminator::Goto(target) => require_target(*target, function.id, blocks),
        ExecutableTerminator::Branch {
            condition,
            then_target,
            else_target,
        } => {
            require_value_owner(*condition, function.id)?;
            require_available_value(function, values, *condition, position, dominance)?;
            require_target(*then_target, function.id, blocks)?;
            require_target(*else_target, function.id, blocks)
        }
        ExecutableTerminator::CompletionSwitch {
            completion,
            cases,
            default,
        } => {
            require_completion_owner(*completion, function.id)?;
            require_available_completion(completions, *completion, position, dominance)?;
            let mut previous = None;
            for case in cases {
                if previous.is_some_and(|last| case.code.as_int() <= last) {
                    return Err(ExecutableIrValidationError::CompletionCasesNotOrdered {
                        completion: *completion,
                    });
                }
                previous = Some(case.code.as_int());
                require_target(case.target, function.id, blocks)?;
            }
            require_target(*default, function.id, blocks)
        }
        ExecutableTerminator::ReturnCompletion(completion) => {
            require_completion_owner(*completion, function.id)?;
            require_available_completion(completions, *completion, position, dominance)
        }
    }
}

fn validate_terminator_targets(
    terminator: &ExecutableTerminator,
    function: ExecutableFunctionId,
    blocks: &BTreeSet<ExecutableBlockId>,
) -> Result<(), ExecutableIrValidationError> {
    match terminator {
        ExecutableTerminator::Goto(target) => require_target(*target, function, blocks),
        ExecutableTerminator::Branch {
            then_target,
            else_target,
            ..
        } => {
            require_target(*then_target, function, blocks)?;
            require_target(*else_target, function, blocks)
        }
        ExecutableTerminator::CompletionSwitch { cases, default, .. } => {
            for case in cases {
                require_target(case.target, function, blocks)?;
            }
            require_target(*default, function, blocks)
        }
        ExecutableTerminator::ReturnCompletion(_) => Ok(()),
    }
}

fn require_target(
    target: ExecutableBlockId,
    function: ExecutableFunctionId,
    blocks: &BTreeSet<ExecutableBlockId>,
) -> Result<(), ExecutableIrValidationError> {
    if target.function() != function {
        return Err(ExecutableIrValidationError::ForeignBlockId(target));
    }
    if !blocks.contains(&target) {
        return Err(ExecutableIrValidationError::UnknownTargetBlock(target));
    }
    Ok(())
}

fn require_value_owner(
    value: ExecutableValueId,
    function: ExecutableFunctionId,
) -> Result<(), ExecutableIrValidationError> {
    if value.function() != function {
        return Err(ExecutableIrValidationError::ForeignValueId(value));
    }
    Ok(())
}

fn require_argv_owner(
    argv: ExecutableArgvId,
    function: ExecutableFunctionId,
) -> Result<(), ExecutableIrValidationError> {
    if argv.function() != function {
        return Err(ExecutableIrValidationError::ForeignArgvId(argv));
    }
    Ok(())
}

fn require_completion_owner(
    completion: CompletionId,
    function: ExecutableFunctionId,
) -> Result<(), ExecutableIrValidationError> {
    if completion.function() != function {
        return Err(ExecutableIrValidationError::ForeignCompletionId(completion));
    }
    Ok(())
}

fn require_available_completion(
    completions: &BTreeMap<CompletionId, InstructionPosition>,
    completion: CompletionId,
    position: InstructionPosition,
    dominance: &Dominance,
) -> Result<(), ExecutableIrValidationError> {
    let Some(definition) = completions.get(&completion) else {
        return Err(ExecutableIrValidationError::UndefinedCompletion(completion));
    };
    if definition.block == position.block && definition.instruction >= position.instruction {
        return Err(ExecutableIrValidationError::CompletionUsedBeforeDefinition(
            completion,
        ));
    }
    if definition.block != position.block && !dominance.dominates(definition.block, position.block)
    {
        return Err(ExecutableIrValidationError::CompletionUsedBeforeDefinition(
            completion,
        ));
    }
    Ok(())
}

/// A malformed executable-IR invariant.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExecutableIrValidationError {
    /// A block ID did not equal its deterministic vector position.
    NonDeterministicBlockOrder {
        /// Expected block ID at that position.
        expected: ExecutableBlockId,
        /// Actual block ID in the function.
        actual: ExecutableBlockId,
    },
    /// The entry block is absent from the function.
    UnknownEntryBlock(ExecutableBlockId),
    /// A block was not given exactly one terminator.
    MissingTerminator(ExecutableBlockId),
    /// An instruction or terminator referenced a block owned by another function.
    ForeignBlockId(ExecutableBlockId),
    /// A branch target does not exist in this function.
    UnknownTargetBlock(ExecutableBlockId),
    /// An instruction referenced a value owned by another function.
    ForeignValueId(ExecutableValueId),
    /// An instruction referenced an argv vector owned by another function.
    ForeignArgvId(ExecutableArgvId),
    /// An instruction referenced a completion owned by another function.
    ForeignCompletionId(CompletionId),
    /// A value ID was defined twice.
    DuplicateValueDefinition(ExecutableValueId),
    /// An argv ID was defined twice.
    DuplicateArgvDefinition(ExecutableArgvId),
    /// A completion ID was defined twice.
    DuplicateCompletionDefinition(CompletionId),
    /// A value use has no definition.
    UndefinedValue(ExecutableValueId),
    /// A value was consumed before its defining operation.
    ValueUsedBeforeDefinition(ExecutableValueId),
    /// A word result did not have an explicit normal-completion edge.
    MissingNormalCompletionEdge {
        /// Completion that must be dispatched to make its result available.
        completion: CompletionId,
    },
    /// A word result was not available on every path to its use.
    ValueNotAvailableOnAllPaths(ExecutableValueId),
    /// An argv use has no `BuildArgv` definition.
    UndefinedArgv(ExecutableArgvId),
    /// An argv was invoked before its `BuildArgv` operation.
    ArgvUsedBeforeDefinition(ExecutableArgvId),
    /// An argv vector was not available on every path to its invocation.
    ArgvNotAvailableOnAllPaths(ExecutableArgvId),
    /// A completion use has no producing operation.
    UndefinedCompletion(CompletionId),
    /// A completion was returned or switched before it was produced.
    CompletionUsedBeforeDefinition(CompletionId),
    /// A `{*}` source word bypassed the expansion operation.
    ExpandedWordEvaluatedDirectly {
        /// Output value of the invalid evaluation.
        value: ExecutableValueId,
    },
    /// A word-evaluation instruction has a mismatched source site.
    WordSourceMismatch {
        /// Output value of the mismatched evaluation.
        value: ExecutableValueId,
    },
    /// An expansion input was not produced by word evaluation.
    ExpansionInputIsNotWord {
        /// Invalid expansion input.
        input: ExecutableValueId,
    },
    /// An expansion failed to retain an original `{*}` word.
    ExpansionOriginalIsNotExpanded {
        /// Output expansion value.
        value: ExecutableValueId,
    },
    /// An expansion instruction has a mismatched source site.
    ExpansionSourceMismatch {
        /// Output expansion value.
        value: ExecutableValueId,
    },
    /// The evaluated inner word does not match the original `{*}` word.
    ExpansionInnerWordMismatch {
        /// Input word value.
        input: ExecutableValueId,
    },
    /// One argv vector reuses a word result, which would erase an observable
    /// second substitution or trace callback.
    RepeatedArgvValue {
        /// Constructed argv vector.
        argv: ExecutableArgvId,
        /// Reused word result.
        value: ExecutableValueId,
    },
    /// An expansion fragment was supplied as an ordinary argv word.
    ExpandedValueUsedAsWord {
        /// Constructed argv vector.
        argv: ExecutableArgvId,
        /// Expansion result used incorrectly.
        value: ExecutableValueId,
    },
    /// An ordinary word was supplied as an expansion fragment.
    WordValueUsedAsExpansion {
        /// Constructed argv vector.
        argv: ExecutableArgvId,
        /// Word result used incorrectly.
        value: ExecutableValueId,
    },
    /// Argv entries were not evaluated in source left-to-right order.
    ArgvEvaluationOutOfOrder {
        /// Constructed argv vector.
        argv: ExecutableArgvId,
        /// Entry whose evaluation was out of order.
        value: ExecutableValueId,
    },
    /// A generic invocation has no command-head word.
    InvocationHasEmptyArgv {
        /// Completion produced by the invalid invocation.
        completion: CompletionId,
    },
    /// The retained original words do not align with constructed argv entries.
    InvocationWordsDoNotMatchArgv {
        /// Completion produced by the invalid invocation.
        completion: CompletionId,
        /// Number of retained source words.
        words: usize,
        /// Number of argv entries.
        entries: usize,
    },
    /// An argv entry came from a source word other than the retained
    /// provenance word at the same position.
    InvocationWordDoesNotMatchArgv {
        /// Completion produced by the invalid invocation.
        completion: CompletionId,
        /// Position in the original word vector.
        word_index: usize,
    },
    /// A lowered operation's retained source site does not match its payload.
    OperationSourceMismatch {
        /// Completion identifying the malformed operation.
        completion: CompletionId,
    },
    /// A lowered operation's registry descriptor does not match its source-IR
    /// structural kind.
    OperationDescriptorMismatch {
        /// Completion identifying the malformed operation.
        completion: CompletionId,
    },
    /// Completion cases were not in strictly increasing Tcl integer-code order.
    CompletionCasesNotOrdered {
        /// Completion being dispatched.
        completion: CompletionId,
    },
    /// A lowered operation's retained cell/completion footprint does not match
    /// the footprint its statement projects.
    OperationFootprintMismatch {
        /// Completion identifying the malformed operation.
        completion: CompletionId,
    },
    /// A structured-control value was supplied as an argv entry, which would
    /// claim a source word the value never came from.
    ComputedValueUsedInArgv {
        /// Constructed argv vector.
        argv: ExecutableArgvId,
        /// Structured-control value used incorrectly.
        value: ExecutableValueId,
    },
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
    /// Historical token arrays did not agree with the source statement.
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
    if script.statements.is_empty() {
        return Err(SourceCompatibilityDecline::EmptyScript);
    }
    let mut builder = FunctionBuilder::new(function);
    let entry = builder.new_block();
    let (tail, completion) = builder.emit_script(
        registry,
        context,
        script,
        &[],
        entry,
        ControlContext::FUNCTION_BODY,
    )?;
    let Some(completion) = completion else {
        return Err(SourceCompatibilityDecline::EmptyScript);
    };
    builder.terminate(tail, ExecutableTerminator::ReturnCompletion(completion));
    let executable = ExecutableFunction::new(function, entry, builder.blocks);
    debug_assert!(
        executable.validate().is_ok(),
        "compatibility builder emitted invalid executable IR: {:?}",
        executable.validate()
    );
    Ok(executable)
}

/// Where an abnormal completion goes from the statement being emitted.
///
/// This is what makes "any non-OK code unwinds" a graph fact rather than a
/// backend convention: a completion whose code is not routed by one of these
/// targets reaches the `default` arm of a [`ExecutableTerminator::CompletionSwitch`]
/// and leaves the function carrying its own triple.
#[derive(Clone, Copy)]
struct ControlContext {
    /// Block that receives a `TCL_BREAK` completion, inside a loop body.
    break_target: Option<ExecutableBlockId>,
    /// Block that receives a `TCL_CONTINUE` completion, inside a loop body.
    continue_target: Option<ExecutableBlockId>,
    /// Block that joins every other abnormal completion — a `catch` or `try`
    /// handler. `None` leaves the function with the completion.
    unwind: Option<ExecutableBlockId>,
}

impl ControlContext {
    /// The function body itself: nothing is caught, so every abnormal code
    /// leaves the function.
    const FUNCTION_BODY: Self = Self {
        break_target: None,
        continue_target: None,
        unwind: None,
    };

    const fn loop_body(
        self,
        break_target: ExecutableBlockId,
        continue_target: ExecutableBlockId,
    ) -> Self {
        Self {
            break_target: Some(break_target),
            continue_target: Some(continue_target),
            unwind: self.unwind,
        }
    }

    /// A `catch`/`try` body: every abnormal code, loop control included, joins
    /// the handler instead of leaving the region.
    const fn caught_body(handler: ExecutableBlockId) -> Self {
        Self {
            break_target: None,
            continue_target: None,
            unwind: Some(handler),
        }
    }
}

struct FunctionBuilder {
    allocator: IdAllocator,
    blocks: Vec<ExecutableBlock>,
}

impl FunctionBuilder {
    fn new(function: ExecutableFunctionId) -> Self {
        Self {
            allocator: IdAllocator::new(function),
            blocks: Vec::new(),
        }
    }

    /// Allocate one empty block. Allocation order is block-ID order, which is
    /// the deterministic vector position [`ExecutableFunction::validate`]
    /// requires.
    fn new_block(&mut self) -> ExecutableBlockId {
        let id = self.allocator.block();
        self.blocks.push(ExecutableBlock::new(id));
        id
    }

    fn push(&mut self, block: ExecutableBlockId, instruction: ExecutableInstruction) {
        self.blocks[block.index()].instructions.push(instruction);
    }

    fn terminate(&mut self, block: ExecutableBlockId, terminator: ExecutableTerminator) {
        self.blocks[block.index()].terminator = Some(terminator);
    }

    /// Terminate `block` by dispatching `completion`, and return the block
    /// where normal completion continues.
    fn dispatch(
        &mut self,
        block: ExecutableBlockId,
        completion: CompletionId,
        control: ControlContext,
    ) -> ExecutableBlockId {
        let ok = self.new_block();
        let mut cases = vec![CompletionCase {
            code: CompletionCode::Ok,
            target: ok,
        }];
        if let Some(target) = control.break_target {
            cases.push(CompletionCase {
                code: CompletionCode::Break,
                target,
            });
        }
        if let Some(target) = control.continue_target {
            cases.push(CompletionCase {
                code: CompletionCode::Continue,
                target,
            });
        }
        let default = control.unwind.unwrap_or_else(|| {
            let leave = self.new_block();
            self.terminate(leave, ExecutableTerminator::ReturnCompletion(completion));
            leave
        });
        self.terminate(
            block,
            ExecutableTerminator::CompletionSwitch {
                completion,
                cases,
                default,
            },
        );
        ok
    }

    /// Emit every statement of `script` starting at the empty block `block`.
    ///
    /// Returns the still-unterminated block where normal completion continues
    /// and the completion of the last statement emitted, or `None` when the
    /// script was empty.
    fn emit_script(
        &mut self,
        registry: &CommandRegistry,
        context: Option<SemanticContext>,
        script: &Script,
        path: &[u32],
        block: ExecutableBlockId,
        control: ControlContext,
    ) -> Result<(ExecutableBlockId, Option<CompletionId>), SourceCompatibilityDecline> {
        let mut current = block;
        let mut last = None;
        for (statement_index, statement) in script.statements.iter().enumerate() {
            let node = child_node(path, statement_index);
            let (next, completion) =
                self.emit_statement(registry, context, statement, &node, current, control)?;
            current = next;
            last = Some(completion);
            // A retained `return` ends the sequence: what follows it in the
            // same script is unreachable, exactly as the source-faithful
            // lowering already recorded.
            if matches!(statement, Statement::Return { .. }) {
                break;
            }
        }
        Ok((current, last))
    }

    fn emit_statement(
        &mut self,
        registry: &CommandRegistry,
        context: Option<SemanticContext>,
        statement: &Statement,
        node: &NodeId,
        block: ExecutableBlockId,
        control: ControlContext,
    ) -> Result<(ExecutableBlockId, CompletionId), SourceCompatibilityDecline> {
        let source = SourceSite::source(statement.span());
        if let Some(descriptor) = lowered_operation_descriptor(statement) {
            let completion = self.allocator.completion();
            self.push(
                block,
                ExecutableInstruction::ExecuteLowered(LoweredOperation {
                    completion,
                    descriptor,
                    footprint: lowered_operation_footprint(statement),
                    statement: statement.clone(),
                    node: node.clone(),
                    source,
                }),
            );
            return Ok((self.dispatch(block, completion, control), completion));
        }
        if structured_region_projection(statement).is_some() {
            return self.emit_structured_region(registry, context, statement, node, block, control);
        }
        if let Some(descriptor) = opaque_region_descriptor(statement) {
            let completion = self.allocator.completion();
            self.push(
                block,
                ExecutableInstruction::ExecuteOpaqueRegion(OpaqueRegion {
                    completion,
                    descriptor: descriptor.lowering_hook(),
                    statement: statement.clone(),
                    node: node.clone(),
                    source,
                }),
            );
            return Ok((self.dispatch(block, completion, control), completion));
        }
        self.emit_call(registry, context, statement, node, block, control, source)
    }

    fn emit_call(
        &mut self,
        registry: &CommandRegistry,
        context: Option<SemanticContext>,
        statement: &Statement,
        node: &NodeId,
        block: ExecutableBlockId,
        control: ControlContext,
        source: SourceSite,
    ) -> Result<(ExecutableBlockId, CompletionId), SourceCompatibilityDecline> {
        let statement_index = node.path().last().copied().unwrap_or(0) as usize;
        let source_call = source_call(statement, statement_index)?;
        let words = exact_words(
            source_call.tokens,
            statement_index,
            source_call.command,
            source_call.args,
        )?;
        let resolution = resolve_invocation_facts(registry, context, &words, statement_index)?;
        let mut stages = Vec::new();
        let entries = plan_argv_entries(&words, node, &mut self.allocator, &mut stages);
        let argv = self.allocator.argv();
        stages.push(Stage::BuildArgv {
            argv,
            completion: self.allocator.completion(),
            entries,
        });
        stages.push(Stage::Invoke {
            argv,
            completion: self.allocator.completion(),
            resolution,
            original_words: words,
            node: node.clone(),
            source,
        });
        let mut current = block;
        let mut last = None;
        for stage in stages {
            let completion = stage.completion();
            for instruction in stage.into_instructions() {
                self.push(current, instruction);
            }
            current = self.dispatch(current, completion, control);
            last = Some(completion);
        }
        Ok((
            current,
            last.expect("a call always plans at least one stage"),
        ))
    }

    fn emit_structured_region(
        &mut self,
        registry: &CommandRegistry,
        context: Option<SemanticContext>,
        statement: &Statement,
        node: &NodeId,
        block: ExecutableBlockId,
        control: ControlContext,
    ) -> Result<(ExecutableBlockId, CompletionId), SourceCompatibilityDecline> {
        match statement {
            Statement::If {
                clauses, else_body, ..
            } => self.emit_if(
                registry,
                context,
                statement,
                clauses,
                else_body.as_ref(),
                node,
                block,
                control,
            ),
            Statement::While {
                condition,
                condition_span,
                condition_base,
                body,
                ..
            } => self.emit_condition_loop(
                registry,
                context,
                statement,
                LoopParts {
                    init: None,
                    condition,
                    condition_span: *condition_span,
                    condition_base: *condition_base,
                    next: None,
                    body,
                },
                node,
                block,
                control,
            ),
            Statement::For {
                init,
                condition,
                condition_span,
                condition_base,
                next,
                body,
                ..
            } => self.emit_condition_loop(
                registry,
                context,
                statement,
                LoopParts {
                    init: Some(init),
                    condition,
                    condition_span: *condition_span,
                    condition_base: *condition_base,
                    next: Some(next),
                    body,
                },
                node,
                block,
                control,
            ),
            Statement::Foreach {
                iterators, body, ..
            } => self.emit_cursor_loop(
                registry, context, statement, iterators, body, node, block, control,
            ),
            Statement::Catch {
                body,
                result_var,
                options_var,
                ..
            } => self.emit_catch(
                registry,
                context,
                statement,
                body,
                result_var.as_deref(),
                options_var.as_deref(),
                node,
                block,
                control,
            ),
            Statement::Try {
                body,
                handlers,
                finally_body,
                ..
            } => self.emit_try(
                registry,
                context,
                statement,
                body,
                handlers,
                finally_body.as_ref(),
                node,
                block,
                control,
            ),
            Statement::Switch {
                subject,
                subject_span,
                arms,
                default_body,
                mode,
                nocase,
                patterns_braced,
                ..
            } => self.emit_switch(
                registry,
                context,
                statement,
                SwitchParts {
                    subject,
                    subject_span: *subject_span,
                    arms,
                    default_body: default_body.as_ref(),
                    mode: *mode,
                    nocase: *nocase,
                    patterns_braced: *patterns_braced,
                },
                node,
                block,
                control,
            ),
            _ => unreachable!("structured_region_projection selected a non-structured statement"),
        }
    }

    /// Produce the region's completion where its interior edges join, then
    /// dispatch it in the enclosing control context.
    fn complete_region(
        &mut self,
        statement: &Statement,
        node: &NodeId,
        join: ExecutableBlockId,
        control: ControlContext,
    ) -> (ExecutableBlockId, CompletionId) {
        let (descriptor, kind) = structured_region_projection(statement)
            .expect("only a projected statement reaches region completion");
        let completion = self.allocator.completion();
        self.push(
            join,
            ExecutableInstruction::CompleteStructuredRegion(StructuredRegion {
                completion,
                descriptor,
                kind,
                statement: statement.clone(),
                node: node.clone(),
                source: SourceSite::source(statement.span()),
            }),
        );
        (self.dispatch(join, completion, control), completion)
    }

    fn emit_condition(
        &mut self,
        condition: &ExprNode,
        condition_span: tcl_lexer::Span,
        condition_base: Option<u32>,
        node: &NodeId,
        block: ExecutableBlockId,
        control: ControlContext,
    ) -> (ExecutableBlockId, ExecutableValueId) {
        let value = self.allocator.value();
        let completion = self.allocator.completion();
        self.push(
            block,
            ExecutableInstruction::EvaluateExpr {
                value,
                completion,
                expr: ExecutableExpr::Condition {
                    expr: Box::new(condition.clone()),
                    base: condition_base,
                },
                node: node.clone(),
                source: SourceSite::source(condition_span),
            },
        );
        (self.dispatch(block, completion, control), value)
    }

    #[allow(clippy::too_many_arguments)]
    fn emit_if(
        &mut self,
        registry: &CommandRegistry,
        context: Option<SemanticContext>,
        statement: &Statement,
        clauses: &[crate::ir::IfClause],
        else_body: Option<&Script>,
        node: &NodeId,
        block: ExecutableBlockId,
        control: ControlContext,
    ) -> Result<(ExecutableBlockId, CompletionId), SourceCompatibilityDecline> {
        let join = self.new_block();
        let mut current = block;
        for (index, clause) in clauses.iter().enumerate() {
            let (decided, condition) = self.emit_condition(
                &clause.condition,
                clause.condition_span,
                clause.condition_base,
                node,
                current,
                control,
            );
            let body_entry = self.new_block();
            let next_test = self.new_block();
            self.terminate(
                decided,
                ExecutableTerminator::Branch {
                    condition,
                    then_target: body_entry,
                    else_target: next_test,
                },
            );
            let path = child_path(node, u32::try_from(index).unwrap_or(u32::MAX));
            let (tail, _) =
                self.emit_script(registry, context, &clause.body, &path, body_entry, control)?;
            self.terminate(tail, ExecutableTerminator::Goto(join));
            current = next_test;
        }
        if let Some(else_body) = else_body {
            let path = child_path(node, u32::try_from(clauses.len()).unwrap_or(u32::MAX));
            let (tail, _) =
                self.emit_script(registry, context, else_body, &path, current, control)?;
            self.terminate(tail, ExecutableTerminator::Goto(join));
        } else {
            self.terminate(current, ExecutableTerminator::Goto(join));
        }
        Ok(self.complete_region(statement, node, join, control))
    }

    #[allow(clippy::too_many_arguments)]
    fn emit_condition_loop(
        &mut self,
        registry: &CommandRegistry,
        context: Option<SemanticContext>,
        statement: &Statement,
        parts: LoopParts<'_>,
        node: &NodeId,
        block: ExecutableBlockId,
        control: ControlContext,
    ) -> Result<(ExecutableBlockId, CompletionId), SourceCompatibilityDecline> {
        let mut current = block;
        if let Some(init) = parts.init {
            let path = child_path(node, LOOP_INIT_SLOT);
            let (tail, _) = self.emit_script(registry, context, init, &path, current, control)?;
            current = tail;
        }
        let header = self.new_block();
        self.terminate(current, ExecutableTerminator::Goto(header));
        let exit = self.new_block();
        let (decided, condition) = self.emit_condition(
            parts.condition,
            parts.condition_span,
            parts.condition_base,
            node,
            header,
            control,
        );
        let body_entry = self.new_block();
        self.terminate(
            decided,
            ExecutableTerminator::Branch {
                condition,
                then_target: body_entry,
                else_target: exit,
            },
        );
        // `continue` re-runs the `next` script of a `for` before the header;
        // a `while` continues straight at the header.
        let continue_target = if parts.next.is_some() {
            self.new_block()
        } else {
            header
        };
        let body_control = control.loop_body(exit, continue_target);
        let path = child_path(node, LOOP_BODY_SLOT);
        let (tail, _) = self.emit_script(
            registry,
            context,
            parts.body,
            &path,
            body_entry,
            body_control,
        )?;
        if let Some(next) = parts.next {
            self.terminate(tail, ExecutableTerminator::Goto(continue_target));
            let path = child_path(node, LOOP_NEXT_SLOT);
            let (next_tail, _) =
                self.emit_script(registry, context, next, &path, continue_target, control)?;
            self.terminate(next_tail, ExecutableTerminator::Goto(header));
        } else {
            self.terminate(tail, ExecutableTerminator::Goto(header));
        }
        Ok(self.complete_region(statement, node, exit, control))
    }

    #[allow(clippy::too_many_arguments)]
    fn emit_cursor_loop(
        &mut self,
        registry: &CommandRegistry,
        context: Option<SemanticContext>,
        statement: &Statement,
        iterators: &[crate::ir::ForeachIterator],
        body: &Script,
        node: &NodeId,
        block: ExecutableBlockId,
        control: ControlContext,
    ) -> Result<(ExecutableBlockId, CompletionId), SourceCompatibilityDecline> {
        let source = SourceSite::source(statement.span());
        let mut current = block;
        let mut groups = Vec::with_capacity(iterators.len());
        for iterator in iterators {
            let value = self.allocator.value();
            let completion = self.allocator.completion();
            self.push(
                current,
                ExecutableInstruction::EvaluateExpr {
                    value,
                    completion,
                    expr: ExecutableExpr::Operand {
                        text: iterator.list_arg.clone(),
                        braced: iterator.list_braced,
                    },
                    node: node.clone(),
                    source: source.clone(),
                },
            );
            current = self.dispatch(current, completion, control);
            groups.push(IteratorGroup {
                list: value,
                variables: iterator
                    .vars
                    .iter()
                    .map(|name| CellReference::from_name(name, false))
                    .collect(),
            });
        }
        let header = self.new_block();
        self.terminate(current, ExecutableTerminator::Goto(header));
        let exit = self.new_block();
        let has_iteration = self.allocator.value();
        let completion = self.allocator.completion();
        self.push(
            header,
            ExecutableInstruction::IterateLists {
                has_iteration,
                completion,
                groups,
                node: node.clone(),
                source,
            },
        );
        let decided = self.dispatch(header, completion, control);
        let body_entry = self.new_block();
        self.terminate(
            decided,
            ExecutableTerminator::Branch {
                condition: has_iteration,
                then_target: body_entry,
                else_target: exit,
            },
        );
        let body_control = control.loop_body(exit, header);
        let path = child_path(node, LOOP_BODY_SLOT);
        let (tail, _) =
            self.emit_script(registry, context, body, &path, body_entry, body_control)?;
        self.terminate(tail, ExecutableTerminator::Goto(header));
        Ok(self.complete_region(statement, node, exit, control))
    }

    #[allow(clippy::too_many_arguments)]
    fn emit_catch(
        &mut self,
        registry: &CommandRegistry,
        context: Option<SemanticContext>,
        statement: &Statement,
        body: &Script,
        result_var: Option<&str>,
        options_var: Option<&str>,
        node: &NodeId,
        block: ExecutableBlockId,
        control: ControlContext,
    ) -> Result<(ExecutableBlockId, CompletionId), SourceCompatibilityDecline> {
        let source = SourceSite::source(statement.span());
        let handler = self.new_block();
        let join = self.new_block();
        let path = child_path(node, LOOP_BODY_SLOT);
        let (tail, _) = self.emit_script(
            registry,
            context,
            body,
            &path,
            block,
            ControlContext::caught_body(handler),
        )?;
        self.terminate(tail, ExecutableTerminator::Goto(join));

        let caught = self.allocator.completion();
        self.push(
            handler,
            ExecutableInstruction::JoinCompletion {
                completion: caught,
                node: node.clone(),
                source: source.clone(),
            },
        );
        let mut current = self.new_block();
        self.terminate(handler, ExecutableTerminator::Goto(current));
        for (variable, payload) in [
            (result_var, CompletionPayload::Result),
            (options_var, CompletionPayload::Options),
        ] {
            let Some(variable) = variable else { continue };
            let completion = self.allocator.completion();
            self.push(
                current,
                ExecutableInstruction::WriteCompletionCell {
                    completion,
                    payload_of: caught,
                    payload,
                    cell: CellReference::from_name(variable, false),
                    node: node.clone(),
                    source: source.clone(),
                },
            );
            current = self.dispatch(current, completion, control);
        }
        self.terminate(current, ExecutableTerminator::Goto(join));
        Ok(self.complete_region(statement, node, join, control))
    }

    #[allow(clippy::too_many_arguments)]
    fn emit_try(
        &mut self,
        registry: &CommandRegistry,
        context: Option<SemanticContext>,
        statement: &Statement,
        body: &Script,
        handlers: &[crate::ir::TryHandler],
        finally_body: Option<&Script>,
        node: &NodeId,
        block: ExecutableBlockId,
        control: ControlContext,
    ) -> Result<(ExecutableBlockId, CompletionId), SourceCompatibilityDecline> {
        let source = SourceSite::source(statement.span());
        let dispatch_block = self.new_block();
        let finally_entry = self.new_block();
        let join = self.new_block();
        let path = child_path(node, LOOP_BODY_SLOT);
        let (tail, _) = self.emit_script(
            registry,
            context,
            body,
            &path,
            block,
            ControlContext::caught_body(dispatch_block),
        )?;
        self.terminate(tail, ExecutableTerminator::Goto(finally_entry));

        let caught = self.allocator.completion();
        self.push(
            dispatch_block,
            ExecutableInstruction::JoinCompletion {
                completion: caught,
                node: node.clone(),
                source: source.clone(),
            },
        );
        // Every handler's abnormal completion also runs `finally`, so handler
        // bodies unwind into the same finally edge as the body.
        let handler_control = ControlContext::caught_body(finally_entry);
        let handler_entries = self.emit_try_handlers(
            registry,
            context,
            handlers,
            caught,
            node,
            &source,
            finally_entry,
            handler_control,
        )?;
        let mut cases = Vec::new();
        for (code, target) in handler_entries {
            if cases.iter().any(|case: &CompletionCase| case.code == code) {
                continue;
            }
            cases.push(CompletionCase { code, target });
        }
        cases.sort_by_key(|case| case.code.as_int());
        self.terminate(
            dispatch_block,
            ExecutableTerminator::CompletionSwitch {
                completion: caught,
                cases,
                // An unhandled code still runs `finally` before it leaves.
                default: finally_entry,
            },
        );

        let finally_tail = if let Some(finally_body) = finally_body {
            let path = child_path(node, TRY_FINALLY_SLOT);
            let (tail, _) = self.emit_script(
                registry,
                context,
                finally_body,
                &path,
                finally_entry,
                ControlContext::caught_body(join),
            )?;
            tail
        } else {
            finally_entry
        };
        self.terminate(finally_tail, ExecutableTerminator::Goto(join));
        Ok(self.complete_region(statement, node, join, control))
    }

    #[allow(clippy::too_many_arguments)]
    fn emit_try_handlers(
        &mut self,
        registry: &CommandRegistry,
        context: Option<SemanticContext>,
        handlers: &[crate::ir::TryHandler],
        caught: CompletionId,
        node: &NodeId,
        source: &SourceSite,
        finally_entry: ExecutableBlockId,
        control: ControlContext,
    ) -> Result<Vec<(CompletionCode, ExecutableBlockId)>, SourceCompatibilityDecline> {
        let mut entries = Vec::new();
        for (index, handler) in handlers.iter().enumerate() {
            let Some(code) = try_handler_code(handler) else {
                continue;
            };
            let body = resolve_try_fallthrough(handlers, index);
            let entry = self.new_block();
            let mut current = entry;
            if let Some(prefix) = handler.trap_pattern.clone() {
                // A `trap` selector narrows an error by its `-errorcode`
                // prefix; the parse is the registry-owned one lowering already
                // performed, never a local re-parse of the handler grammar.
                let value = self.allocator.value();
                let completion = self.allocator.completion();
                self.push(
                    current,
                    ExecutableInstruction::EvaluateExpr {
                        value,
                        completion,
                        expr: ExecutableExpr::TrapPrefix {
                            completion: caught,
                            prefix,
                        },
                        node: node.clone(),
                        source: source.clone(),
                    },
                );
                let decided = self.dispatch(current, completion, control);
                let matched = self.new_block();
                self.terminate(
                    decided,
                    ExecutableTerminator::Branch {
                        condition: value,
                        then_target: matched,
                        else_target: finally_entry,
                    },
                );
                current = matched;
            }
            for (variable, payload) in [
                (handler.var_name.as_deref(), CompletionPayload::Result),
                (handler.options_var.as_deref(), CompletionPayload::Options),
            ] {
                let Some(variable) = variable else { continue };
                let completion = self.allocator.completion();
                self.push(
                    current,
                    ExecutableInstruction::WriteCompletionCell {
                        completion,
                        payload_of: caught,
                        payload,
                        cell: CellReference::from_name(variable, false),
                        node: node.clone(),
                        source: source.clone(),
                    },
                );
                current = self.dispatch(current, completion, control);
            }
            let path = child_path(node, TRY_HANDLER_SLOT + u32::try_from(index).unwrap_or(0));
            let (tail, _) = self.emit_script(registry, context, body, &path, current, control)?;
            self.terminate(tail, ExecutableTerminator::Goto(finally_entry));
            entries.push((code, entry));
        }
        Ok(entries)
    }

    #[allow(clippy::too_many_arguments)]
    fn emit_switch(
        &mut self,
        registry: &CommandRegistry,
        context: Option<SemanticContext>,
        statement: &Statement,
        parts: SwitchParts<'_>,
        node: &NodeId,
        block: ExecutableBlockId,
        control: ControlContext,
    ) -> Result<(ExecutableBlockId, CompletionId), SourceCompatibilityDecline> {
        let source = SourceSite::source(statement.span());
        let subject_value = self.allocator.value();
        let subject_completion = self.allocator.completion();
        self.push(
            block,
            ExecutableInstruction::EvaluateExpr {
                value: subject_value,
                completion: subject_completion,
                expr: ExecutableExpr::Operand {
                    text: parts.subject.to_owned(),
                    braced: false,
                },
                node: node.clone(),
                source: SourceSite::source(parts.subject_span),
            },
        );
        let mut current = self.dispatch(block, subject_completion, control);
        let join = self.new_block();
        // A `-` body falls through to the next arm that has one, so the shared
        // body is emitted once and every falling arm branches to it.
        let bodies: Vec<Option<ExecutableBlockId>> = parts
            .arms
            .iter()
            .map(|arm| arm.body.as_ref().map(|_| self.new_block()))
            .collect();
        for (index, arm) in parts.arms.iter().enumerate() {
            let Some(target) = next_switch_body(&bodies, index) else {
                continue;
            };
            let value = self.allocator.value();
            let completion = self.allocator.completion();
            self.push(
                current,
                ExecutableInstruction::MatchPattern {
                    value,
                    completion,
                    subject: subject_value,
                    pattern: SwitchPattern {
                        text: arm.pattern.clone(),
                        mode: parts.mode,
                        nocase: parts.nocase,
                        literal: parts.patterns_braced,
                    },
                    node: node.clone(),
                    source: SourceSite::source(arm.pattern_span),
                },
            );
            let decided = self.dispatch(current, completion, control);
            let next_test = self.new_block();
            self.terminate(
                decided,
                ExecutableTerminator::Branch {
                    condition: value,
                    then_target: target,
                    else_target: next_test,
                },
            );
            current = next_test;
        }
        for (index, arm) in parts.arms.iter().enumerate() {
            let (Some(body), Some(entry)) = (arm.body.as_ref(), bodies[index]) else {
                continue;
            };
            let path = child_path(node, u32::try_from(index).unwrap_or(u32::MAX));
            let (tail, _) = self.emit_script(registry, context, body, &path, entry, control)?;
            self.terminate(tail, ExecutableTerminator::Goto(join));
        }
        if let Some(default_body) = parts.default_body {
            let path = child_path(node, SWITCH_DEFAULT_SLOT);
            let (tail, _) =
                self.emit_script(registry, context, default_body, &path, current, control)?;
            self.terminate(tail, ExecutableTerminator::Goto(join));
        } else {
            self.terminate(current, ExecutableTerminator::Goto(join));
        }
        let _ = source;
        Ok(self.complete_region(statement, node, join, control))
    }
}

/// The `for`/`while` operands one loop projection needs.
struct LoopParts<'a> {
    init: Option<&'a Script>,
    condition: &'a ExprNode,
    condition_span: tcl_lexer::Span,
    condition_base: Option<u32>,
    next: Option<&'a Script>,
    body: &'a Script,
}

/// The registry-parsed `switch` operands one decision tree needs.
struct SwitchParts<'a> {
    subject: &'a str,
    subject_span: tcl_lexer::Span,
    arms: &'a [crate::ir::SwitchArm],
    default_body: Option<&'a Script>,
    mode: SwitchMode,
    nocase: bool,
    patterns_braced: bool,
}

/// Reserved child-node slots for the interior scripts of a structured region.
///
/// They sit above any plausible statement index so an interior script's nodes
/// can never collide with a sibling clause body's.
const LOOP_BODY_SLOT: u32 = 1 << 24;
const LOOP_INIT_SLOT: u32 = (1 << 24) + 1;
const LOOP_NEXT_SLOT: u32 = (1 << 24) + 2;
const TRY_FINALLY_SLOT: u32 = (1 << 24) + 3;
const SWITCH_DEFAULT_SLOT: u32 = (1 << 24) + 4;
const TRY_HANDLER_SLOT: u32 = 1 << 25;

fn child_node(path: &[u32], index: usize) -> NodeId {
    let mut child = path.to_vec();
    child.push(u32::try_from(index).unwrap_or(u32::MAX));
    NodeId::from_path(child)
}

fn child_path(node: &NodeId, slot: u32) -> Vec<u32> {
    let mut path = node.path().to_vec();
    path.push(slot);
    path
}

/// The body a `try` handler runs, following `-` fallthrough to the next
/// handler that has one.
fn resolve_try_fallthrough(handlers: &[crate::ir::TryHandler], index: usize) -> &Script {
    for handler in &handlers[index..] {
        if !handler.fallthrough {
            return &handler.body;
        }
    }
    &handlers[index].body
}

/// The block that runs when a `switch` arm matches, following `-` fallthrough.
fn next_switch_body(
    bodies: &[Option<ExecutableBlockId>],
    index: usize,
) -> Option<ExecutableBlockId> {
    bodies[index..].iter().find_map(|body| *body)
}

fn lowered_operation_descriptor(statement: &Statement) -> Option<LoweringHookId> {
    match statement {
        Statement::AssignConst { .. }
        | Statement::AssignExpr { .. }
        | Statement::AssignValue { .. } => Some(LoweringHookId::Set),
        Statement::Incr { .. } => Some(LoweringHookId::Incr),
        Statement::ExprEval { .. } => Some(LoweringHookId::Expr),
        Statement::Return { .. } => Some(LoweringHookId::Return),
        Statement::Call { .. }
        | Statement::Barrier { .. }
        | Statement::Block { .. }
        | Statement::UpFrame { .. }
        | Statement::If { .. }
        | Statement::For { .. }
        | Statement::While { .. }
        | Statement::Foreach { .. }
        | Statement::Catch { .. }
        | Statement::Try { .. }
        | Statement::Switch { .. } => None,
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum OpaqueRegionDescriptor {
    Unidentified,
    Identified(LoweringHookId),
}

impl OpaqueRegionDescriptor {
    const fn lowering_hook(self) -> Option<LoweringHookId> {
        match self {
            Self::Unidentified => None,
            Self::Identified(hook) => Some(hook),
        }
    }
}

fn opaque_region_descriptor(statement: &Statement) -> Option<OpaqueRegionDescriptor> {
    match statement {
        Statement::Block { .. } => Some(OpaqueRegionDescriptor::Unidentified),
        Statement::UpFrame { .. } => {
            Some(OpaqueRegionDescriptor::Identified(LoweringHookId::Uplevel))
        }
        Statement::If { .. } => Some(OpaqueRegionDescriptor::Identified(LoweringHookId::If)),
        Statement::For { .. } => Some(OpaqueRegionDescriptor::Identified(LoweringHookId::For)),
        Statement::While { .. } => Some(OpaqueRegionDescriptor::Identified(LoweringHookId::While)),
        Statement::Foreach {
            is_lmap,
            is_dict_iteration,
            is_array_iteration,
            ..
        } => Some(OpaqueRegionDescriptor::Identified(if *is_array_iteration {
            LoweringHookId::ArrayFor
        } else if *is_dict_iteration {
            LoweringHookId::Dict
        } else if *is_lmap {
            LoweringHookId::Lmap
        } else {
            LoweringHookId::Foreach
        })),
        Statement::Catch { .. } => Some(OpaqueRegionDescriptor::Identified(LoweringHookId::Catch)),
        Statement::Try { .. } => Some(OpaqueRegionDescriptor::Identified(LoweringHookId::Try)),
        Statement::Switch { .. } => {
            Some(OpaqueRegionDescriptor::Identified(LoweringHookId::Switch))
        }
        Statement::AssignConst { .. }
        | Statement::AssignExpr { .. }
        | Statement::AssignValue { .. }
        | Statement::Incr { .. }
        | Statement::ExprEval { .. }
        | Statement::Call { .. }
        | Statement::Return { .. }
        | Statement::Barrier { .. } => None,
    }
}

/// The structured projection selected for one source statement, or `None` when
/// the statement keeps a typed opaque region.
///
/// The identity is the registry-owned [`LoweringHookId`] the lowering already
/// chose; nothing here recognises a command spelling.
fn structured_region_projection(
    statement: &Statement,
) -> Option<(LoweringHookId, StructuredRegionKind)> {
    match statement {
        Statement::If { .. } => Some((LoweringHookId::If, StructuredRegionKind::Conditional)),
        Statement::While { .. } => {
            Some((LoweringHookId::While, StructuredRegionKind::ConditionLoop))
        }
        Statement::For { .. } => Some((LoweringHookId::For, StructuredRegionKind::ConditionLoop)),
        Statement::Foreach {
            is_lmap,
            is_dict_iteration,
            is_array_iteration,
            ..
        } => {
            // `dict for`/`dict map` iterate a dictionary and Tcl 9's `array
            // for` iterates an array, neither of which is the Tcl-list cursor
            // this projection models, so both keep their opaque region.
            (!*is_dict_iteration && !*is_array_iteration).then(|| {
                (
                    if *is_lmap {
                        LoweringHookId::Lmap
                    } else {
                        LoweringHookId::Foreach
                    },
                    StructuredRegionKind::CursorLoop,
                )
            })
        }
        Statement::Catch { .. } => Some((LoweringHookId::Catch, StructuredRegionKind::Catch)),
        Statement::Try { .. } => Some((LoweringHookId::Try, StructuredRegionKind::Try)),
        Statement::Switch { .. } => Some((LoweringHookId::Switch, StructuredRegionKind::Switch)),
        Statement::AssignConst { .. }
        | Statement::AssignExpr { .. }
        | Statement::AssignValue { .. }
        | Statement::Incr { .. }
        | Statement::ExprEval { .. }
        | Statement::Call { .. }
        | Statement::Return { .. }
        | Statement::Barrier { .. }
        | Statement::Block { .. }
        | Statement::UpFrame { .. } => None,
    }
}

/// The completion code a `try` handler clause selects, when its selector is
/// statically a completion code.
///
/// `trap` always selects `TCL_ERROR` and narrows it further by `-errorcode`
/// prefix; `on` names a code directly. The selector spelling is decoded by the
/// registry's completion-code table, never by a local keyword match.
fn try_handler_code(handler: &crate::ir::TryHandler) -> Option<CompletionCode> {
    if handler.trap_pattern.is_some() || handler.kind == "trap" {
        return Some(CompletionCode::Error);
    }
    tcl_registry::completion::completion_code_selector(
        &handler.match_arg,
        tcl_syntax::number::Numbers::of_profile(None),
    )
}

/// Project the exact cell and completion footprint of an already-lowered
/// operation from the statement the registry descriptor authorised.
///
/// This is deliberately computable from the statement alone: validation
/// recomputes it and rejects a retained footprint that disagrees, so no
/// consumer can be handed a footprint the IR does not itself prove.
fn lowered_operation_footprint(statement: &Statement) -> LoweredFootprint {
    match statement {
        Statement::AssignConst {
            name, name_braced, ..
        } => {
            let write = CellReference::from_name(name, *name_braced);
            let exact = write.name().is_some();
            LoweredFootprint {
                writes: vec![write],
                reads: Vec::new(),
                reads_unbounded: false,
                runs_commands: false,
                // A constant assignment to a statically named cell has no
                // operand that can fail; a computed target name can.
                completion: if exact {
                    vec![CompletionCode::Ok]
                } else {
                    vec![CompletionCode::Ok, CompletionCode::Error]
                },
            }
        }
        Statement::AssignValue {
            name,
            name_braced,
            value,
            ..
        } => {
            let operand = operand_text_footprint(value);
            LoweredFootprint {
                writes: vec![CellReference::from_name(name, *name_braced)],
                reads: Vec::new(),
                reads_unbounded: operand.reads_unbounded,
                runs_commands: operand.runs_commands,
                completion: vec![CompletionCode::Ok, CompletionCode::Error],
            }
        }
        Statement::AssignExpr {
            name,
            name_braced,
            expr,
            ..
        } => {
            let operand = expr_footprint(expr);
            LoweredFootprint {
                writes: vec![CellReference::from_name(name, *name_braced)],
                reads: operand.reads,
                reads_unbounded: operand.reads_unbounded,
                runs_commands: operand.runs_commands,
                completion: vec![CompletionCode::Ok, CompletionCode::Error],
            }
        }
        Statement::Incr {
            name,
            name_braced,
            amount,
            ..
        } => {
            let target = CellReference::from_name(name, *name_braced);
            let operand = amount
                .as_deref()
                .map_or_else(OperandFootprint::exact, operand_text_footprint);
            LoweredFootprint {
                writes: vec![target.clone()],
                // `incr` reads its target before writing it.
                reads: vec![target],
                reads_unbounded: operand.reads_unbounded,
                runs_commands: operand.runs_commands,
                completion: vec![CompletionCode::Ok, CompletionCode::Error],
            }
        }
        Statement::ExprEval { expr, .. } => {
            let operand = expr_footprint(expr);
            LoweredFootprint {
                writes: Vec::new(),
                reads: operand.reads,
                reads_unbounded: operand.reads_unbounded,
                runs_commands: operand.runs_commands,
                completion: vec![CompletionCode::Ok, CompletionCode::Error],
            }
        }
        Statement::Return {
            value,
            expr,
            braced,
            ..
        } => {
            let operand = match (expr, value) {
                (Some(expr), _) => expr_footprint(expr),
                // A braced result word suppresses substitution entirely.
                (None, Some(_)) if *braced => OperandFootprint::exact(),
                (None, Some(value)) => operand_text_footprint(value),
                (None, None) => OperandFootprint::exact(),
            };
            let mut completion = Vec::new();
            if operand.runs_commands || operand.reads_unbounded {
                completion.push(CompletionCode::Error);
            }
            completion.push(CompletionCode::Return);
            LoweredFootprint {
                writes: Vec::new(),
                reads: operand.reads,
                reads_unbounded: operand.reads_unbounded,
                runs_commands: operand.runs_commands,
                completion,
            }
        }
        Statement::Call { .. }
        | Statement::Barrier { .. }
        | Statement::Block { .. }
        | Statement::UpFrame { .. }
        | Statement::If { .. }
        | Statement::For { .. }
        | Statement::While { .. }
        | Statement::Foreach { .. }
        | Statement::Catch { .. }
        | Statement::Try { .. }
        | Statement::Switch { .. } => LoweredFootprint::conservative(),
    }
}

/// What evaluating one retained operand can read and run.
struct OperandFootprint {
    reads: Vec<CellReference>,
    reads_unbounded: bool,
    runs_commands: bool,
}

impl OperandFootprint {
    /// An operand that neither reads a cell nor runs a command.
    const fn exact() -> Self {
        Self {
            reads: Vec::new(),
            reads_unbounded: false,
            runs_commands: false,
        }
    }
}

/// The footprint of a retained operand word held as exact text.
///
/// The text has already lost the token shape needed to enumerate its variable
/// references without re-running word substitution, so a substituted operand
/// reports unbounded reads rather than a guessed list. A bracket makes it a
/// command-running operand, which no cell footprint can bound.
fn operand_text_footprint(text: &str) -> OperandFootprint {
    OperandFootprint {
        reads: Vec::new(),
        reads_unbounded: text.contains('$') || text.contains('['),
        runs_commands: text.contains('['),
    }
}

/// The footprint of a parsed Tcl expression.
fn expr_footprint(expr: &ExprNode) -> OperandFootprint {
    let unparsed = expr_has_raw(expr);
    let runs_commands = unparsed || !expr.command_texts().is_empty();
    let mut reads: Vec<CellReference> = expr
        .vars()
        .into_iter()
        .map(|name| CellReference::from_name(&name, false))
        .collect();
    reads.sort();
    reads.dedup();
    OperandFootprint {
        reads_unbounded: unparsed || reads.iter().any(|cell| cell.name().is_none()),
        reads,
        runs_commands,
    }
}

/// Whether any part of the expression fell back to unparsed raw text, which
/// can hide both variable references and command substitutions.
fn expr_has_raw(expr: &ExprNode) -> bool {
    match expr {
        ExprNode::Raw { .. } => true,
        ExprNode::Literal { .. }
        | ExprNode::String { .. }
        | ExprNode::Var { .. }
        | ExprNode::Command { .. } => false,
        ExprNode::Binary { left, right, .. } => expr_has_raw(left) || expr_has_raw(right),
        ExprNode::Unary { operand, .. } => expr_has_raw(operand),
        ExprNode::Ternary {
            condition,
            true_branch,
            false_branch,
        } => expr_has_raw(condition) || expr_has_raw(true_branch) || expr_has_raw(false_branch),
        ExprNode::Call { args, .. } => args.iter().any(expr_has_raw),
    }
}

fn plan_argv_entries(
    words: &[WordExpr],
    node: &NodeId,
    allocator: &mut IdAllocator,
    stages: &mut Vec<Stage>,
) -> Vec<ArgvEntry> {
    let mut entries = Vec::with_capacity(words.len());
    for word in words {
        entries.push(plan_word_entry(word, node, allocator, stages));
    }
    entries
}

fn plan_word_entry(
    word: &WordExpr,
    node: &NodeId,
    allocator: &mut IdAllocator,
    stages: &mut Vec<Stage>,
) -> ArgvEntry {
    if let WordExpr::Expand {
        word: inner,
        source: expansion_source,
    } = word
    {
        let input = allocator.value();
        stages.push(Stage::evaluate(
            input,
            allocator.completion(),
            inner.as_ref().clone(),
            node.clone(),
            inner.source().clone(),
        ));
        let value = allocator.value();
        stages.push(Stage::expand(
            value,
            allocator.completion(),
            input,
            word.clone(),
            node.clone(),
            expansion_source.clone(),
        ));
        ArgvEntry::Expanded(value)
    } else {
        let value = allocator.value();
        stages.push(Stage::evaluate(
            value,
            allocator.completion(),
            word.clone(),
            node.clone(),
            word.source().clone(),
        ));
        ArgvEntry::Value(value)
    }
}

struct SourceCall<'a> {
    command: &'a str,
    args: &'a [String],
    tokens: &'a crate::ir::CommandTokens,
}

fn source_call(
    statement: &Statement,
    statement_index: usize,
) -> Result<SourceCall<'_>, SourceCompatibilityDecline> {
    match statement {
        Statement::Call {
            command,
            args,
            tokens: Some(tokens),
            ..
        }
        | Statement::Barrier {
            command,
            args,
            tokens: Some(tokens),
            ..
        } => Ok(SourceCall {
            command,
            args,
            tokens,
        }),
        Statement::Call { .. } | Statement::Barrier { .. } => {
            Err(SourceCompatibilityDecline::MissingCommandTokens { statement_index })
        }
        other => Err(SourceCompatibilityDecline::UnsupportedStatement {
            statement_index,
            kind: statement_kind_name(other),
        }),
    }
}

fn exact_words(
    tokens: &crate::ir::CommandTokens,
    statement_index: usize,
    command: &str,
    args: &[String],
) -> Result<Vec<WordExpr>, SourceCompatibilityDecline> {
    if tokens.words().is_empty() {
        return Err(SourceCompatibilityDecline::MissingCommandHead { statement_index });
    }
    if !tokens.words_align_with_argv_text()
        || tokens.argv_texts.len() != args.len().saturating_add(1)
        || tokens.argv_texts.first().map(String::as_str) != Some(command)
        || tokens.argv_texts.get(1..) != Some(args)
    {
        return Err(SourceCompatibilityDecline::InconsistentCommandTokens { statement_index });
    }
    Ok(tokens.words().to_vec())
}

/// Resolve source-aware word facts without re-evaluating, flattening, or
/// re-parsing any Tcl word. The small borrowed vector exists only for this
/// registry call; `InvocationFacts`/the unresolved projection own every fact
/// retained in executable IR.
fn resolve_invocation_facts(
    registry: &CommandRegistry,
    context: Option<SemanticContext>,
    words: &[WordExpr],
    statement_index: usize,
) -> Result<InvocationResolution, SourceCompatibilityDecline> {
    match resolve_word_exprs(registry, context, words) {
        Ok(resolution) => Ok(resolution),
        Err(RegistryInvocationDecline::MissingCommandHead) => {
            Err(SourceCompatibilityDecline::MissingCommandHead { statement_index })
        }
        Err(RegistryInvocationDecline::IncompleteResolution) => {
            Err(SourceCompatibilityDecline::IncompleteRegistryResolution { statement_index })
        }
    }
}

fn statement_kind_name(statement: &Statement) -> &'static str {
    match statement {
        Statement::AssignConst { .. } => "AssignConst",
        Statement::AssignExpr { .. } => "AssignExpr",
        Statement::AssignValue { .. } => "AssignValue",
        Statement::Incr { .. } => "Incr",
        Statement::ExprEval { .. } => "ExprEval",
        Statement::Call { .. } => "Call",
        Statement::Return { .. } => "Return",
        Statement::Barrier { .. } => "Barrier",
        Statement::Block { .. } => "Block",
        Statement::UpFrame { .. } => "UpFrame",
        Statement::If { .. } => "If",
        Statement::For { .. } => "For",
        Statement::While { .. } => "While",
        Statement::Foreach { .. } => "Foreach",
        Statement::Catch { .. } => "Catch",
        Statement::Try { .. } => "Try",
        Statement::Switch { .. } => "Switch",
    }
}

struct IdAllocator {
    function: ExecutableFunctionId,
    next_block: usize,
    next_value: usize,
    next_argv: usize,
    next_completion: usize,
}

impl IdAllocator {
    const fn new(function: ExecutableFunctionId) -> Self {
        Self {
            function,
            next_block: 0,
            next_value: 0,
            next_argv: 0,
            next_completion: 0,
        }
    }

    fn block(&mut self) -> ExecutableBlockId {
        let id = ExecutableBlockId::new(self.function, self.next_block);
        self.next_block += 1;
        id
    }

    fn value(&mut self) -> ExecutableValueId {
        let id = ExecutableValueId::new(self.function, self.next_value);
        self.next_value += 1;
        id
    }

    fn argv(&mut self) -> ExecutableArgvId {
        let id = ExecutableArgvId::new(self.function, self.next_argv);
        self.next_argv += 1;
        id
    }

    fn completion(&mut self) -> CompletionId {
        let id = CompletionId::new(self.function, self.next_completion);
        self.next_completion += 1;
        id
    }
}

enum Stage {
    Evaluate {
        value: ExecutableValueId,
        completion: CompletionId,
        word: WordExpr,
        node: NodeId,
        source: SourceSite,
    },
    Expand {
        value: ExecutableValueId,
        completion: CompletionId,
        input: ExecutableValueId,
        original: WordExpr,
        node: NodeId,
        source: SourceSite,
    },
    BuildArgv {
        argv: ExecutableArgvId,
        completion: CompletionId,
        entries: Vec<ArgvEntry>,
    },
    Invoke {
        argv: ExecutableArgvId,
        completion: CompletionId,
        resolution: InvocationResolution,
        original_words: Vec<WordExpr>,
        node: NodeId,
        source: SourceSite,
    },
}

impl Stage {
    fn evaluate(
        value: ExecutableValueId,
        completion: CompletionId,
        word: WordExpr,
        node: NodeId,
        source: SourceSite,
    ) -> Self {
        Self::Evaluate {
            value,
            completion,
            word,
            node,
            source,
        }
    }

    fn expand(
        value: ExecutableValueId,
        completion: CompletionId,
        input: ExecutableValueId,
        original: WordExpr,
        node: NodeId,
        source: SourceSite,
    ) -> Self {
        Self::Expand {
            value,
            completion,
            input,
            original,
            node,
            source,
        }
    }

    const fn completion(&self) -> CompletionId {
        match self {
            Self::Evaluate { completion, .. }
            | Self::Expand { completion, .. }
            | Self::BuildArgv { completion, .. }
            | Self::Invoke { completion, .. } => *completion,
        }
    }

    fn into_instructions(self) -> Vec<ExecutableInstruction> {
        match self {
            Self::Evaluate {
                value,
                completion,
                word,
                node,
                source,
            } => vec![ExecutableInstruction::EvaluateWord {
                value,
                completion,
                word,
                node,
                source,
            }],
            Self::Expand {
                value,
                completion,
                input,
                original,
                node,
                source,
            } => vec![ExecutableInstruction::ExpandWord {
                value,
                completion,
                input,
                original,
                node,
                source,
            }],
            Self::BuildArgv {
                argv,
                completion,
                entries,
            } => vec![ExecutableInstruction::BuildArgv {
                argv,
                completion,
                entries,
            }],
            Self::Invoke {
                argv,
                completion,
                resolution,
                original_words,
                node,
                source,
            } => vec![ExecutableInstruction::Invoke(GenericInvoke {
                completion,
                argv,
                resolution,
                original_words,
                node,
                source,
            })],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use tcl_lexer::Span;
    use tcl_registry::{
        ArgRole, CompletionDescriptor, InvocationWordKind, OwnedSubcommandResolution,
        SemanticOperationId, TclType,
    };

    fn source(span: u32) -> SourceSite {
        SourceSite::source(Span::new(span, span + 1))
    }

    fn literal(text: &str, span: u32) -> WordExpr {
        WordExpr::Literal {
            text: text.to_owned(),
            source: source(span),
        }
    }

    fn substitution(text: &str, span: u32) -> WordExpr {
        WordExpr::CommandSubstitution {
            spelling: text.to_owned(),
            source: source(span),
        }
    }

    fn tokens(words: Vec<WordExpr>) -> crate::ir::CommandTokens {
        let argv_texts = words.iter().map(WordExpr::legacy_text).collect::<Vec<_>>();
        crate::ir::CommandTokens {
            argv: words.iter().map(|word| word.source().span).collect(),
            argv_texts,
            word_exprs: words,
            argv_kinds: Vec::new(),
            single_token_word: Vec::new(),
            all_tokens: Vec::new(),
            expand_word: None,
            synthetic: None,
        }
    }

    fn call(words: Vec<WordExpr>) -> Statement {
        let command = words[0].legacy_text();
        let args = words.iter().skip(1).map(WordExpr::legacy_text).collect();
        Statement::Call {
            span: Span::new(0, 20),
            command,
            canonical_command: None,
            args,
            defs: Vec::new(),
            reads: Vec::new(),
            reads_own_defs: false,
            safe_on_uninit: false,
            tokens: Some(tokens(words)),
            foreach_groups: None,
        }
    }

    fn test_context() -> SemanticContext {
        SemanticContext::for_environment("tcl8.6")
    }

    fn find_invoke(function: &ExecutableFunction) -> &GenericInvoke {
        function
            .blocks
            .iter()
            .flat_map(|block| &block.instructions)
            .find_map(|instruction| match instruction {
                ExecutableInstruction::Invoke(invoke) => Some(invoke),
                _ => None,
            })
            .expect("generic invocation")
    }

    fn build(source: &str, id: usize) -> ExecutableFunction {
        let registry = CommandRegistry::build_default();
        let module = crate::lowering::lower_to_ir(source, &registry);
        let function = build_linear_executable_ir(
            &registry,
            Some(test_context()),
            ExecutableFunctionId::new(id),
            &module.top_level,
        )
        .expect("structured control remains executable");
        function.validate().expect("valid executable IR");
        function
    }

    fn instructions(function: &ExecutableFunction) -> Vec<&ExecutableInstruction> {
        function
            .blocks
            .iter()
            .flat_map(|block| &block.instructions)
            .collect()
    }

    fn structured_regions(function: &ExecutableFunction) -> Vec<&StructuredRegion> {
        instructions(function)
            .into_iter()
            .filter_map(|instruction| match instruction {
                ExecutableInstruction::CompleteStructuredRegion(region) => Some(region),
                _ => None,
            })
            .collect()
    }

    fn region_block(function: &ExecutableFunction) -> ExecutableBlockId {
        function
            .blocks
            .iter()
            .find(|block| {
                block.instructions.iter().any(|instruction| {
                    matches!(
                        instruction,
                        ExecutableInstruction::CompleteStructuredRegion(_)
                    )
                })
            })
            .expect("structured region")
            .id
    }

    #[test]
    fn if_becomes_branch_edges_joining_at_the_region_completion() {
        let function = build("if {$enabled} {puts on} else {puts off}", 200);
        assert!(
            instructions(&function)
                .iter()
                .all(|instruction| !matches!(
                    instruction,
                    ExecutableInstruction::ExecuteOpaqueRegion(_)
                )),
            "an `if` no longer needs an opaque compatibility barrier"
        );
        let conditions: Vec<_> = instructions(&function)
            .into_iter()
            .filter_map(|instruction| match instruction {
                ExecutableInstruction::EvaluateExpr {
                    expr: ExecutableExpr::Condition { .. },
                    ..
                } => Some(instruction),
                _ => None,
            })
            .collect();
        assert_eq!(conditions.len(), 1, "one clause, one condition");
        let branches = function
            .blocks
            .iter()
            .filter(|block| matches!(block.terminator, Some(ExecutableTerminator::Branch { .. })))
            .count();
        assert_eq!(branches, 1);
        let regions = structured_regions(&function);
        assert_eq!(regions.len(), 1);
        assert_eq!(regions[0].kind, StructuredRegionKind::Conditional);
        assert_eq!(regions[0].descriptor, LoweringHookId::If);
        // Both arms converge on the block that produces the region completion.
        let join = region_block(&function);
        let predecessors = function
            .blocks
            .iter()
            .filter(|block| {
                matches!(block.terminator, Some(ExecutableTerminator::Goto(target)) if target == join)
            })
            .count();
        assert_eq!(predecessors, 2, "then and else arms both join");
    }

    #[test]
    fn while_loop_has_an_explicit_back_edge_to_its_header() {
        let function = build("while {$more} {puts tick}", 201);
        let header = function
            .blocks
            .iter()
            .find(|block| {
                matches!(
                    block.instructions.first(),
                    Some(ExecutableInstruction::EvaluateExpr {
                        expr: ExecutableExpr::Condition { .. },
                        ..
                    })
                )
            })
            .expect("loop header")
            .id;
        let back_edges = function
            .blocks
            .iter()
            .filter(|block| {
                block.id.index() > header.index()
                    && matches!(
                        block.terminator,
                        Some(ExecutableTerminator::Goto(target)) if target == header
                    )
            })
            .count();
        assert_eq!(back_edges, 1, "the loop body branches back to the header");
        assert_eq!(
            structured_regions(&function)[0].kind,
            StructuredRegionKind::ConditionLoop
        );
    }

    #[test]
    fn break_and_continue_are_routed_to_loop_targets_not_out_of_the_function() {
        let function = build("while {1} {break}", 202);
        let exit = region_block(&function);
        let routed = function
            .blocks
            .iter()
            .filter_map(|block| match &block.terminator {
                Some(ExecutableTerminator::CompletionSwitch { cases, .. }) => Some(cases),
                _ => None,
            })
            .filter(|cases| {
                cases.iter().any(|case| {
                    case.code == CompletionCode::Break && case.target == exit
                })
            })
            .count();
        assert!(
            routed > 0,
            "a completion of code TCL_BREAK inside the body reaches the loop exit"
        );
        assert!(
            function
                .blocks
                .iter()
                .filter_map(|block| match &block.terminator {
                    Some(ExecutableTerminator::CompletionSwitch { cases, .. }) => Some(cases),
                    _ => None,
                })
                .any(|cases| cases
                    .iter()
                    .any(|case| case.code == CompletionCode::Continue)),
            "the loop body also names an explicit continue target"
        );
    }

    #[test]
    fn every_non_ok_completion_leaves_the_function_at_top_level() {
        let function = build("puts hello", 203);
        for block in &function.blocks {
            let Some(ExecutableTerminator::CompletionSwitch {
                cases, default, ..
            }) = &block.terminator
            else {
                continue;
            };
            assert_eq!(
                cases.len(),
                1,
                "outside a loop only the OK edge is named; every other code unwinds"
            );
            assert!(cases[0].code.is_ok());
            let unwind = function
                .blocks
                .iter()
                .find(|candidate| candidate.id == *default)
                .expect("unwind block");
            assert!(matches!(
                unwind.terminator,
                Some(ExecutableTerminator::ReturnCompletion(_))
            ));
        }
    }

    #[test]
    fn for_loop_continues_through_its_next_script() {
        let function = build("for {set i 0} {$i < 4} {incr i} {puts $i}", 204);
        let continue_targets: BTreeSet<_> = function
            .blocks
            .iter()
            .filter_map(|block| match &block.terminator {
                Some(ExecutableTerminator::CompletionSwitch { cases, .. }) => Some(cases),
                _ => None,
            })
            .flat_map(|cases| cases.iter())
            .filter(|case| case.code == CompletionCode::Continue)
            .map(|case| case.target)
            .collect();
        assert_eq!(continue_targets.len(), 1);
        let target = *continue_targets.iter().next().expect("continue target");
        // The `for` continue target runs the loop's `next` script, so it is not
        // the header itself.
        let next_block = function
            .blocks
            .iter()
            .find(|block| block.id == target)
            .expect("continue block");
        assert!(matches!(
            next_block.instructions.first(),
            Some(ExecutableInstruction::ExecuteLowered(LoweredOperation {
                descriptor: LoweringHookId::Incr,
                ..
            }))
        ));
    }

    #[test]
    fn foreach_becomes_a_list_cursor_loop_that_binds_its_variables() {
        let function = build("foreach {a b} $pairs {puts $a}", 205);
        let cursor = instructions(&function)
            .into_iter()
            .find_map(|instruction| match instruction {
                ExecutableInstruction::IterateLists { groups, .. } => Some(groups),
                _ => None,
            })
            .expect("list-cursor loop header");
        assert_eq!(cursor.len(), 1);
        assert_eq!(
            cursor[0].variables,
            vec![
                CellReference::Named {
                    name: "a".to_owned(),
                    element: false
                },
                CellReference::Named {
                    name: "b".to_owned(),
                    element: false
                },
            ]
        );
        assert_eq!(
            structured_regions(&function)[0].kind,
            StructuredRegionKind::CursorLoop
        );
    }

    #[test]
    fn dict_iteration_keeps_its_opaque_region() {
        let function = build("dict for {k v} $d {puts $k}", 206);
        assert!(
            instructions(&function).iter().any(|instruction| matches!(
                instruction,
                ExecutableInstruction::ExecuteOpaqueRegion(_)
            )),
            "a dict cursor is not the Tcl-list cursor this projection models"
        );
    }

    #[test]
    fn catch_joins_its_abrupt_edge_and_writes_the_result_cells() {
        let function = build("catch {error boom} msg opts", 207);
        let joins = instructions(&function)
            .into_iter()
            .filter(|instruction| {
                matches!(instruction, ExecutableInstruction::JoinCompletion { .. })
            })
            .count();
        assert_eq!(joins, 1, "every abrupt edge out of the body joins once");
        let written: Vec<_> = instructions(&function)
            .into_iter()
            .filter_map(|instruction| match instruction {
                ExecutableInstruction::WriteCompletionCell { payload, cell, .. } => {
                    Some((*payload, cell.clone()))
                }
                _ => None,
            })
            .collect();
        assert_eq!(
            written,
            vec![
                (
                    CompletionPayload::Result,
                    CellReference::Named {
                        name: "msg".to_owned(),
                        element: false
                    }
                ),
                (
                    CompletionPayload::Options,
                    CellReference::Named {
                        name: "opts".to_owned(),
                        element: false
                    }
                ),
            ]
        );
        assert_eq!(
            structured_regions(&function)[0].kind,
            StructuredRegionKind::Catch
        );
    }

    #[test]
    fn try_routes_handlers_by_completion_class() {
        let function = build(
            "try {risky} on error {m} {puts $m} finally {cleanup}",
            208,
        );
        let joined = instructions(&function)
            .into_iter()
            .find_map(|instruction| match instruction {
                ExecutableInstruction::JoinCompletion { completion, .. } => Some(*completion),
                _ => None,
            })
            .expect("try joins its abrupt edges");
        let cases = function
            .blocks
            .iter()
            .find_map(|block| match &block.terminator {
                Some(ExecutableTerminator::CompletionSwitch {
                    completion, cases, ..
                }) if *completion == joined => Some(cases.clone()),
                _ => None,
            })
            .expect("the joined completion selects a handler by its code");
        assert_eq!(
            cases.iter().map(|case| case.code).collect::<Vec<_>>(),
            vec![CompletionCode::Error]
        );
        assert_eq!(
            structured_regions(&function)[0].kind,
            StructuredRegionKind::Try
        );
    }

    #[test]
    fn switch_becomes_a_decision_tree_with_shared_fallthrough_bodies() {
        let function = build("switch $mode {a - b {puts ab} c {puts c}}", 209);
        let patterns: Vec<_> = instructions(&function)
            .into_iter()
            .filter_map(|instruction| match instruction {
                ExecutableInstruction::MatchPattern { pattern, .. } => Some(pattern.text.clone()),
                _ => None,
            })
            .collect();
        assert_eq!(patterns, vec!["a", "b", "c"]);
        // `a -` falls through, so both `a` and `b` branch to the same body.
        let targets: Vec<_> = function
            .blocks
            .iter()
            .filter_map(|block| match &block.terminator {
                Some(ExecutableTerminator::Branch { then_target, .. }) => Some(*then_target),
                _ => None,
            })
            .collect();
        assert_eq!(targets.len(), 3);
        assert_eq!(targets[0], targets[1], "a fallthrough arm shares one body");
        assert_ne!(targets[1], targets[2]);
        assert_eq!(
            structured_regions(&function)[0].kind,
            StructuredRegionKind::Switch
        );
    }

    #[test]
    fn uplevel_and_inlined_blocks_stay_opaque() {
        let function = build("uplevel 1 {set outer 1}", 210);
        let region = instructions(&function)
            .into_iter()
            .find_map(|instruction| match instruction {
                ExecutableInstruction::ExecuteOpaqueRegion(region) => Some(region),
                _ => None,
            })
            .expect("an inlined uplevel body is not straight-line splice-able yet");
        assert_eq!(region.descriptor, Some(LoweringHookId::Uplevel));
    }

    #[test]
    fn constant_assignment_has_an_exact_footprint_and_cannot_fail() {
        let function = build("set counter 0", 211);
        let operation = instructions(&function)
            .into_iter()
            .find_map(|instruction| match instruction {
                ExecutableInstruction::ExecuteLowered(operation) => Some(operation),
                _ => None,
            })
            .expect("lowered assignment");
        assert_eq!(
            operation.footprint.writes,
            vec![CellReference::Named {
                name: "counter".to_owned(),
                element: false
            }]
        );
        assert!(operation.footprint.reads.is_empty());
        assert!(operation.footprint.is_bounded());
        assert_eq!(operation.footprint.completion, vec![CompletionCode::Ok]);
    }

    #[test]
    fn expression_assignment_reads_exactly_its_operand_variables() {
        let function = build("set total [expr {$a + $b}]", 212);
        let operation = instructions(&function)
            .into_iter()
            .find_map(|instruction| match instruction {
                ExecutableInstruction::ExecuteLowered(operation) => Some(operation),
                _ => None,
            })
            .expect("lowered expression assignment");
        assert_eq!(
            operation.footprint.reads,
            vec![
                CellReference::Named {
                    name: "a".to_owned(),
                    element: false
                },
                CellReference::Named {
                    name: "b".to_owned(),
                    element: false
                },
            ]
        );
        assert!(operation.footprint.is_bounded());
        assert_eq!(
            operation.footprint.completion,
            vec![CompletionCode::Ok, CompletionCode::Error]
        );
    }

    #[test]
    fn increment_reads_before_it_writes_and_a_command_operand_is_unbounded() {
        let bounded = build("incr n 2", 213);
        let operation = instructions(&bounded)
            .into_iter()
            .find_map(|instruction| match instruction {
                ExecutableInstruction::ExecuteLowered(operation) => Some(operation),
                _ => None,
            })
            .expect("lowered increment");
        assert_eq!(operation.footprint.writes, operation.footprint.reads);
        assert!(operation.footprint.is_bounded());

        let unbounded = build("set total [expr {[step] + 1}]", 214);
        let operation = instructions(&unbounded)
            .into_iter()
            .find_map(|instruction| match instruction {
                ExecutableInstruction::ExecuteLowered(operation) => Some(operation),
                _ => None,
            })
            .expect("lowered expression assignment");
        assert!(
            operation.footprint.runs_commands,
            "a command substitution in the operand is not bounded by a cell footprint"
        );
        assert!(!operation.footprint.is_bounded());
    }

    #[test]
    fn a_retained_footprint_that_disagrees_with_its_statement_is_rejected() {
        let registry = CommandRegistry::build_default();
        let module = crate::lowering::lower_to_ir("set counter 0", &registry);
        let function_id = ExecutableFunctionId::new(215);
        let mut function = build_linear_executable_ir(
            &registry,
            Some(test_context()),
            function_id,
            &module.top_level,
        )
        .expect("lowered assignment");
        for block in &mut function.blocks {
            for instruction in &mut block.instructions {
                if let ExecutableInstruction::ExecuteLowered(operation) = instruction {
                    operation.footprint.writes = vec![CellReference::Computed];
                }
            }
        }
        assert!(matches!(
            function.validate(),
            Err(ExecutableIrValidationError::OperationFootprintMismatch { .. })
        ));
    }

    #[test]
    fn a_structured_region_still_isolates_the_statements_around_it() {
        let function = build(
            "set total 0\nforeach item $items {incr total $item}\nreturn $total",
            216,
        );
        let descriptors: Vec<_> = instructions(&function)
            .into_iter()
            .filter_map(|instruction| match instruction {
                ExecutableInstruction::ExecuteLowered(operation) => Some(operation.descriptor),
                ExecutableInstruction::CompleteStructuredRegion(region) => Some(region.descriptor),
                _ => None,
            })
            .collect();
        assert!(descriptors.contains(&LoweringHookId::Set));
        assert!(descriptors.contains(&LoweringHookId::Foreach));
        assert!(descriptors.contains(&LoweringHookId::Return));
    }

    #[test]
    fn substitutions_complete_before_generic_invocation() {
        let script = Script::from_statements(vec![call(vec![
            literal("puts", 0),
            substitution("[make-message]", 5),
        ])]);
        let function = build_linear_executable_ir(
            &CommandRegistry::build_default(),
            Some(test_context()),
            ExecutableFunctionId::new(1),
            &script,
        )
        .expect("a literal command head and structured substitution are supported");
        function.validate().expect("valid executable IR");

        let mut evaluation = None;
        let mut invocation = None;
        for (block_index, block) in function.blocks.iter().enumerate() {
            for instruction in &block.instructions {
                match instruction {
                    ExecutableInstruction::EvaluateWord {
                        word: WordExpr::CommandSubstitution { .. },
                        ..
                    } => evaluation = Some(block_index),
                    ExecutableInstruction::Invoke(_) => invocation = Some(block_index),
                    _ => {}
                }
            }
        }
        assert!(evaluation < invocation);
    }

    #[test]
    fn expansion_failure_switches_before_invocation() {
        let expanded = WordExpr::Expand {
            source: source(5),
            word: Box::new(substitution("[not-a-list]", 8)),
        };
        let script = Script::from_statements(vec![call(vec![literal("puts", 0), expanded])]);
        let function = build_linear_executable_ir(
            &CommandRegistry::build_default(),
            Some(test_context()),
            ExecutableFunctionId::new(2),
            &script,
        )
        .expect("expansion is retained as an explicit operation");
        function.validate().expect("valid executable IR");

        let expansion_block = function
            .blocks
            .iter()
            .find(|block| {
                matches!(
                    block.instructions.first(),
                    Some(ExecutableInstruction::ExpandWord { .. })
                )
            })
            .expect("expansion block");
        let ExecutableTerminator::CompletionSwitch { default, .. } =
            expansion_block.terminator.as_ref().expect("terminator")
        else {
            panic!("an expansion error must dispatch before invocation");
        };
        let failure = function
            .blocks
            .iter()
            .find(|block| block.id == *default)
            .expect("failure block");
        assert!(matches!(
            failure.terminator,
            Some(ExecutableTerminator::ReturnCompletion(_))
        ));
    }

    #[test]
    fn compatibility_builder_carries_complete_registry_facts() {
        let script =
            Script::from_statements(vec![call(vec![literal("incr", 0), literal("counter", 5)])]);
        let function = build_linear_executable_ir(
            &CommandRegistry::build_default(),
            Some(test_context()),
            ExecutableFunctionId::new(7),
            &script,
        )
        .expect("literal registry command");
        let InvocationResolution::Resolved(facts) = &find_invoke(&function).resolution else {
            panic!("literal registry command must resolve");
        };

        assert_eq!(facts.canonical_command, "incr");
        assert_ne!(facts.operation, SemanticOperationId::Invoke);
        assert_eq!(facts.completion, CompletionDescriptor::CONSERVATIVE);
        assert!(facts.effects.requires_world_barrier());
        assert_eq!(facts.return_type, Some(TclType::Int));
        assert_eq!(facts.arg_roles, vec![(0, ArgRole::VarWrite)]);
        assert!(facts.arg_roles_complete);
    }

    #[test]
    fn ordinary_lowered_operations_form_one_executable_sequence() {
        let registry = CommandRegistry::build_default();
        let module = crate::lowering::lower_to_ir(
            "set counter 0\nincr counter\nexpr {$counter + 1}",
            &registry,
        );
        let function = build_linear_executable_ir(
            &registry,
            Some(test_context()),
            ExecutableFunctionId::new(70),
            &module.top_level,
        )
        .expect("already-lowered operations remain executable");
        function.validate().expect("valid executable operation CFG");

        let descriptors: Vec<_> = function
            .blocks
            .iter()
            .flat_map(|block| &block.instructions)
            .filter_map(|instruction| match instruction {
                ExecutableInstruction::ExecuteLowered(operation) => Some(operation.descriptor),
                _ => None,
            })
            .collect();
        assert_eq!(
            descriptors,
            [
                LoweringHookId::Set,
                LoweringHookId::Incr,
                LoweringHookId::Expr
            ]
        );
    }

    #[test]
    fn braced_array_element_assignment_keeps_exact_target_payload() {
        let registry = CommandRegistry::build_default();
        let module = crate::lowering::lower_to_ir("set {totals($key)} 41", &registry);
        let function = build_linear_executable_ir(
            &registry,
            Some(test_context()),
            ExecutableFunctionId::new(71),
            &module.top_level,
        )
        .expect("array-element assignments use the shared set operation");
        let operation = function
            .blocks
            .iter()
            .flat_map(|block| &block.instructions)
            .find_map(|instruction| match instruction {
                ExecutableInstruction::ExecuteLowered(operation) => Some(operation),
                _ => None,
            })
            .expect("lowered assignment");
        assert_eq!(operation.descriptor, LoweringHookId::Set);
        assert!(matches!(
            &operation.statement,
            Statement::AssignConst {
                name,
                name_braced: true,
                value,
                ..
            } if name == "totals($key)" && value == "41"
        ));
    }

    #[test]
    fn structured_region_does_not_decline_following_invocation() {
        let registry = CommandRegistry::build_default();
        let module =
            crate::lowering::lower_to_ir("if {$enabled} {puts enabled}\nputs finished", &registry);
        let function = build_linear_executable_ir(
            &registry,
            Some(test_context()),
            ExecutableFunctionId::new(72),
            &module.top_level,
        )
        .expect("structured control is isolated, not a whole-function decline");
        function.validate().expect("valid structured-region sequence");

        let region = function
            .blocks
            .iter()
            .flat_map(|block| &block.instructions)
            .find_map(|instruction| match instruction {
                ExecutableInstruction::CompleteStructuredRegion(region) => Some(region),
                _ => None,
            })
            .expect("if region");
        assert_eq!(region.descriptor, LoweringHookId::If);
        assert_eq!(region.kind, StructuredRegionKind::Conditional);
        assert_eq!(region.node.path(), &[0]);
        // The body's own invocation is now a node *inside* the region, and the
        // statement after the region keeps its own top-level node.
        let invoked: Vec<_> = function
            .blocks
            .iter()
            .flat_map(|block| &block.instructions)
            .filter_map(|instruction| match instruction {
                ExecutableInstruction::Invoke(invoke) => Some(invoke.node.path().to_vec()),
                _ => None,
            })
            .collect();
        assert!(invoked.iter().any(|path| path.first() == Some(&0) && path.len() > 1));
        assert!(invoked.contains(&vec![1]));
    }

    #[test]
    fn corpus_accumulator_keeps_facts_around_foreach_region() {
        let registry = CommandRegistry::build_default();
        let module = crate::lowering::lower_to_ir(
            "set total 0\nforeach item $items {incr total $item}\nreturn $total",
            &registry,
        );
        let function = build_linear_executable_ir(
            &registry,
            Some(test_context()),
            ExecutableFunctionId::new(73),
            &module.top_level,
        )
        .expect("ordinary real-job loop shape remains executable");
        function.validate().expect("valid accumulator CFG");

        let instructions: Vec<_> = function
            .blocks
            .iter()
            .flat_map(|block| &block.instructions)
            .collect();
        assert!(instructions.iter().any(|instruction| matches!(
            instruction,
            ExecutableInstruction::ExecuteLowered(LoweredOperation {
                descriptor: LoweringHookId::Set,
                ..
            })
        )));
        assert!(instructions.iter().any(|instruction| matches!(
            instruction,
            ExecutableInstruction::CompleteStructuredRegion(StructuredRegion {
                descriptor: LoweringHookId::Foreach,
                kind: StructuredRegionKind::CursorLoop,
                ..
            })
        )));
        assert!(instructions.iter().any(|instruction| matches!(
            instruction,
            ExecutableInstruction::ExecuteLowered(LoweredOperation {
                descriptor: LoweringHookId::Return,
                ..
            })
        )));
    }

    #[test]
    fn compatibility_builder_preserves_dynamic_subcommand_outcomes() {
        let subcommand = WordExpr::Variable {
            spelling: "$operation".to_owned(),
            source: source(7),
        };
        let script = Script::from_statements(vec![call(vec![
            literal("string", 0),
            subcommand.clone(),
            literal("text", 18),
        ])]);
        let function = build_linear_executable_ir(
            &CommandRegistry::build_default(),
            Some(test_context()),
            ExecutableFunctionId::new(8),
            &script,
        )
        .expect("a dynamic subcommand leaves a generic registry invocation");
        let InvocationResolution::Resolved(facts) = &find_invoke(&function).resolution else {
            panic!("the literal command head must resolve");
        };

        assert!(matches!(
            &facts.subcommand,
            OwnedSubcommandResolution::Indeterminate {
                word_kind: InvocationWordKind::Dynamic
            }
        ));
        assert_eq!(facts.operation, SemanticOperationId::Invoke);
        assert_eq!(find_invoke(&function).original_words[1], subcommand);
    }

    #[test]
    fn compatibility_builder_preserves_ambiguous_subcommand_outcomes() {
        let script = Script::from_statements(vec![call(vec![
            literal("string", 0),
            literal("i", 7),
            literal("text", 9),
        ])]);
        let function = build_linear_executable_ir(
            &CommandRegistry::build_default(),
            Some(test_context()),
            ExecutableFunctionId::new(9),
            &script,
        )
        .expect("an ambiguous subcommand leaves a generic registry invocation");
        let InvocationResolution::Resolved(facts) = &find_invoke(&function).resolution else {
            panic!("the literal command head must resolve");
        };

        assert!(matches!(
            &facts.subcommand,
            OwnedSubcommandResolution::Ambiguous { spelling } if spelling == "i"
        ));
        assert_eq!(facts.operation, SemanticOperationId::Invoke);
    }

    #[test]
    fn compatibility_builder_retains_expansion_for_registry_resolution() {
        let expanded = WordExpr::Expand {
            source: source(5),
            word: Box::new(WordExpr::Variable {
                spelling: "$arguments".to_owned(),
                source: source(8),
            }),
        };
        let script =
            Script::from_statements(vec![call(vec![literal("incr", 0), expanded.clone()])]);
        let function = build_linear_executable_ir(
            &CommandRegistry::build_default(),
            Some(test_context()),
            ExecutableFunctionId::new(10),
            &script,
        )
        .expect("expanded arguments remain a generic invocation");
        let InvocationResolution::Resolved(facts) = &find_invoke(&function).resolution else {
            panic!("the literal command head must resolve");
        };

        assert_eq!(facts.form, None);
        assert_eq!(find_invoke(&function).original_words[1], expanded);
        let entries = function
            .blocks
            .iter()
            .flat_map(|block| &block.instructions)
            .find_map(|instruction| match instruction {
                ExecutableInstruction::BuildArgv { entries, .. } => Some(entries),
                _ => None,
            })
            .expect("argv build");
        assert!(matches!(entries[1], ArgvEntry::Expanded(_)));
    }

    #[test]
    fn compatibility_builder_evaluates_each_source_word_once() {
        let repeated = WordExpr::Variable {
            spelling: "$message".to_owned(),
            source: source(5),
        };
        let repeated_again = WordExpr::Variable {
            spelling: "$message".to_owned(),
            source: source(15),
        };
        let script = Script::from_statements(vec![call(vec![
            literal("puts", 0),
            repeated,
            repeated_again,
        ])]);
        let function = build_linear_executable_ir(
            &CommandRegistry::build_default(),
            Some(test_context()),
            ExecutableFunctionId::new(11),
            &script,
        )
        .expect("simple generic invocation");
        let evaluated: Vec<_> = function
            .blocks
            .iter()
            .flat_map(|block| &block.instructions)
            .filter_map(|instruction| match instruction {
                ExecutableInstruction::EvaluateWord { value, .. } => Some(*value),
                _ => None,
            })
            .collect();
        let entries = function
            .blocks
            .iter()
            .flat_map(|block| &block.instructions)
            .find_map(|instruction| match instruction {
                ExecutableInstruction::BuildArgv { entries, .. } => Some(entries),
                _ => None,
            })
            .expect("argv build");
        let entry_values: Vec<_> = entries.iter().copied().map(ArgvEntry::value).collect();

        assert_eq!(evaluated.len(), 3);
        assert_eq!(entry_values, evaluated);
        assert_eq!(
            entry_values.iter().copied().collect::<BTreeSet<_>>().len(),
            entry_values.len()
        );
    }

    #[test]
    fn completion_switch_accepts_a_custom_completion_code() {
        let function_id = ExecutableFunctionId::new(3);
        let block0 = ExecutableBlockId::new(function_id, 0);
        let block1 = ExecutableBlockId::new(function_id, 1);
        let block2 = ExecutableBlockId::new(function_id, 2);
        let block3 = ExecutableBlockId::new(function_id, 3);
        let block4 = ExecutableBlockId::new(function_id, 4);
        let block5 = ExecutableBlockId::new(function_id, 5);
        let word_completion = CompletionId::new(function_id, 0);
        let argv_completion = CompletionId::new(function_id, 1);
        let completion = CompletionId::new(function_id, 2);
        let argv = ExecutableArgvId::new(function_id, 0);
        let value = ExecutableValueId::new(function_id, 0);
        let site = source(0);
        let invocation = GenericInvoke {
            completion,
            argv,
            resolution: InvocationResolution::Unresolved(
                OwnedInvocationResolutionUnresolved::UnknownLiteralHead {
                    spelling: "fixture".to_owned(),
                },
            ),
            original_words: vec![literal("fixture", 0)],
            node: NodeId::from_path(vec![0]),
            source: site.clone(),
        };
        let function = ExecutableFunction::new(
            function_id,
            block0,
            vec![
                ExecutableBlock {
                    id: block0,
                    instructions: vec![ExecutableInstruction::EvaluateWord {
                        value,
                        completion: word_completion,
                        word: literal("fixture", 0),
                        node: NodeId::from_path(vec![0]),
                        source: site,
                    }],
                    terminator: Some(ExecutableTerminator::CompletionSwitch {
                        completion: word_completion,
                        cases: vec![CompletionCase {
                            code: CompletionCode::Ok,
                            target: block1,
                        }],
                        default: block4,
                    }),
                },
                ExecutableBlock {
                    id: block1,
                    instructions: vec![ExecutableInstruction::BuildArgv {
                        argv,
                        completion: argv_completion,
                        entries: vec![ArgvEntry::Value(value)],
                    }],
                    terminator: Some(ExecutableTerminator::CompletionSwitch {
                        completion: argv_completion,
                        cases: vec![CompletionCase {
                            code: CompletionCode::Ok,
                            target: block2,
                        }],
                        default: block5,
                    }),
                },
                ExecutableBlock {
                    id: block2,
                    instructions: vec![ExecutableInstruction::Invoke(invocation)],
                    terminator: Some(ExecutableTerminator::CompletionSwitch {
                        completion,
                        cases: vec![CompletionCase {
                            code: CompletionCode::Other(91),
                            target: block3,
                        }],
                        default: block3,
                    }),
                },
                ExecutableBlock {
                    id: block3,
                    instructions: Vec::new(),
                    terminator: Some(ExecutableTerminator::ReturnCompletion(completion)),
                },
                ExecutableBlock {
                    id: block4,
                    instructions: Vec::new(),
                    terminator: Some(ExecutableTerminator::ReturnCompletion(word_completion)),
                },
                ExecutableBlock {
                    id: block5,
                    instructions: Vec::new(),
                    terminator: Some(ExecutableTerminator::ReturnCompletion(argv_completion)),
                },
            ],
        );
        function
            .validate()
            .expect("custom Tcl codes are valid switch arms");
    }

    #[test]
    fn value_availability_uses_cfg_dominance_not_block_vector_order() {
        let function_id = ExecutableFunctionId::new(6);
        let block0 = ExecutableBlockId::new(function_id, 0);
        let block1 = ExecutableBlockId::new(function_id, 1);
        let block2 = ExecutableBlockId::new(function_id, 2);
        let block3 = ExecutableBlockId::new(function_id, 3);
        let value = ExecutableValueId::new(function_id, 0);
        let word_completion = CompletionId::new(function_id, 0);
        let argv = ExecutableArgvId::new(function_id, 0);
        let argv_completion = CompletionId::new(function_id, 1);
        let node = NodeId::from_path(vec![0]);
        let function = ExecutableFunction::new(
            function_id,
            block0,
            vec![
                ExecutableBlock {
                    id: block0,
                    instructions: Vec::new(),
                    terminator: Some(ExecutableTerminator::Goto(block2)),
                },
                ExecutableBlock {
                    id: block1,
                    instructions: vec![ExecutableInstruction::BuildArgv {
                        argv,
                        completion: argv_completion,
                        entries: vec![ArgvEntry::Value(value)],
                    }],
                    terminator: Some(ExecutableTerminator::ReturnCompletion(argv_completion)),
                },
                ExecutableBlock {
                    id: block2,
                    instructions: vec![ExecutableInstruction::EvaluateWord {
                        value,
                        completion: word_completion,
                        word: literal("fixture", 0),
                        node,
                        source: source(0),
                    }],
                    terminator: Some(ExecutableTerminator::CompletionSwitch {
                        completion: word_completion,
                        cases: vec![CompletionCase {
                            code: CompletionCode::Ok,
                            target: block1,
                        }],
                        default: block3,
                    }),
                },
                ExecutableBlock {
                    id: block3,
                    instructions: Vec::new(),
                    terminator: Some(ExecutableTerminator::ReturnCompletion(word_completion)),
                },
            ],
        );
        function
            .validate()
            .expect("the definition dominates its use despite vector order");
    }

    #[test]
    fn generic_invoke_retains_original_words_as_non_executable_provenance() {
        let variable = WordExpr::Variable {
            spelling: "$message".to_owned(),
            source: source(5),
        };
        let script =
            Script::from_statements(vec![call(vec![literal("puts", 0), variable.clone()])]);
        let function = build_linear_executable_ir(
            &CommandRegistry::build_default(),
            Some(test_context()),
            ExecutableFunctionId::new(4),
            &script,
        )
        .expect("simple call");
        let invoke = find_invoke(&function);
        assert_eq!(invoke.original_words, vec![literal("puts", 0), variable]);
        assert!(matches!(
            &invoke.resolution,
            InvocationResolution::Resolved(_)
        ));
    }

    #[test]
    fn compatibility_builder_retains_dynamic_command_heads_as_unresolved() {
        let head = WordExpr::Variable {
            spelling: "$command".to_owned(),
            source: source(0),
        };
        let script = Script::from_statements(vec![call(vec![head.clone(), literal("x", 9)])]);
        let function = build_linear_executable_ir(
            &CommandRegistry::build_default(),
            Some(test_context()),
            ExecutableFunctionId::new(5),
            &script,
        )
        .expect("a computed head remains a generic invocation");
        let invoke = find_invoke(&function);
        assert!(matches!(
            &invoke.resolution,
            InvocationResolution::Unresolved(OwnedInvocationResolutionUnresolved::ComputedHead {
                word_kind: InvocationWordKind::Dynamic,
            })
        ));
        assert_eq!(invoke.original_words, vec![head, literal("x", 9)]);
    }
}
