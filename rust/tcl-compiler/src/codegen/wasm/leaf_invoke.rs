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

//! Per-statement prebuilt-argv planning for the general structured tier.
//!
//! The general tier's normal path for a leaf `Statement::Call` is to evaluate
//! the command's words itself and hand the runtime a complete argv through
//! `tcl_invoke_argv`. This module owns the *planning* half of that: it decides,
//! from the structured [`WordExpr`] words the lowering retained, whether every
//! word has a compiled evaluation, and lays out the single transient call frame
//! the emitted code allocates. It never serialises an instruction.
//!
//! Planning is deliberately total: it either returns a complete
//! [`WasmLeafInvokePlan`] or one typed [`WasmLeafInvokeDecline`], so the emitter
//! can never begin emitting a statement it cannot finish. Selection runs
//! through [`crate::backend_registry::BackendRegistry`] keyed by the registry's
//! [`SemanticOperationId`], so a command never reaches the emitter by name.
//!
//! # Frame layout
//!
//! One statement allocates one frame:
//!
//! ```text
//! [ object slots  : N * 4 bytes  ]   every owned value the statement creates
//! [ completions   : M * 12 bytes ]   one TclCompletionAbi per invocation
//! ```
//!
//! Every object slot is nulled before the statement's first evaluation and
//! released (null-safely) on the single cleanup path, so an abrupt completion
//! part-way through word evaluation leaks nothing. An invocation's argv is the
//! contiguous run of object slots holding its words, so `tcl_invoke_argv`
//! receives a pointer directly into the frame.

use tcl_lexer::Span;
use tcl_registry::SemanticOperationId;
use tcl_runtime_api::codegen_abi::{
    WASM32_COMPLETION_ALIGN, WASM32_COMPLETION_SIZE, WASM32_POINTER_BYTES,
};

use crate::backend_registry::{
    BackendDeclineReason, BackendPlanKind, BackendRegistry, BackendSelection, BackendSelector,
    SelectionFacts, SelectionInput, SelectionRegion, SelectorAttemptFailure, SelectorDecision,
    SelectorPriority, SelectorRequest,
};
use crate::codegen::values::whole_var_reference;
use crate::ir::{CommandTokens, Provenance, SourceSite, WordExpr, WordPart};
use tcl_dialect::model::Family;
use crate::target_contract::{
    LegalisationRequirements, TargetCapabilities, TargetContract, TargetFamily,
};

/// Recursion cap for nested command substitution and compound-word parts.
///
/// Word nesting is bounded by source nesting, and the planner recurses once per
/// level. The cap keeps a pathological `[a [b [c …]]]` off the native stack,
/// matching the other recursive-descent walkers in this crate.
const MAX_WORD_DEPTH: u32 = 32;

/// A complete, immutable plan for compiling one leaf command invocation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct WasmLeafInvokePlan {
    /// The statement's own invocation.
    pub(super) root: WasmInvokeNode,
    /// Number of owned-object slots the frame reserves.
    pub(super) object_slots: usize,
    /// Byte offset of the first completion record inside the frame.
    pub(super) completion_base: i32,
    /// Total transient frame size in bytes.
    pub(super) frame_bytes: i32,
    /// Frame alignment in bytes.
    pub(super) frame_align: i32,
}

/// One compiled invocation: its argv words, its completion storage, and the
/// object slots that adopt the completion's owned result and options.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct WasmInvokeNode {
    /// First object slot of this invocation's contiguous argv run.
    pub(super) argv_slot: usize,
    /// Compiled evaluation for each argv word, in evaluation order.
    pub(super) words: Vec<WasmWordPlan>,
    /// Index of this invocation's completion record.
    pub(super) completion: usize,
    /// Object slot adopting the completion's owned result.
    pub(super) result_slot: usize,
    /// Object slot adopting the completion's owned return options.
    pub(super) options_slot: usize,
}

/// Compiled evaluation for one Tcl word (or one part of a compound word).
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum WasmWordPlan {
    /// A bare or braced literal, materialised straight from the constant pool.
    Literal {
        /// Object slot receiving the value.
        slot: usize,
        /// The word's exact Tcl value.
        text: String,
    },
    /// A scalar variable read under its exact Tcl name.
    Scalar {
        /// Object slot receiving the value.
        slot: usize,
        /// The variable's exact Tcl name.
        name: String,
    },
    /// An array element read with a statically known key.
    Element {
        /// Object slot receiving the value.
        slot: usize,
        /// The array's Tcl name.
        name: String,
        /// The element key.
        key: String,
    },
    /// A `[…]` command substitution, evaluated as a nested invocation whose
    /// completion result becomes the word's value.
    Invoke(Box<WasmInvokeNode>),
    /// A compound word whose evaluated parts are joined left to right.
    Concat {
        /// Object slot receiving the joined value.
        slot: usize,
        /// First object slot of the contiguous part run.
        parts_slot: usize,
        /// Compiled evaluation for each part, in evaluation order.
        parts: Vec<WasmWordPlan>,
    },
}

/// A precise reason one leaf invocation cannot be compiled to a prebuilt argv.
///
/// Every variant keeps the statement on the source-span `tcl_eval_code`
/// fallback; none of them is an error.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum WasmLeafInvokeDecline {
    /// Lowering retained no structured word snapshot for the statement.
    MissingCommandTokens,
    /// Two statements share one source span, so neither can be told apart by
    /// the span-keyed plan table. Both fall back rather than risk one
    /// statement executing the other's plan.
    DuplicateStatementSpan,
    /// The word sequence has no command head.
    MissingCommandHead,
    /// `{*}` expansion needs runtime Tcl list processing.
    ArgumentExpansion,
    /// A word's substitution behaviour is not modelled by the word snapshot.
    OpaqueWord,
    /// The word contains backslash substitution the emitter does not perform.
    BackslashSubstitution,
    /// The variable reference computes its own name.
    DynamicVariableName,
    /// The variable spelling cannot be told apart from another Tcl meaning.
    AmbiguousVariableSpelling,
    /// The command substitution is not one complete command.
    UnmodelledCommandSubstitution,
    /// Word nesting exceeded the planner's recursion cap.
    WordNestingTooDeep,
    /// The transient call frame does not fit wasm32 addressing.
    FrameTooLarge,
}

impl WasmLeafInvokeDecline {
    /// Stable code-generation evidence spelling.
    #[must_use]
    pub(super) const fn as_str(self) -> &'static str {
        match self {
            Self::MissingCommandTokens => "missing-command-tokens",
            Self::DuplicateStatementSpan => "duplicate-statement-span",
            Self::MissingCommandHead => "missing-command-head",
            Self::ArgumentExpansion => "argument-expansion",
            Self::OpaqueWord => "opaque-word",
            Self::BackslashSubstitution => "backslash-substitution",
            Self::DynamicVariableName => "dynamic-variable-name",
            Self::AmbiguousVariableSpelling => "ambiguous-variable-spelling",
            Self::UnmodelledCommandSubstitution => "unmodelled-command-substitution",
            Self::WordNestingTooDeep => "word-nesting-too-deep",
            Self::FrameTooLarge => "frame-too-large",
        }
    }
}

/// Immutable backend selection context: the statement's structured words.
#[derive(Debug, Clone, Copy)]
struct LeafSelectionContext<'a> {
    words: &'a [WordExpr],
}

/// The prebuilt-argv selector for one leaf command invocation.
#[derive(Debug, Clone, Copy)]
struct LeafInvokeSelector;

impl<'a> BackendSelector<WasmLeafInvokePlan, LeafSelectionContext<'a>, WasmLeafInvokeDecline>
    for LeafInvokeSelector
{
    fn select(
        &self,
        request: &SelectorRequest<'_, LeafSelectionContext<'a>>,
    ) -> SelectorDecision<WasmLeafInvokePlan, WasmLeafInvokeDecline> {
        match plan_leaf_invocation(request.context().words) {
            Ok(plan) => SelectorDecision::Selected(plan),
            Err(decline) => SelectorDecision::Declined(decline),
        }
    }
}

/// Select the prebuilt-argv plan for one leaf invocation through the shared
/// backend registry.
///
/// `operation` is the registry-resolved semantic operation for the command, so
/// the selection ladder — not a command name in the emitter — decides which
/// plan kind may apply. A registered generic-invocation selector is reachable
/// for every operation through the registry's generic fallback, so a command
/// the registry could not resolve still selects here under
/// [`SemanticOperationId::Invoke`].
pub(super) fn select_leaf_invocation(
    words: &[WordExpr],
    operation: SemanticOperationId,
    facts: SelectionFacts<'_>,
) -> Result<WasmLeafInvokePlan, BackendDeclineReason<WasmLeafInvokeDecline>> {
    let mut registry: BackendRegistry<
        WasmLeafInvokePlan,
        LeafSelectionContext<'_>,
        WasmLeafInvokeDecline,
    > = BackendRegistry::new(TargetContract::new(
        TargetFamily::Wasm,
        TargetCapabilities::wasm(),
    ));
    if registry
        .register(
            SemanticOperationId::Invoke,
            BackendPlanKind::GenericInvoke,
            SelectorPriority::DEFAULT,
            LegalisationRequirements::new(),
            LeafInvokeSelector,
        )
        .is_err()
    {
        return Err(BackendDeclineReason::MissingOperation(operation));
    }
    let context = LeafSelectionContext { words };
    match registry.select_with_context(
        SelectionInput::with_facts(operation, SelectionRegion::PrebuiltArgvInvocation, facts),
        &context,
    ) {
        BackendSelection::Selected(selected) => Ok(selected.into_plan()),
        BackendSelection::Declined(decline) => Err(decline),
    }
}

/// The word-level reason inside a backend selection decline, when the ladder
/// reached this backend's selector at all.
pub(super) fn word_decline(
    decline: &BackendDeclineReason<WasmLeafInvokeDecline>,
) -> Option<WasmLeafInvokeDecline> {
    let BackendDeclineReason::NoViablePlan { attempts, .. } = decline else {
        return None;
    };
    attempts.iter().find_map(|attempt| match attempt.failure {
        SelectorAttemptFailure::Selector(reason) => Some(reason),
        _ => None,
    })
}

/// Reserve object slots and completion records for one statement.
#[derive(Debug, Default)]
struct FrameAllocator {
    objects: usize,
    completions: usize,
}

impl FrameAllocator {
    fn object(&mut self) -> usize {
        self.objects(1)
    }

    fn objects(&mut self, count: usize) -> usize {
        let first = self.objects;
        self.objects += count;
        first
    }

    fn completion(&mut self) -> usize {
        let index = self.completions;
        self.completions += 1;
        index
    }
}

/// Plan the compiled evaluation and frame layout for one leaf invocation.
pub(super) fn plan_leaf_invocation(
    words: &[WordExpr],
) -> Result<WasmLeafInvokePlan, WasmLeafInvokeDecline> {
    let mut allocator = FrameAllocator::default();
    let result_slot = allocator.object();
    let root = plan_invoke(words, result_slot, &mut allocator, 0)?;
    let object_bytes = i32::try_from(allocator.objects)
        .ok()
        .and_then(|slots| slots.checked_mul(WASM32_POINTER_BYTES))
        .ok_or(WasmLeafInvokeDecline::FrameTooLarge)?;
    let completion_bytes = i32::try_from(allocator.completions)
        .ok()
        .and_then(|records| records.checked_mul(WASM32_COMPLETION_SIZE))
        .ok_or(WasmLeafInvokeDecline::FrameTooLarge)?;
    let frame_bytes = object_bytes
        .checked_add(completion_bytes)
        .ok_or(WasmLeafInvokeDecline::FrameTooLarge)?;
    Ok(WasmLeafInvokePlan {
        root,
        object_slots: allocator.objects,
        completion_base: object_bytes,
        frame_bytes,
        frame_align: WASM32_COMPLETION_ALIGN,
    })
}

fn plan_invoke(
    words: &[WordExpr],
    result_slot: usize,
    allocator: &mut FrameAllocator,
    depth: u32,
) -> Result<WasmInvokeNode, WasmLeafInvokeDecline> {
    if depth > MAX_WORD_DEPTH {
        return Err(WasmLeafInvokeDecline::WordNestingTooDeep);
    }
    if words.is_empty() {
        return Err(WasmLeafInvokeDecline::MissingCommandHead);
    }
    let argv_slot = allocator.objects(words.len());
    let options_slot = allocator.object();
    let completion = allocator.completion();
    let mut plans = Vec::with_capacity(words.len());
    for (index, word) in words.iter().enumerate() {
        plans.push(plan_word(word, argv_slot + index, allocator, depth)?);
    }
    Ok(WasmInvokeNode {
        argv_slot,
        words: plans,
        completion,
        result_slot,
        options_slot,
    })
}

fn plan_word(
    word: &WordExpr,
    slot: usize,
    allocator: &mut FrameAllocator,
    depth: u32,
) -> Result<WasmWordPlan, WasmLeafInvokeDecline> {
    match word {
        // A bare literal is exactly its own value: the word model only reports
        // this shape for a single unquoted fragment with no backslash.
        WordExpr::Literal { text, .. } => Ok(WasmWordPlan::Literal {
            slot,
            text: text.clone(),
        }),
        WordExpr::BracedLiteral { text, .. } => {
            // Braces suppress substitution, but a backslash inside them still
            // has parser-level meaning this emitter does not reproduce.
            if text.contains('\\') {
                return Err(WasmLeafInvokeDecline::BackslashSubstitution);
            }
            Ok(WasmWordPlan::Literal {
                slot,
                text: text.clone(),
            })
        }
        WordExpr::Variable { spelling, source } => plan_variable(spelling, source, slot),
        WordExpr::CommandSubstitution { spelling, source } => {
            let inner = nested_words(spelling, source)?;
            Ok(WasmWordPlan::Invoke(Box::new(plan_invoke(
                &inner,
                slot,
                allocator,
                depth + 1,
            )?)))
        }
        WordExpr::Template { parts, .. } => plan_template(parts, slot, allocator, depth),
        WordExpr::Expand { .. } => Err(WasmLeafInvokeDecline::ArgumentExpansion),
        WordExpr::Opaque { .. } => Err(WasmLeafInvokeDecline::OpaqueWord),
    }
}

fn plan_template(
    parts: &[WordPart],
    slot: usize,
    allocator: &mut FrameAllocator,
    depth: u32,
) -> Result<WasmWordPlan, WasmLeafInvokeDecline> {
    if parts.is_empty() {
        return Err(WasmLeafInvokeDecline::OpaqueWord);
    }
    if depth > MAX_WORD_DEPTH {
        return Err(WasmLeafInvokeDecline::WordNestingTooDeep);
    }
    let parts_slot = allocator.objects(parts.len());
    let mut plans = Vec::with_capacity(parts.len());
    for (index, part) in parts.iter().enumerate() {
        plans.push(plan_part(part, parts_slot + index, allocator, depth + 1)?);
    }
    Ok(WasmWordPlan::Concat {
        slot,
        parts_slot,
        parts: plans,
    })
}

fn plan_part(
    part: &WordPart,
    slot: usize,
    allocator: &mut FrameAllocator,
    depth: u32,
) -> Result<WasmWordPlan, WasmLeafInvokeDecline> {
    match part {
        WordPart::Text { text, .. } => {
            if text.contains('\\') {
                return Err(WasmLeafInvokeDecline::BackslashSubstitution);
            }
            Ok(WasmWordPlan::Literal {
                slot,
                text: text.clone(),
            })
        }
        WordPart::Variable { spelling, source } => plan_variable(spelling, source, slot),
        WordPart::CommandSubstitution { spelling, source } => {
            let inner = nested_words(spelling, source)?;
            Ok(WasmWordPlan::Invoke(Box::new(plan_invoke(
                &inner,
                slot,
                allocator,
                depth + 1,
            )?)))
        }
        WordPart::Opaque { .. } => Err(WasmLeafInvokeDecline::OpaqueWord),
    }
}

/// Plan a variable read, distinguishing `$a(b)` from `${a(b)}`.
///
/// The compatibility spelling normalises both to `${a(b)}`, so the recorded
/// lexical extent is what tells them apart: a `${…}` reference is exactly two
/// bytes longer than its name, a `$…` reference exactly one. Only the latter
/// is an array-element access; `${a(b)}` names a scalar whose name happens to
/// contain parentheses.
fn plan_variable(
    spelling: &str,
    source: &SourceSite,
    slot: usize,
) -> Result<WasmWordPlan, WasmLeafInvokeDecline> {
    let name = whole_var_reference(spelling).ok_or(WasmLeafInvokeDecline::DynamicVariableName)?;
    if !name.contains('(') {
        return Ok(WasmWordPlan::Scalar {
            slot,
            name: name.to_owned(),
        });
    }
    if source.provenance != Provenance::Source {
        return Err(WasmLeafInvokeDecline::AmbiguousVariableSpelling);
    }
    match extent(source.span).checked_sub(name.len()) {
        // `${name}` — the whole thing is one scalar name.
        Some(2) => Ok(WasmWordPlan::Scalar {
            slot,
            name: name.to_owned(),
        }),
        // `$name(key)` — a genuine array element access.
        Some(1) => {
            let (base, key) =
                split_element(name).ok_or(WasmLeafInvokeDecline::AmbiguousVariableSpelling)?;
            if key.bytes().any(|byte| matches!(byte, b'$' | b'[' | b'\\')) {
                return Err(WasmLeafInvokeDecline::DynamicVariableName);
            }
            Ok(WasmWordPlan::Element {
                slot,
                name: base.to_owned(),
                key: key.to_owned(),
            })
        }
        _ => Err(WasmLeafInvokeDecline::AmbiguousVariableSpelling),
    }
}

fn extent(span: Span) -> usize {
    span.end().saturating_sub(span.start()) as usize
}

/// Split `base(key)` the way the runtime's own array reference split does.
fn split_element(name: &str) -> Option<(&str, &str)> {
    let inner = name.strip_suffix(')')?;
    let open = inner.find('(')?;
    let (base, key) = inner.split_at(open);
    (!base.is_empty()).then(|| (base, &key[1..]))
}

/// The structured words of a `[…]` command substitution.
///
/// The word snapshot keeps a command substitution as one opaque spelling, so
/// its inner words are recovered by running the canonical segmenter over the
/// recorded lexical extent — the same segmentation the outer command's words
/// came from, not a bespoke parser. Anything other than exactly one complete
/// command declines.
fn nested_words(
    spelling: &str,
    source: &SourceSite,
) -> Result<Vec<WordExpr>, WasmLeafInvokeDecline> {
    let inner = spelling
        .strip_prefix('[')
        .and_then(|text| text.strip_suffix(']'))
        .ok_or(WasmLeafInvokeDecline::UnmodelledCommandSubstitution)?;
    if inner.trim().is_empty() {
        return Err(WasmLeafInvokeDecline::UnmodelledCommandSubstitution);
    }
    let base = if source.provenance == Provenance::Source {
        source.span.start().saturating_add(1)
    } else {
        0
    };
    let segments = crate::segmenter::segment_commands_with_offset(inner, base);
    let [segment] = segments.as_slice() else {
        return Err(WasmLeafInvokeDecline::UnmodelledCommandSubstitution);
    };
    if segment.is_partial {
        return Err(WasmLeafInvokeDecline::UnmodelledCommandSubstitution);
    }
    let tokens = CommandTokens::from_segmented(segment);
    if tokens.word_exprs.is_empty() {
        return Err(WasmLeafInvokeDecline::MissingCommandHead);
    }
    Ok(tokens.word_exprs)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::segmenter::segment_commands;

    /// The structured words of the first command in `source`.
    fn words(source: &str) -> Vec<WordExpr> {
        let segments = segment_commands(source);
        CommandTokens::from_segmented(&segments[0]).word_exprs
    }

    fn plan(source: &str) -> Result<WasmLeafInvokePlan, WasmLeafInvokeDecline> {
        plan_leaf_invocation(&words(source))
    }

    #[test]
    fn literal_words_plan_a_flat_argv() {
        let plan = plan("string length hello").expect("literal argv plans");
        assert_eq!(plan.root.words.len(), 3);
        assert!(
            plan.root
                .words
                .iter()
                .all(|word| matches!(word, WasmWordPlan::Literal { .. }))
        );
        // Three argv words, the root result, and the root options.
        assert_eq!(plan.object_slots, 5);
        assert_eq!(plan.completion_base, 5 * WASM32_POINTER_BYTES);
        assert_eq!(
            plan.frame_bytes,
            5 * WASM32_POINTER_BYTES + WASM32_COMPLETION_SIZE
        );
    }

    #[test]
    fn braced_words_keep_their_unsubstituted_value() {
        let plan = plan("puts {[add 2 4]}").expect("braced literal plans");
        assert_eq!(
            plan.root.words[1],
            WasmWordPlan::Literal {
                slot: plan.root.argv_slot + 1,
                text: "[add 2 4]".to_owned(),
            }
        );
    }

    #[test]
    fn scalar_variable_words_plan_a_named_read() {
        let plan = plan("string length $value").expect("scalar read plans");
        assert_eq!(
            plan.root.words[2],
            WasmWordPlan::Scalar {
                slot: plan.root.argv_slot + 2,
                name: "value".to_owned(),
            }
        );
    }

    #[test]
    fn braced_variable_spelling_stays_a_scalar_name() {
        let plan = plan("puts ${value}").expect("braced scalar read plans");
        assert_eq!(
            plan.root.words[1],
            WasmWordPlan::Scalar {
                slot: plan.root.argv_slot + 1,
                name: "value".to_owned(),
            }
        );
    }

    /// `$a(b)` is an element access; `${a(b)}` names a scalar. The
    /// compatibility spelling is identical, so the lexical extent decides.
    #[test]
    fn array_and_brace_spellings_are_told_apart() {
        let element = plan("puts $a(b)").expect("array element read plans");
        assert_eq!(
            element.root.words[1],
            WasmWordPlan::Element {
                slot: element.root.argv_slot + 1,
                name: "a".to_owned(),
                key: "b".to_owned(),
            }
        );
        let scalar = plan("puts ${a(b)}").expect("odd scalar name plans");
        assert_eq!(
            scalar.root.words[1],
            WasmWordPlan::Scalar {
                slot: scalar.root.argv_slot + 1,
                name: "a(b)".to_owned(),
            }
        );
    }

    #[test]
    fn dynamic_array_keys_decline() {
        assert_eq!(
            plan("puts $a($i)"),
            Err(WasmLeafInvokeDecline::DynamicVariableName)
        );
    }

    #[test]
    fn command_substitution_plans_a_nested_invocation() {
        let plan = plan("puts [string length abc]").expect("nested invocation plans");
        let WasmWordPlan::Invoke(nested) = &plan.root.words[1] else {
            panic!("expected a nested invocation, got {:?}", plan.root.words[1]);
        };
        assert_eq!(nested.words.len(), 3);
        // The nested completion's result lands in the outer word's own slot.
        assert_eq!(nested.result_slot, plan.root.argv_slot + 1);
        assert_ne!(nested.completion, plan.root.completion);
    }

    #[test]
    fn quoted_words_plan_a_concatenation() {
        let plan = plan("puts \"hi $name\"").expect("quoted word plans");
        let WasmWordPlan::Concat { parts, .. } = &plan.root.words[1] else {
            panic!("expected a concatenation, got {:?}", plan.root.words[1]);
        };
        assert!(
            parts
                .iter()
                .any(|part| matches!(part, WasmWordPlan::Scalar { name, .. } if name == "name")),
            "{parts:?}"
        );
        assert!(
            parts
                .iter()
                .any(|part| matches!(part, WasmWordPlan::Literal { text, .. } if text == "hi ")),
            "{parts:?}"
        );
    }

    #[test]
    fn expansion_declines() {
        assert_eq!(
            plan("puts {*}$args"),
            Err(WasmLeafInvokeDecline::ArgumentExpansion)
        );
    }

    #[test]
    fn backslash_substitution_declines() {
        assert_eq!(
            plan("puts a\\tb"),
            Err(WasmLeafInvokeDecline::BackslashSubstitution)
        );
    }

    #[test]
    fn every_decline_has_a_stable_spelling() {
        for decline in [
            WasmLeafInvokeDecline::MissingCommandTokens,
            WasmLeafInvokeDecline::MissingCommandHead,
            WasmLeafInvokeDecline::ArgumentExpansion,
            WasmLeafInvokeDecline::OpaqueWord,
            WasmLeafInvokeDecline::BackslashSubstitution,
            WasmLeafInvokeDecline::DynamicVariableName,
            WasmLeafInvokeDecline::AmbiguousVariableSpelling,
            WasmLeafInvokeDecline::UnmodelledCommandSubstitution,
            WasmLeafInvokeDecline::WordNestingTooDeep,
            WasmLeafInvokeDecline::FrameTooLarge,
        ] {
            assert!(!decline.as_str().is_empty());
        }
    }

    #[test]
    fn the_registry_selects_the_generic_invocation_plan() {
        let words = words("string length hello");
        assert!(
            select_leaf_invocation(
                &words,
                SemanticOperationId::Invoke,
                SelectionFacts::unavailable(),
            )
            .is_ok()
        );
    }

    #[test]
    fn the_registry_reports_the_typed_word_decline() {
        let words = words("puts {*}$args");
        let Err(BackendDeclineReason::NoViablePlan { attempts, .. }) = select_leaf_invocation(
            &words,
            SemanticOperationId::Invoke,
            SelectionFacts::unavailable(),
        ) else {
            panic!("expected a typed backend decline");
        };
        assert!(!attempts.is_empty());
    }
}
