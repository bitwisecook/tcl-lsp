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

//! `FunctionUnit` accessors: retained source metadata, semantic values, existence facts and source positions.

use super::{ExistencePoint, FunctionUnit};
use std::sync::Arc;
use tcl_registry::CommandRegistry;
use tcl_registry::value_transfer::{AnalysisTier, DeclineReason, DomainFact, FactView};

impl FunctionUnit {
    pub(crate) fn with_body_id(mut self, id: crate::ir::ProcedureId) -> Self {
        self.body_id = Some(id);
        self
    }

    /// Complete actual source input, independently of Native execution grants.
    #[must_use]
    pub fn source_metadata_input(&self) -> Option<&crate::analyser::ResolvedAnalysisInput> {
        self.source_metadata_input.as_ref()
    }

    /// Exact grammar used to construct this function's source facts.
    #[must_use]
    pub const fn source_lexer_config(&self) -> tcl_lexer::LexerConfig {
        self.source_config
    }

    /// Borrow the retained generation only for its actual immutable command store.
    #[must_use]
    pub fn invocation_metadata_context<'a>(
        &'a self,
        registry: &CommandRegistry,
    ) -> Option<crate::registry_invocation::InvocationMetadataContext<'a>> {
        let input = self.source_metadata_input()?;
        if self.source_config.normalized() != input.lexer_config().normalized() {
            return None;
        }
        crate::registry_invocation::InvocationMetadataContext::for_analysis_input(registry, input)
    }

    /// Borrow source metadata only when this function and module retain the
    /// same actual availability owner and source profiles. Each producer's
    /// lexer configuration is checked independently, so a genuine nested-body
    /// BOM rule does not have to equal the file-level rule.
    #[must_use]
    pub fn invocation_metadata_context_for_module<'a>(
        &'a self,
        registry: &CommandRegistry,
        module: &crate::ir::Module,
    ) -> Option<crate::registry_invocation::InvocationMetadataContext<'a>> {
        let module_input =
            crate::registry_invocation::InvocationMetadataContext::for_module(registry, module)?
                .source_analysis_input()?;
        let input = self.source_metadata_input()?;
        if !std::ptr::eq(
            input.borrowed_context_registry(),
            module_input.borrowed_context_registry(),
        ) || input.analyser_profile().cache_key() != module_input.analyser_profile().cache_key()
            || input.unit_profile().cache_key() != module_input.unit_profile().cache_key()
            || input.vendor_source_policy() != module_input.vendor_source_policy()
        {
            return None;
        }
        self.invocation_metadata_context(registry)
    }

    pub(super) fn with_source_metadata(
        mut self,
        input: Option<&crate::analyser::ResolvedAnalysisInput>,
        config: tcl_lexer::LexerConfig,
    ) -> Self {
        self.source_metadata_input = input.cloned();
        self.source_config = config;
        self
    }

    /// Known contents and producer obligations for diagnostics, without erasure permission.
    /// Guarded/carrierless fixture units may have no separate semantic projection.
    #[must_use]
    pub fn semantic_values(&self) -> Option<&crate::sccp::SemanticValueFacts> {
        self.has_original_semantic_value_owner().then_some(())?;
        self.semantic_value_projection.get(&self.cfg, &self.ssa)
    }

    /// Diagnostic knowledge keeps producers and never grants an executable replacement.
    pub(crate) fn diagnostic_value_facts(&self) -> crate::sccp::DiagnosticValueFacts<'_> {
        self.semantic_values().map_or_else(
            || crate::sccp::DiagnosticValueFacts::compatibility(&self.sccp),
            crate::sccp::DiagnosticValueFacts::from_semantic,
        )
    }

    /// Correspondence to the actual immutable construction cache. Complete
    /// source metadata and positioned read ownership remain separate guards.
    pub(crate) fn has_original_semantic_value_owner(&self) -> bool {
        self.retained_semantic_value_owner
            .as_ref()
            .is_some_and(|owner| Arc::ptr_eq(owner, &self.semantic_value_projection))
    }

    /// Detach lazy values before every CFG/SSA proof transformation. Restoration,
    /// relocation and lexical rebasing all use this same lifecycle. A swapped
    /// producer remains unavailable rather than becoming a new original owner.
    pub fn invalidate_semantic_values(&mut self) {
        if !self.has_original_semantic_value_owner() {
            self.retained_semantic_value_owner = None;
            self.semantic_value_projection = Arc::default();
            return;
        }
        self.semantic_value_projection = Arc::new(self.semantic_value_projection.uncached());
        self.retained_semantic_value_owner = Some(Arc::clone(&self.semantic_value_projection));
    }

    /// Analysis constants whose implicit math calls require runtime validation.
    /// A backend may consume them only while retaining and checking the exact
    /// dependencies, independently of native compiler admission obligations.
    #[must_use]
    pub fn requires_native_math_binding_validation(&self) -> bool {
        self.cfg
            .implicit_math_invocations
            .iter()
            .any(|call| call.reached().is_none())
            || !self.cfg.required_math_invocations.is_empty()
            || !self.sccp.required_math_invocations.is_empty()
            || !self.cfg.required_expression_preparations.is_empty()
            || !self.sccp.required_expression_preparations.is_empty()
    }

    /// The existence fact `symbol`'s place holds at `point`, as the
    /// registry's input view (`docs/design/compiler/value-transfers.md`
    /// § *Existence*, availability across tiers): `Domain(Existence(_))`
    /// where the run computed one, `Pending` where it never reached the
    /// point, and `Top(Unavailable(tier))` for a unit computed below the
    /// deep tier or over the complexity ceiling ([`Self::tier`]) — neither
    /// bound nor unbound, so no consumer reads it as either and every one
    /// stays silent on it.
    #[must_use]
    pub fn existence(&self, symbol: crate::ssa::Symbol, point: ExistencePoint) -> FactView {
        if self.tier != AnalysisTier::Deep {
            return FactView::Top(DeclineReason::Unavailable(self.tier));
        }
        let fact = match point {
            ExistencePoint::Before(block, index) => {
                self.sccp.existence_before(block, index, symbol)
            }
            ExistencePoint::Exit(block) => self.sccp.existence_at_exit(block, symbol),
            ExistencePoint::Version(version) => {
                self.sccp.existence.get(&(symbol, version)).copied()
            }
        };
        fact.map_or(FactView::Pending, |fact| {
            FactView::Domain(DomainFact::Existence(fact))
        })
    }

    /// Whether this function's [`Self::dynamic_names`] barrier forbids any
    /// pass that *moves, folds, or deletes* a variable's value across other
    /// statements.
    ///
    /// A computed variable name (`set $name v`, `unset $n`, `[set $v]`)
    /// can create, destroy, or observe **any** variable in the frame, so no
    /// "sole reaching definition", "write-only chain", or "not referenced
    /// later" fact derived from spelled names is trustworthy anywhere in the
    /// function.  Consulted by the value-motion passes — O102 load
    /// forwarding, O104 / O130 chain folding, O119 multi-`set` packing, and
    /// O125 code sinking — which abstain for the whole function when it
    /// answers `true`, matching the abstention O109 / O126 elimination and
    /// SCCP's existence fold already apply.
    #[must_use]
    pub fn dynamic_barrier_blocks_value_motion(&self) -> bool {
        !self.dynamic_names.is_clear() || self.cfg.has_opaque_native_accesses()
    }

    /// Recover the absolute span of `span` (a span carried by this unit's
    /// `cfg`/`ssa`/`sccp`) by adding [`Self::base_offset`].  A no-op when the
    /// unit was built at its real position (`base_offset == 0`); the
    /// offset-aware seam for the memoised (offset-0) path.
    #[must_use]
    pub fn abs_span(&self, span: tcl_lexer::Span) -> tcl_lexer::Span {
        if self.base_offset == 0 {
            return span;
        }
        let s = (i64::from(span.start()) + self.base_offset).max(0);
        let e = (i64::from(span.end()) + self.base_offset).max(0);
        // `s`/`e` are clamped to `>= 0`; a value past `u32::MAX` is a degenerate
        // out-of-range offset, so saturate rather than wrap.
        let s = u32::try_from(s).unwrap_or(u32::MAX);
        let e = u32::try_from(e).unwrap_or(u32::MAX);
        tcl_lexer::Span::new(s, e)
    }

    /// Recover the absolute byte position of `pos` by adding
    /// [`Self::base_offset`] (see [`Self::abs_span`]).
    #[must_use]
    pub fn abs_pos(&self, pos: u32) -> u32 {
        if self.base_offset == 0 {
            return pos;
        }
        // Clamped to `>= 0`; saturate a degenerate out-of-range offset.
        u32::try_from((i64::from(pos) + self.base_offset).max(0)).unwrap_or(u32::MAX)
    }

    /// The statement and word of this unit whose source range is `span`
    /// (absolute, as [`Self::abs_span`] gives it), when a call in a block
    /// the solver reached wrote it: the address
    /// [`crate::value_transfer::proven_word_value`] reads. A word is matched
    /// by its whole range or by its representative token's, `0` being the
    /// command's own.
    #[must_use]
    pub fn word_at(
        &self,
        span: tcl_lexer::Span,
    ) -> Option<(crate::value_transfer::StatementId, usize)> {
        let mut blocks: Vec<crate::cfg::BlockId> =
            self.sccp.executable_blocks.iter().copied().collect();
        blocks.sort_unstable();
        blocks.into_iter().find_map(|block| {
            let statements = &self.cfg.blocks.get(&block)?.statements;
            statements
                .iter()
                .enumerate()
                .find_map(|(index, statement)| {
                    let crate::ir::Statement::Call {
                        tokens: Some(tokens),
                        ..
                    } = statement
                    else {
                        return None;
                    };
                    if tokens.synthetic.is_some() {
                        return None;
                    }
                    let word = (0..tokens.argv.len().max(tokens.word_exprs.len())).find(|&at| {
                        tokens
                            .word_exprs
                            .get(at)
                            .is_some_and(|word| self.abs_span(word.source().span) == span)
                            || tokens
                                .argv
                                .get(at)
                                .is_some_and(|&token| self.abs_span(token) == span)
                    })?;
                    Some((crate::value_transfer::StatementId { block, index }, word))
                })
        })
    }
}
