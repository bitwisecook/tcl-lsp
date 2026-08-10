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
use tcl_registry::dialects::DialectSet;
use tcl_registry::hooks::LoweringHookId;
use tcl_registry::{CommandRegistry, SemanticOperationId};

use crate::ir::{NodeId, Script, SourceSite, Statement, WordExpr};
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
        (ArgvEntry::Value(_), _)
        | (
            ArgvEntry::Expanded(_),
            ExecutableInstruction::EvaluateWord { .. }
            | ExecutableInstruction::BuildArgv { .. }
            | ExecutableInstruction::Invoke(_)
            | ExecutableInstruction::ExecuteLowered(_)
            | ExecutableInstruction::ExecuteOpaqueRegion(_),
        ) => None,
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
}

impl ValueDefinition {
    const fn position(self) -> InstructionPosition {
        match self {
            Self::Word { position, .. } | Self::Expansion { position, .. } => position,
        }
    }

    const fn completion(self) -> CompletionId {
        match self {
            Self::Word { completion, .. } | Self::Expansion { completion, .. } => completion,
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
    }
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
        | ExecutableInstruction::ExecuteOpaqueRegion(_) => Ok(()),
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

/// Reachability and dominators for the small executable CFG.
///
/// Definition availability must use graph structure, not the incidental order
/// in the `blocks` vector: deterministic IDs are for reproducibility, not an
/// execution schedule.
struct Dominance {
    reachable: BTreeSet<usize>,
    dominators: Vec<BTreeSet<usize>>,
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
        let mut reachable = BTreeSet::new();
        let mut pending = vec![entry];
        while let Some(block) = pending.pop() {
            if !reachable.insert(block) {
                continue;
            }
            pending.extend(successors[block].iter().copied());
        }

        let mut dominators = vec![BTreeSet::new(); count];
        for block in &reachable {
            dominators[*block] = if *block == entry {
                BTreeSet::from([entry])
            } else {
                reachable.clone()
            };
        }
        let mut changed = true;
        while changed {
            changed = false;
            for block in reachable.iter().copied().filter(|block| *block != entry) {
                let mut incoming = predecessors[block]
                    .iter()
                    .copied()
                    .filter(|predecessor| reachable.contains(predecessor));
                let Some(first) = incoming.next() else {
                    continue;
                };
                let mut next = dominators[first].clone();
                for predecessor in incoming {
                    next = next
                        .intersection(&dominators[predecessor])
                        .copied()
                        .collect();
                }
                next.insert(block);
                if next != dominators[block] {
                    dominators[block] = next;
                    changed = true;
                }
            }
        }
        Self {
            reachable,
            dominators,
        }
    }

    fn dominates(&self, definition: usize, use_block: usize) -> bool {
        if !self.reachable.contains(&use_block) {
            return definition == use_block;
        }
        self.dominators[use_block].contains(&definition)
    }
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
/// statements become registry-identified structural operations without
/// reconstructing a command head. Structured control that this IR cannot yet
/// spell exactly becomes one typed opaque region, so it does not discard facts
/// for the rest of the function. No optimisation or backend specialisation is
/// enabled by this adapter.
pub fn build_linear_executable_ir(
    registry: &CommandRegistry,
    dialect: DialectSet,
    function: ExecutableFunctionId,
    script: &Script,
) -> Result<ExecutableFunction, SourceCompatibilityDecline> {
    if script.statements.is_empty() {
        return Err(SourceCompatibilityDecline::EmptyScript);
    }
    let mut allocator = IdAllocator::new(function);
    let stages = plan_linear_stages(registry, dialect, script, &mut allocator)?;
    Ok(build_staged_function(function, &mut allocator, stages))
}

fn plan_linear_stages(
    registry: &CommandRegistry,
    dialect: DialectSet,
    script: &Script,
    allocator: &mut IdAllocator,
) -> Result<Vec<Stage>, SourceCompatibilityDecline> {
    let mut stages = Vec::new();
    for (statement_index, statement) in script.statements.iter().enumerate() {
        let terminates = plan_source_statement(
            registry,
            dialect,
            statement,
            statement_index,
            allocator,
            &mut stages,
        )?;
        if terminates {
            break;
        }
    }
    Ok(stages)
}

fn plan_source_statement(
    registry: &CommandRegistry,
    dialect: DialectSet,
    statement: &Statement,
    statement_index: usize,
    allocator: &mut IdAllocator,
    stages: &mut Vec<Stage>,
) -> Result<bool, SourceCompatibilityDecline> {
    let source = SourceSite::source(statement.span());
    let node = NodeId::from_path(vec![u32::try_from(statement_index).unwrap_or(u32::MAX)]);
    if let Some(descriptor) = lowered_operation_descriptor(statement) {
        stages.push(Stage::ExecuteLowered(LoweredOperation {
            completion: allocator.completion(),
            descriptor,
            statement: statement.clone(),
            node,
            source,
        }));
        return Ok(matches!(statement, Statement::Return { .. }));
    }
    if let Some(descriptor) = opaque_region_descriptor(statement) {
        stages.push(Stage::ExecuteOpaqueRegion(OpaqueRegion {
            completion: allocator.completion(),
            descriptor: descriptor.lowering_hook(),
            statement: statement.clone(),
            node,
            source,
        }));
        return Ok(false);
    }
    let source_call = source_call(statement, statement_index)?;
    let words = exact_words(
        source_call.tokens,
        statement_index,
        source_call.command,
        source_call.args,
    )?;
    let resolution = resolve_invocation_facts(registry, dialect, &words, statement_index)?;
    let entries = plan_argv_entries(&words, &node, allocator, stages);
    let argv = allocator.argv();
    stages.push(Stage::BuildArgv {
        argv,
        completion: allocator.completion(),
        entries,
    });
    stages.push(Stage::Invoke {
        argv,
        completion: allocator.completion(),
        resolution,
        original_words: words,
        node,
        source,
    });
    Ok(false)
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

fn build_staged_function(
    function: ExecutableFunctionId,
    allocator: &mut IdAllocator,
    stages: Vec<Stage>,
) -> ExecutableFunction {
    let stage_count = stages.len();
    let stage_completions: Vec<_> = stages.iter().map(Stage::completion).collect();
    let stage_blocks: Vec<_> = (0..stage_count).map(|_| allocator.block()).collect();
    let failure_blocks: Vec<_> = (0..stage_count.saturating_sub(1))
        .map(|_| allocator.block())
        .collect();
    let mut blocks = Vec::with_capacity(stage_count + failure_blocks.len());
    for (index, stage) in stages.into_iter().enumerate() {
        let is_last = index + 1 == stage_count;
        let terminator = if is_last {
            ExecutableTerminator::ReturnCompletion(stage.completion())
        } else {
            ExecutableTerminator::CompletionSwitch {
                completion: stage.completion(),
                cases: vec![CompletionCase {
                    code: CompletionCode::Ok,
                    target: stage_blocks[index + 1],
                }],
                default: failure_blocks[index],
            }
        };
        blocks.push(ExecutableBlock {
            id: stage_blocks[index],
            instructions: stage.into_instructions(),
            terminator: Some(terminator),
        });
    }
    for (index, failure) in failure_blocks.into_iter().enumerate() {
        blocks.push(ExecutableBlock {
            id: failure,
            instructions: Vec::new(),
            terminator: Some(ExecutableTerminator::ReturnCompletion(
                stage_completions[index],
            )),
        });
    }
    let executable = ExecutableFunction::new(function, stage_blocks[0], blocks);
    debug_assert!(
        executable.validate().is_ok(),
        "compatibility builder emitted invalid executable IR: {:?}",
        executable.validate()
    );
    executable
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
    dialect: DialectSet,
    words: &[WordExpr],
    statement_index: usize,
) -> Result<InvocationResolution, SourceCompatibilityDecline> {
    match resolve_word_exprs(registry, dialect, words) {
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
    ExecuteLowered(LoweredOperation),
    ExecuteOpaqueRegion(OpaqueRegion),
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
            Self::ExecuteLowered(operation) => operation.completion,
            Self::ExecuteOpaqueRegion(region) => region.completion,
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
            Self::ExecuteLowered(operation) => {
                vec![ExecutableInstruction::ExecuteLowered(operation)]
            }
            Self::ExecuteOpaqueRegion(region) => {
                vec![ExecutableInstruction::ExecuteOpaqueRegion(region)]
            }
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

    #[test]
    fn substitutions_complete_before_generic_invocation() {
        let script = Script::from_statements(vec![call(vec![
            literal("puts", 0),
            substitution("[make-message]", 5),
        ])]);
        let function = build_linear_executable_ir(
            &CommandRegistry::build_default(),
            DialectSet::TCL86,
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
            DialectSet::TCL86,
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
            DialectSet::TCL86,
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
            DialectSet::TCL86,
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
            DialectSet::TCL86,
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
            DialectSet::TCL86,
            ExecutableFunctionId::new(72),
            &module.top_level,
        )
        .expect("structured control is isolated, not a whole-function decline");
        function.validate().expect("valid opaque-region sequence");

        let region = function
            .blocks
            .iter()
            .flat_map(|block| &block.instructions)
            .find_map(|instruction| match instruction {
                ExecutableInstruction::ExecuteOpaqueRegion(region) => Some(region),
                _ => None,
            })
            .expect("if region");
        assert_eq!(region.descriptor, Some(LoweringHookId::If));
        assert_eq!(region.node.path(), &[0]);
        assert_eq!(find_invoke(&function).node.path(), &[1]);
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
            DialectSet::TCL86,
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
            ExecutableInstruction::ExecuteOpaqueRegion(OpaqueRegion {
                descriptor: Some(LoweringHookId::Foreach),
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
            DialectSet::TCL86,
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
            DialectSet::TCL86,
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
            DialectSet::TCL86,
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
            DialectSet::TCL86,
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
            DialectSet::TCL86,
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
            DialectSet::TCL86,
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
