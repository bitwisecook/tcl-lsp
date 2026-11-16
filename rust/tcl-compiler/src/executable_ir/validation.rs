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

//! The bounded executable-IR invariants: block layout, definitions, owners, uses and terminators, and the error they report.

use super::dominance::Dominance;
use super::footprints::lowered_operation_footprint;
use super::region_descriptors::{
    OpaqueRegionDescriptor, lowered_operation_descriptor, opaque_region_descriptor,
    structured_region_projection,
};
use super::{
    ArgvEntry, CompletionId, ExecutableArgvId, ExecutableBlockId, ExecutableExpr,
    ExecutableFunction, ExecutableFunctionId, ExecutableInstruction, ExecutableTerminator,
    ExecutableValueId, GenericInvoke, LoweredOperation, OpaqueRegion, StructuredRegion,
};
use crate::ir::WordExpr;
use std::collections::{BTreeMap, BTreeSet};
use tcl_core_types::Code as CompletionCode;

impl ExecutableFunction {
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
        (ArgvEntry::Value(_) | ArgvEntry::Expanded(_), _) => None,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(super) struct InstructionPosition {
    pub(super) block: usize,
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
    pub(super) const fn position(self) -> InstructionPosition {
        match self {
            Self::Word { position, .. }
            | Self::Expansion { position, .. }
            | Self::Computed { position, .. } => position,
        }
    }

    pub(super) const fn completion(self) -> CompletionId {
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
    pub(super) completion: CompletionId,
    pub(super) entries: Vec<ArgvEntry>,
}

#[derive(Default)]
struct DefinitionTables {
    values: BTreeMap<ExecutableValueId, ValueDefinition>,
    argvs: BTreeMap<ExecutableArgvId, ArgvDefinition>,
    completions: BTreeMap<CompletionId, InstructionPosition>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct DefinedWord {
    pub(super) word: WordExpr,
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

#[allow(clippy::too_many_lines)]
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
        ExecutableInstruction::CompleteEvaluatedRegion(region) => insert_completion_definition(
            &mut definitions.completions,
            region.invocation.completion,
            position,
        ),
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
    if structured_region_projection(&region.statement) != Some((region.descriptor, region.kind)) {
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
    if operation.footprint
        != lowered_operation_footprint(&operation.statement, operation.original_tokens.as_deref())
    {
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
        // These consume no executable value, so they have no use to check.
        ExecutableInstruction::EvaluateWord { .. }
        | ExecutableInstruction::ExecuteLowered(_)
        | ExecutableInstruction::ExecuteOpaqueRegion(_)
        | ExecutableInstruction::EvaluateExpr { .. }
        | ExecutableInstruction::JoinCompletion { .. }
        | ExecutableInstruction::WriteCompletionCell { .. }
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
        ExecutableInstruction::CompleteEvaluatedRegion(region) => validate_invoke_use(
            function,
            &region.invocation,
            position,
            values,
            argvs,
            dominance,
        ),
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
        ExecutableInstruction::CompleteEvaluatedRegion(region) => {
            require_completion_owner(region.invocation.completion, function)?;
            require_argv_owner(region.invocation.argv, function)?;
            if region.entry.function() != function {
                return Err(ExecutableIrValidationError::ForeignBlockId(region.entry));
            }
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
        ExecutableTerminator::RegionChoice { enter, skip } => {
            require_target(*enter, function.id, blocks)?;
            require_target(*skip, function.id, blocks)
        }
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
        ExecutableTerminator::RegionChoice { enter, skip } => {
            require_target(*enter, function, blocks)?;
            require_target(*skip, function, blocks)
        }
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
