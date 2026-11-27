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

//! The original parsed tokens of one command invocation and the proofs carried beside them.

use super::source_bpf_command::SourceBpfCommand;
use super::synthetic_marker::SyntheticMarker;
use tcl_lexer::{LexerConfig, SourceMap, Span, Token};
use tcl_source::segment::SegmentedCommand;
use tcl_source::words::WordExpr;
use tcl_source::words::{Provenance, SourceSite};

/// Original parsed tokens for a command invocation.
///
/// Carried on [`Statement::Call`] and [`Statement::Barrier`] so
/// downstream passes (optimiser, compiler checks) can inspect tokens
/// without re-lexing.
///
/// Token references use [`Span`]s into the
/// source buffer. The `argv_texts` and `single_token_word` fields
/// preserve the per-word metadata needed by analysis passes.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct CommandTokens {
    /// Per-word representative token spans.
    pub argv: Vec<Span>,
    /// Per-word text values.
    pub argv_texts: Vec<String>,
    /// Structured per-word syntax derived once during segmentation/lowering.
    ///
    /// [`Self::argv_texts`] and the representative-token arrays remain the
    /// compatibility view consumed by existing analyser and codegen passes.
    /// The semantic IR uses this ordered representation so it never needs to
    /// re-lex a Tcl word or infer substitution order from a flattened string.
    pub word_exprs: Vec<WordExpr>,
    /// Per-word representative token kind. Preserves the [`tcl_lexer::TokenType`]
    /// of each argv entry so analysis passes can distinguish a
    /// brace-string literal (`Str`) from a bareword (`Esc`), a
    /// variable reference (`Var`), or a command substitution
    /// (`Cmd`).
    pub argv_kinds: Vec<tcl_lexer::TokenType>,
    /// Whether each word consists of a single token.
    pub single_token_word: Vec<bool>,
    /// All tokens in the command (including separators).
    pub all_tokens: Vec<Span>,
    /// `{*}` expansion markers per word, if any word uses expansion.
    pub expand_word: Option<Vec<bool>>,
    /// The [`SyntheticMarker`] this statement *is*, when the CFG builder
    /// synthesised it to carry an effect rather than a command to run.
    ///
    /// `None` for everything that came from a Tcl word — both constructors
    /// hard-code it, so the only way to set it is [`Self::marker`], which no
    /// lowering path calls. That is the whole point: command *names* are
    /// forgeable (see [`SyntheticMarker`]), a private constructor is not.
    pub synthetic: Option<SyntheticMarker>,
    /// Proved immediate caller-frame script semantics, retaining runtime argv.
    pub evaluated_body: Option<Box<crate::execution_region::EvaluatedBodyRegion>>,
    /// Point-specific dispatch alternatives after source argument evaluation.
    ///
    /// `None` is an unqueried compatibility snapshot. An explicitly uncertain
    /// answer must never be replaced with a written-head catalogue lookup.
    pub source_binding: Option<crate::command_binding::SourceInvocationBinding>,
    /// Exact nested dispatch proofs keyed by the nested command head's source offset.
    pub nested_bindings:
        std::sync::Arc<Vec<(u32, crate::command_binding::SourceInvocationBinding)>>,
    /// Bindings at reached reads during original argument evaluation or the
    /// selected native expression. Temporal owners distinguish those phases;
    /// executed script bodies are excluded. Reads can differ within one word.
    /// A missing record supplies no proof of the cell read.
    pub variable_accesses: Vec<crate::command_binding::SourceVariableAccess>,
    /// Actual hosted source/context for conditional taint metadata only.
    /// This grants no handler, store, effects, Normal or native compiler facts.
    pub hosted_taint_context:
        Option<std::sync::Arc<crate::registry_invocation::HostedSourceTaintContext>>,
    /// Closed BPF DSL source or typed generation metadata. This is independent
    /// of Tcl dispatch, Native entry, effects and whole-script coverage.
    pub source_bpf_command: Option<SourceBpfCommand>,
}

impl CommandTokens {
    /// Retained execution grammar for nested source parsing. An unqueried or
    /// dialect-unknown carrier keeps the caller's lexical configuration.
    #[must_use]
    pub fn native_lexer_config(&self, fallback: tcl_lexer::LexerConfig) -> tcl_lexer::LexerConfig {
        self.source_binding
            .as_ref()
            .and_then(|binding| binding.variable_context.invocation_dialect)
            .map_or(fallback, |native| {
                fallback.with_grammar(native.lexer_grammar)
            })
    }

    /// Relocate physical cells in retained source proofs; lexical spans and command allocations remain source-owned.
    pub fn relocate_variable_proofs(
        &mut self,
        relocation: &crate::var_resolve::VariableProofRelocation,
    ) {
        if let Some(binding) = &mut self.source_binding {
            binding.relocate_variable_proofs(relocation);
        }
        for (_, binding) in self.nested_bindings_mut() {
            binding.relocate_variable_proofs(relocation);
        }
        for access in &mut self.variable_accesses {
            *access = access.relocated_variables(relocation);
        }
    }

    /// Refresh actual source ownership while preserving the lowered word layout.
    /// Synthetic iteration inputs contain a selected argv projection, rather
    /// than the original command's entire argument vector. Source restoration
    /// must neither replace that projection nor make it execute again.
    pub fn restore_source_proofs(&mut self, original: &Self) {
        self.hosted_taint_context
            .clone_from(&original.hosted_taint_context);
        self.source_binding.clone_from(&original.source_binding);
        self.nested_bindings.clone_from(&original.nested_bindings);
        self.variable_accesses
            .clone_from(&original.variable_accesses);
    }

    /// Build a token snapshot from the segmenter without losing word shape.
    ///
    /// `sm` is the buffer the segment's spans index into — the document, or a
    /// sub-lexed slice carrying its document base
    /// ([`SourceMap::with_base`]) — and `config` is that document's grammar,
    /// resolved once at the ingress and threaded here so every word is
    /// decomposed under the release it is written for.
    #[must_use]
    pub fn from_segmented(sm: &SourceMap<'_>, config: LexerConfig, seg: &SegmentedCommand) -> Self {
        Self::from_segmented_reusing_words(sm, config, seg, None)
    }

    /// Assemble raw lexical tokens, optionally reusing the exact word vector
    /// already checked by the original source projection. Reuse confers no
    /// binding, body entry or execution authority; stamping stays independent.
    #[must_use]
    pub(crate) fn from_segmented_reusing_words(
        sm: &SourceMap<'_>,
        config: LexerConfig,
        seg: &SegmentedCommand,
        original_words: Option<&[WordExpr]>,
    ) -> Self {
        let expand = seg.expand_word.as_deref();
        // One reused buffer: `WordExpr::from_word` takes the word's lexer
        // tokens, and the segmenter keeps them behind its own fragment type.
        let mut fragments: Vec<Token> = Vec::new();
        let word_exprs = if let Some(words) = original_words {
            words.to_vec()
        } else {
            let mut word_exprs = Vec::with_capacity(seg.argv.len());
            for (idx, token) in seg.argv.iter().enumerate() {
                fragments.clear();
                fragments.extend(
                    seg.word_fragments
                        .get(idx)
                        .map_or(&[][..], Vec::as_slice)
                        .iter()
                        .map(|fragment| fragment.token),
                );
                let text = seg.texts.get(idx).map_or("", String::as_str);
                let expanded = expand
                    .and_then(|flags| flags.get(idx))
                    .copied()
                    .unwrap_or(false);
                let expansion_span = expanded
                    .then(|| {
                        seg.all_tokens
                            .iter()
                            .rev()
                            .find(|candidate| {
                                candidate.kind == tcl_lexer::TokenType::Expand
                                    && candidate.span.end() <= token.span.start()
                            })
                            .map(|candidate| candidate.span)
                    })
                    .flatten();
                word_exprs.push(WordExpr::from_word(
                    sm,
                    config,
                    &fragments,
                    text,
                    expanded,
                    expansion_span,
                ));
            }
            word_exprs
        };
        Self {
            argv: seg.argv.iter().map(|token| token.span).collect(),
            argv_texts: seg.texts.clone(),
            word_exprs,
            argv_kinds: seg.argv.iter().map(|token| token.kind).collect(),
            single_token_word: seg.single_token_word.clone(),
            all_tokens: seg.all_tokens.iter().map(|token| token.span).collect(),
            expand_word: seg.expand_word.clone(),
            // Came from a Tcl word, so it is a command however it is spelled.
            synthetic: None,
            evaluated_body: None,
            source_binding: None,
            nested_bindings: std::sync::Arc::default(),
            variable_accesses: Vec::new(),
            hosted_taint_context: None,
            source_bpf_command: None,
        }
    }

    /// A token snapshot for a statement the CFG builder synthesised — see
    /// [`SyntheticMarker`] for why the marker has to live here rather than in
    /// the statement's `command` spelling.
    ///
    /// The word vectors are empty: a marker has no words, and
    /// [`Self::words_align_with_argv_text`] holds trivially. Consumers that
    /// need real word data (the WASM source-compatibility view) already
    /// decline a snapshot they cannot line up, which is the conservative
    /// direction.
    #[must_use]
    pub fn marker(kind: SyntheticMarker) -> Self {
        Self {
            argv: Vec::new(),
            argv_texts: Vec::new(),
            word_exprs: Vec::new(),
            argv_kinds: Vec::new(),
            single_token_word: Vec::new(),
            all_tokens: Vec::new(),
            expand_word: None,
            synthetic: Some(kind),
            evaluated_body: None,
            source_binding: None,
            nested_bindings: std::sync::Arc::default(),
            variable_accesses: Vec::new(),
            hosted_taint_context: None,
            source_bpf_command: None,
        }
    }

    /// Build compatibility token data when only the parallel arrays are
    /// available.
    ///
    /// New source lowering should use [`Self::from_segmented`]. This adapter
    /// intentionally produces opaque word nodes for data that cannot preserve
    /// ordered fragments; it never guesses Tcl substitution semantics.
    #[must_use]
    pub fn from_lossy_parts(
        argv: Vec<Span>,
        argv_texts: Vec<String>,
        argv_kinds: Vec<tcl_lexer::TokenType>,
        single_token_word: Vec<bool>,
        all_tokens: Vec<Span>,
        expand_word: Option<Vec<bool>>,
    ) -> Self {
        let word_exprs = argv_texts
            .iter()
            .enumerate()
            .map(|(idx, text)| {
                let span = argv.get(idx).copied().unwrap_or_else(|| Span::new(0, 0));
                let kind = argv_kinds
                    .get(idx)
                    .copied()
                    .unwrap_or(tcl_lexer::TokenType::Esc);
                let single = single_token_word.get(idx).copied().unwrap_or(false);
                let expanded = expand_word
                    .as_ref()
                    .and_then(|flags| flags.get(idx))
                    .copied()
                    .unwrap_or(false);
                WordExpr::from_lossy_snapshot(text, span, kind, single, expanded)
            })
            .collect();
        Self {
            argv,
            argv_texts,
            word_exprs,
            argv_kinds,
            single_token_word,
            all_tokens,
            expand_word,
            // A lossy snapshot of real words is still a command.
            synthetic: None,
            evaluated_body: None,
            source_binding: None,
            nested_bindings: std::sync::Arc::default(),
            variable_accesses: Vec::new(),
            hosted_taint_context: None,
            source_bpf_command: None,
        }
    }

    /// Attach proved script semantics without changing invocation tokens.
    #[must_use]
    pub fn with_evaluated_body(
        mut self,
        region: crate::execution_region::EvaluatedBodyRegion,
    ) -> Self {
        self.evaluated_body = region.valid().then(|| Box::new(region));
        self
    }

    /// Retain the shared command owner's answer, including explicit uncertainty.
    #[must_use]
    pub fn with_source_binding(
        mut self,
        binding: crate::command_binding::SourceInvocationBinding,
    ) -> Self {
        self.source_binding = Some(binding);
        self
    }

    /// Resolve a retained read by its exact source reference and spelling.
    /// The invocation context is sampled later and cannot replace this query.
    #[must_use]
    pub fn variable_access_at(
        &self,
        source: &SourceSite,
        spelling: &str,
    ) -> Option<&crate::command_binding::SourceVariableAccess> {
        crate::command_binding::SourceVariableAccess::find_at_source(
            &self.variable_accesses,
            source,
            spelling,
        )
    }

    /// Query a unique original read site without interpreting a compatibility
    /// spelling. Conflicting records at that exact site remain unresolved.
    #[must_use]
    pub fn variable_access_for_site(
        &self,
        source: &SourceSite,
    ) -> Option<&crate::command_binding::SourceVariableAccess> {
        crate::command_binding::SourceVariableAccess::find_at_site(&self.variable_accesses, source)
    }

    /// The nested proofs for writing; a list still shared with another
    /// command's tokens is copied first.
    pub fn nested_bindings_mut(
        &mut self,
    ) -> &mut Vec<(u32, crate::command_binding::SourceInvocationBinding)> {
        std::sync::Arc::make_mut(&mut self.nested_bindings)
    }

    /// Inherit exact nested dispatch proofs from the owning source invocation.
    /// A missing or non-source nested site stays explicitly unknown when the
    /// parent was queried; it never recovers meaning from a written head.
    pub fn inherit_nested_bindings(&mut self, parent: &Self) {
        self.hosted_taint_context
            .clone_from(&parent.hosted_taint_context);
        self.nested_bindings.clone_from(&parent.nested_bindings);
        self.variable_accesses.clone_from(&parent.variable_accesses);
        if parent.source_binding.is_none() {
            return;
        }
        let offset = self
            .words()
            .first()
            .filter(|word| word.source().provenance == Provenance::Source)
            .map(|word| word.source().span.start());
        self.source_binding = Some(
            offset
                .and_then(|offset| {
                    parent
                        .nested_bindings
                        .iter()
                        .find(|(site, _)| *site == offset)
                        .map(|(_, binding)| binding.clone())
                })
                .unwrap_or_else(crate::command_binding::SourceInvocationBinding::unknown),
        );
    }

    /// Whether this snapshot explicitly declines a unique registry target.
    ///
    /// This is distinct from an old snapshot that has never been queried.
    #[must_use]
    pub fn has_unproved_source_binding(&self) -> bool {
        self.source_binding.as_ref().is_some_and(|binding| {
            binding
                .proved_execution_target()
                .is_none_or(|target| !target.registry_backed)
        })
    }

    /// The wrapper invocation a proved wrapper's argument boundary
    /// ([`SyntheticMarker::EvaluatedArguments`]) hands its substituted words to.
    #[must_use]
    pub(crate) fn evaluated_arguments_invocation(&self) -> Option<Self> {
        (self.synthetic == Some(SyntheticMarker::EvaluatedArguments)).then(|| Self {
            synthetic: None,
            ..self.clone()
        })
    }

    /// Return an attached execution plan only when its routing is complete.
    ///
    /// Consumers use this fact to avoid recovering script effects a second time.
    #[must_use]
    pub fn evaluated_body(&self) -> Option<&crate::execution_region::EvaluatedBodyRegion> {
        self.evaluated_body
            .as_deref()
            .filter(|region| region.valid())
    }

    /// Structured word expressions in argv order.
    #[must_use]
    pub fn words(&self) -> &[WordExpr] {
        &self.word_exprs
    }

    /// Whether this synthetic statement's `defs`/`reads` are literal names
    /// its producer already proved (iteration bindings, `[…]` substitution
    /// effects, a condition's kill-set, upvar invalidations, opaque arm
    /// writes). Such a statement invokes nothing; its names resolve directly.
    #[must_use]
    pub fn carries_proved_names(&self) -> bool {
        matches!(
            self.synthetic,
            Some(
                SyntheticMarker::IterationBindings(_)
                    | SyntheticMarker::WordEffects(_)
                    | SyntheticMarker::Condition
                    | SyntheticMarker::UpvarInvalidate
                    | SyntheticMarker::ArmWrites
            )
        )
    }

    /// Whether this stage may dispatch the source command's handler.
    /// Argument evaluation and proved-name bookkeeping retain a command label
    /// without invoking it. Their word effects remain independently executable.
    #[must_use]
    pub fn may_dispatch_source_command(&self) -> bool {
        self.synthetic != Some(SyntheticMarker::EvaluatedArguments) && !self.carries_proved_names()
    }

    /// Whether this boundary evaluates its written words. Residual wrapper
    /// boundaries refer to the argv already evaluated before entering phases.
    #[must_use]
    pub fn evaluates_words(&self) -> bool {
        !matches!(
            self.synthetic,
            Some(
                SyntheticMarker::EvaluatedWrapper
                    | SyntheticMarker::DictionaryScopeEntry
                    | SyntheticMarker::DictionaryScopeWriteback(_)
                    | SyntheticMarker::IterationBindings(_)
                    | SyntheticMarker::CapturedCatchOutputs
            )
        )
    }

    /// Whether the structured compatibility view still aligns with argv text.
    ///
    /// Synthetic codegen snapshots may intentionally omit representative spans
    /// or argv text, so this checks the one invariant the semantic sidecar
    /// requires rather than imposing a stronger invariant on compatibility
    /// consumers.
    #[must_use]
    pub fn words_align_with_argv_text(&self) -> bool {
        self.word_exprs.len() == self.argv_texts.len()
    }

    /// Complete original-word compatibility presentations agree with this
    /// carrier. This is consistency only: source/config/currentness, synthetic
    /// purpose, Native identity and effective alias/expansion argv are checked
    /// separately by the original owner. Count-only compatibility snapshots
    /// continue to use `words_align_with_argv_text`.
    #[must_use]
    pub fn original_words_match_argv_text(&self) -> bool {
        self.words_align_with_argv_text()
            && self
                .words()
                .iter()
                .zip(&self.argv_texts)
                .all(|(word, text)| word.legacy_text() == *text)
    }

    /// Whether **argument** `idx` (0-based, so `argv[idx + 1]`) is a
    /// brace-quoted literal word — `{…}`, the one word form Tcl leaves
    /// completely unsubstituted.
    ///
    /// The IR stores argument *content*, which cannot tell `{$a}` (the
    /// literal three-character name `$a`) from `$a` (a substitution); the
    /// per-word token kind can.  A `$var` lexes to `Var`, a `[cmd]` to `Cmd`,
    /// a `"…"` (which *does* substitute) to `Esc`, and only a brace-quoted
    /// word to `Str`.
    ///
    /// The single-token conjunct is belt-and-braces: a word that opens with
    /// `{` must be entirely brace-quoted (`set x {a}{b}` is a tclsh parse
    /// error, "extra characters after close-brace"), so a `Str`
    /// representative token already implies a single-token word.
    ///
    /// The one definition of this question in the compiler — the dynamic-name
    /// barrier, the place bridge, the inliner's literal-argument binding and
    /// the def/use naming layer all ask it here rather than re-deriving it.
    #[must_use]
    pub fn arg_is_braced_literal(&self, idx: usize) -> bool {
        let Some(word_index) = idx.checked_add(1) else {
            return false;
        };
        self.argv_kinds.get(word_index) == Some(&tcl_lexer::TokenType::Str)
            && self
                .single_token_word
                .get(word_index)
                .copied()
                .unwrap_or(true)
    }
}
