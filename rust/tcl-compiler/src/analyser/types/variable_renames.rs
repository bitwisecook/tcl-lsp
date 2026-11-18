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

//! `AnalysisResult` queries for renaming a variable: the original symbol, root and frame at a cursor, alias coverage and the alpha-rename equivalences.

use super::AnalysisResult;
use super::procedures::ProcDef;
use tcl_lexer::Span;

#[derive(Clone, Copy)]
enum ScalarAlphaFormalSelection {
    Single,
    Fixed,
}

impl AnalysisResult {
    /// Semantic alpha equivalence of a closed source containing one procedure
    /// whose sole required scalar formal is returned as the same whole object.
    /// Actual frame/read/handler effects are proved separately from source
    /// rename coverage. The receipt supplies neither an edit nor a reached call.
    #[must_use]
    pub fn original_scalar_body_alpha_rename(
        &self,
        declaration: &crate::signature_scan::original_name::SourceDeclarationMetadata<ProcDef>,
        new_native_tail: &[u8],
        registry: &tcl_registry::CommandRegistry,
    ) -> Option<crate::command_binding::OriginalScalarBodyAlphaRename> {
        self.original_scalar_alpha_for_formals(
            declaration,
            new_native_tail,
            registry,
            ScalarAlphaFormalSelection::Single,
        )
    }

    /// Fixed scalar formal equivalence in a closed original return body.
    /// Every actual formal and original read keeps its separate edit coverage.
    #[must_use]
    pub fn original_fixed_scalar_body_alpha_rename(
        &self,
        declaration: &crate::signature_scan::original_name::SourceDeclarationMetadata<ProcDef>,
        new_native_tail: &[u8],
        registry: &tcl_registry::CommandRegistry,
    ) -> Option<crate::command_binding::OriginalScalarBodyAlphaRename> {
        self.original_scalar_alpha_for_formals(
            declaration,
            new_native_tail,
            registry,
            ScalarAlphaFormalSelection::Fixed,
        )
    }

    fn original_scalar_alpha_for_formals(
        &self,
        declaration: &crate::signature_scan::original_name::SourceDeclarationMetadata<ProcDef>,
        new_native_tail: &[u8],
        registry: &tcl_registry::CommandRegistry,
        selection: ScalarAlphaFormalSelection,
    ) -> Option<crate::command_binding::OriginalScalarBodyAlphaRename> {
        use crate::signature_scan::variable_symbol::OriginalVariableSymbolReceiver;
        if !self.retains_original_procedure(declaration)
            || !self.matches_original_source_image(
                declaration.name_input().source_image(),
                declaration.name_input().lexer_config(),
            )
        {
            return None;
        }
        let bindings = self.command_realm.as_ref()?.source_bindings_ref();
        if tcl_dialect::trace_env::var_os("TCL_LSP_TRACE_SCALAR_ALPHA").is_some() {
            eprintln!(
                "SCALAR_ALPHA offset={} symbols={} formals={} completed-world={}",
                declaration.declaration_site().offset,
                self.original_variable_symbols.len(),
                self.original_variable_symbols
                    .iter()
                    .filter(|occurrence| occurrence.receiver()
                        == OriginalVariableSymbolReceiver::FormalDeclaration)
                    .count(),
                bindings.original_completed_command_world().is_some()
            );
        }
        let mut agreed = None;
        for occurrence in &self.original_variable_symbols {
            if tcl_dialect::trace_env::var_os("TCL_LSP_TRACE_SCALAR_ALPHA").is_some()
                && occurrence.receiver() == OriginalVariableSymbolReceiver::FormalDeclaration
            {
                eprintln!(
                    "SCALAR_ALPHA formal-span={}..{} declaration={} coverage={}",
                    occurrence.span().start(),
                    occurrence.span().end(),
                    occurrence.is_declaration(),
                    self.original_variable_rename_is_complete(occurrence.symbol())
                );
            }
            if occurrence.receiver() != OriginalVariableSymbolReceiver::FormalDeclaration
                || !occurrence.is_declaration()
                || !self.original_variable_rename_is_complete(occurrence.symbol())
            {
                continue;
            }
            let receipt = match selection {
                ScalarAlphaFormalSelection::Single => bindings.original_scalar_body_alpha_rename(
                    declaration,
                    occurrence.symbol(),
                    new_native_tail,
                    registry,
                ),
                ScalarAlphaFormalSelection::Fixed => bindings
                    .original_fixed_scalar_body_alpha_rename(
                        declaration,
                        occurrence.symbol(),
                        new_native_tail,
                        registry,
                    ),
            };
            let Some(receipt) = receipt else {
                continue;
            };
            if agreed.as_ref().is_some_and(|previous| previous != &receipt) {
                return None;
            }
            agreed = Some(receipt);
        }
        agreed
    }

    /// Closed literal-local store/return equivalence, independently of edits.
    #[must_use]
    pub fn original_local_store_alpha_rename(
        &self,
        declaration: &crate::signature_scan::original_name::SourceDeclarationMetadata<ProcDef>,
        proposed: &[u8],
        registry: &tcl_registry::CommandRegistry,
    ) -> Option<crate::command_binding::OriginalLocalStoreAlphaRename> {
        if !self.retains_original_procedure(declaration)
            || !self.matches_original_source_image(
                declaration.name_input().source_image(),
                declaration.name_input().lexer_config(),
            )
        {
            return None;
        }
        let bindings = self.command_realm.as_ref()?.source_bindings_ref();
        let mut agreed = None;
        for occurrence in &self.original_variable_symbols {
            if !self.original_variable_rename_is_complete(occurrence.symbol()) {
                continue;
            }
            let symbol = occurrence.symbol();
            let crate::signature_scan::variable_symbol::SignatureSourceVariableSlot::Local {
                frame,
                simple,
            } = symbol.slot()
            else {
                continue;
            };
            if self.original_variable_symbols.iter().any(|other| {
                matches!(other.symbol().slot(), crate::signature_scan::variable_symbol::SignatureSourceVariableSlot::Local { frame: other_frame, simple: other_name }
                    if other_frame == frame && other_name != simple && other_name.as_bytes() == proposed)
            }) { continue; }
            let Some(receipt) =
                bindings.original_local_store_alpha_rename(declaration, symbol, proposed, registry)
            else {
                continue;
            };
            if agreed.as_ref().is_some_and(|previous| previous != &receipt) {
                return None;
            }
            agreed = Some(receipt);
        }
        agreed
    }

    /// Select the smallest original naming occurrence at a byte cursor. Equal
    /// source geometry must unanimously identify the same typed byte symbol.
    /// Display strings and current runtime cells are not consulted.
    #[must_use]
    pub fn original_variable_symbol_at_offset(
        &self,
        offset: u32,
    ) -> Option<&crate::signature_scan::variable_symbol::SignatureSourceVariableOccurrence> {
        let contains = |span: Span| span.start() <= offset && offset < span.end();
        if self
            .original_variable_symbol_conflicts
            .iter()
            .copied()
            .any(contains)
        {
            return None;
        }
        let length = self
            .original_variable_symbols
            .iter()
            .filter(|site| contains(site.span()))
            .map(|site| site.span().end() - site.span().start())
            .min()?;
        let mut sites = self.original_variable_symbols.iter().filter(|site| {
            contains(site.span()) && site.span().end() - site.span().start() == length
        });
        let first = sites.next()?;
        sites
            .all(|site| site.symbol() == first.symbol())
            .then_some(first)
    }

    /// Whether every selected alias declaration has its original byte/frame
    /// receipt. Unknown computed variable names remain a rename barrier. This
    /// is source coverage only, never executed alias or current-cell authority.
    /// A document need not contain the symbol; selection and edit existence
    /// remain independent caller obligations.
    #[must_use]
    pub fn original_variable_rename_is_complete(
        &self,
        symbol: &crate::signature_scan::variable_symbol::SignatureSourceVariableSymbol,
    ) -> bool {
        if self.original_variable_symbols.iter().any(|occurrence| {
            occurrence.symbol() == symbol && occurrence.has_source_applicability_obligation()
        }) {
            return false;
        }
        #[cfg(test)]
        if tcl_dialect::trace_env::var_os("TCL_LSP_TRACE_ORIGINAL_VARIABLE_SYMBOLS").is_some() {
            eprintln!(
                "ORIGINAL_VARIABLE_COVERAGE policy={:?} local={} dynamic={:?} unknowns={:?} aliases={:?} receipt_sites={:?} conflicts={:?}",
                symbol.policy(),
                !symbol.is_namespace(),
                self.dynamic_variable_names,
                self.original_variable_name_unknowns,
                self.original_variable_alias_obligations,
                self.original_variable_alias_receipts
                    .iter()
                    .map(crate::signature_scan::variable_symbol::OriginalVariableAliasReceipt::invocation_offset)
                    .collect::<Vec<_>>(),
                self.original_variable_symbol_conflicts
            );
        }
        if self.retained_command_realm().is_none_or(|realm| {
            if let Some(input) = self
                .resolved_input
                .as_ref()
                .filter(|input| input.has_logical_source_name_context())
            {
                realm.original_logical_variable_formal_name_is_unrepresented(input, symbol)
            } else {
                realm
                    .source_bindings_ref()
                    .original_variable_formal_name_is_unrepresented(symbol)
            }
        }) {
            return false;
        }
        self.original_variable_alias_coverage()
    }

    /// [`Self::original_variable_rename_is_complete`] for renaming a C
    /// namespace cell to `proposed` (owner ruling, 17 November 2026): a
    /// computed variable name is a barrier only when it could spell the old or
    /// the new cell, and an alias whose computed namespace sits beside a
    /// written name with another tail cannot link either. Exclusions come only
    /// from [`crate::dynamic_names::computed_variable_name_cannot_spell`]; an
    /// excluded site is neither edited nor counted as covered. Local symbols
    /// keep the whole-document answer.
    #[must_use]
    pub fn original_variable_rename_is_complete_for(
        &self,
        symbol: &crate::signature_scan::variable_symbol::SignatureSourceVariableSymbol,
        proposed: &crate::signature_scan::variable_symbol::SignatureSourceVariableSymbol,
    ) -> bool {
        let (Some(old), Some(new)) = (
            symbol.checked_c_namespace_key(),
            proposed.checked_c_namespace_key(),
        ) else {
            return self.original_variable_rename_is_complete(symbol);
        };
        if self.original_variable_rename_is_complete(symbol) {
            return true;
        }
        let cells = [old.as_str(), new.as_str()];
        self.original_variable_rename_preconditions(symbol)
            && self.original_variable_alias_coverage_excluding(&cells)
    }

    fn original_variable_rename_preconditions(
        &self,
        symbol: &crate::signature_scan::variable_symbol::SignatureSourceVariableSymbol,
    ) -> bool {
        !self.original_variable_symbols.iter().any(|occurrence| {
            occurrence.symbol() == symbol && occurrence.has_source_applicability_obligation()
        }) && self.retained_command_realm().is_some_and(|realm| {
            if let Some(input) = self
                .resolved_input
                .as_ref()
                .filter(|input| input.has_logical_source_name_context())
            {
                !realm.original_logical_variable_formal_name_is_unrepresented(input, symbol)
            } else {
                !realm
                    .source_bindings_ref()
                    .original_variable_formal_name_is_unrepresented(symbol)
            }
        })
    }

    fn original_variable_alias_coverage_excluding(&self, cells: &[&str]) -> bool {
        let Some(config) = self.body_lexer_config else {
            return false;
        };
        let image = self
            .retained_command_realm()
            .and_then(crate::realm::CommandBindingRealm::original_source_image);
        let text = |span: tcl_lexer::Span| {
            image
                .and_then(|image| image.bytes().get(span.as_range()))
                .and_then(|bytes| std::str::from_utf8(bytes).ok())
        };
        let cannot_spell = |span| {
            text(span).is_some_and(|word| {
                crate::dynamic_names::computed_variable_name_cannot_spell(word, cells, config)
            })
        };
        let written_tail_differs = |span| {
            text(span).is_some_and(|word| {
                !word.is_empty()
                    && !word.starts_with(['"', '{'])
                    && !word.contains(['$', '[', '\\', '\0'])
                    && cells.iter().all(|cell| {
                        tcl_syntax::naming::key_tail(cell) != tcl_syntax::naming::key_tail(word)
                    })
            })
        };
        // The local word of an alias whose written target tail rules out
        // both cells names only that alias.
        let excluded_alias_local = |span: tcl_lexer::Span| {
            self.original_variable_alias_operands
                .iter()
                .any(|operand| operand.1 == span && written_tail_differs(operand.2))
        };
        self.dynamic_variable_names
            .iter()
            .all(|site| cannot_spell(site.span))
            && self
                .original_variable_name_unknowns
                .iter()
                .all(|&span| excluded_alias_local(span))
            && self
                .original_variable_alias_obligations
                .iter()
                .all(|&(site, count)| {
                    let receipts = self
                        .original_variable_alias_receipts
                        .iter()
                        .filter(|receipt| receipt.invocation_offset() == site)
                        .count();
                    let targets: Vec<_> = self
                        .original_variable_alias_operands
                        .iter()
                        .filter(|operand| operand.0 == site)
                        .map(|operand| operand.2)
                        .collect();
                    receipts == count
                        || (targets.len() >= count
                            && targets.iter().all(|&target| written_tail_differs(target)))
                })
            && self.original_variable_symbol_conflicts.is_empty()
    }

    /// Whether the selected recorded variable-alias declarations have complete
    /// source-name coverage in this actual current image/input/configuration.
    /// This is not a runtime alias census, successful link, binding continuity,
    /// element/lifetime, physical cell or editable-reference-set certificate.
    #[must_use]
    pub fn original_variable_alias_source_is_complete(
        &self,
        image: &tcl_lexer::SourceImage,
        config: tcl_lexer::LexerConfig,
    ) -> bool {
        self.matches_original_source_image(image, config) && self.original_variable_alias_coverage()
    }

    fn original_variable_alias_coverage(&self) -> bool {
        self.dynamic_variable_names.is_empty()
            && self.original_variable_name_unknowns.is_empty()
            && self
                .original_variable_alias_obligations
                .iter()
                .all(|&(site, count)| {
                    self.original_variable_alias_receipts
                        .iter()
                        .filter(|receipt| receipt.invocation_offset() == site)
                        .count()
                        == count
                })
            && self.original_variable_symbol_conflicts.is_empty()
    }

    /// Original naming advice belongs to the exact complete consumer image,
    /// channel and scanner configuration. Copied producer images may differ;
    /// they do not replace this independently retained consumer correspondence.
    #[must_use]
    pub fn original_variable_symbol_in_source(
        &self,
        image: &tcl_lexer::SourceImage,
        config: tcl_lexer::LexerConfig,
        offset: u32,
    ) -> Option<&crate::signature_scan::variable_symbol::SignatureSourceVariableOccurrence> {
        self.matches_original_source_image(image, config)
            .then_some(())?;
        let occurrence = self.original_variable_symbol_at_offset(offset)?;
        occurrence
            .matches_logical_source(image, self.resolved_input.as_ref()?)
            .then_some(occurrence)
    }

    /// Authentic lexical substitution at this cursor, even when its selected
    /// variable symbol is unknown. Membership, complete source and grammar
    /// agree; no namespace-role operand or computed value becomes a root.
    #[must_use]
    pub fn original_variable_root_in_source(
        &self,
        image: &tcl_lexer::SourceImage,
        config: tcl_lexer::LexerConfig,
        offset: u32,
    ) -> Option<&crate::signature_scan::variable_name::SignatureSourceVariableRoot> {
        self.matches_original_source_image(image, config)
            .then_some(())?;
        let contains =
            |root: &&crate::signature_scan::variable_name::SignatureSourceVariableRoot| {
                let span = root.part_span();
                root.source_image() == image
                    && root.lexer_config() == config
                    && span.start() <= offset
                    && offset < span.end()
            };
        let length = self
            .original_variable_roots
            .iter()
            .filter(contains)
            .map(|root| root.part_span().end() - root.part_span().start())
            .min()?;
        let mut roots = self
            .original_variable_roots
            .iter()
            .filter(contains)
            .filter(|root| root.part_span().end() - root.part_span().start() == length);
        let first = roots.next()?;
        roots.all(|root| root == first).then_some(first)
    }

    /// Source-only local alias candidates retained by their original
    /// declarations, even when the body has no complete read of that alias.
    /// The whole consumer image/configuration must still match. Target cells,
    /// entered links and editable reference coverage remain independent.
    #[must_use]
    pub fn original_variable_alias_advice_in_source(
        &self,
        image: &tcl_lexer::SourceImage,
        config: tcl_lexer::LexerConfig,
    ) -> Option<Vec<crate::signature_scan::variable_symbol::OriginalVariableAliasAdvice<'_>>> {
        self.matches_original_source_image(image, config)
            .then_some(())?;
        Some(self.original_variable_alias_receipts.iter()
            .filter(|receipt| receipt.frame().lexer_config() == config)
            .map(crate::signature_scan::variable_symbol::OriginalVariableAliasAdvice::from_receipt)
            .collect())
    }

    /// Original local naming frame at this cursor, including whitespace inside
    /// an authenticated body. This is lexical visibility only, without an
    /// entered activation, current contents or native frame capability.
    #[must_use]
    pub fn original_variable_frame_in_source(
        &self,
        image: &tcl_lexer::SourceImage,
        config: tcl_lexer::LexerConfig,
        offset: u32,
    ) -> Option<crate::command_binding::SourceOriginalVariableFrame> {
        self.matches_original_source_image(image, config)
            .then_some(())?;
        let input = self.resolved_input.as_ref()?;
        if input.has_logical_source_name_context() {
            let mut pending = vec![&self.global_scope];
            let mut nearest = Vec::new();
            let mut length = u32::MAX;
            while let Some(scope) = pending.pop() {
                pending.extend(&scope.children);
                let Some(span) = scope
                    .body_span
                    .filter(|span| span.start() <= offset && offset <= span.end())
                else {
                    continue;
                };
                let size = span.end() - span.start();
                if size < length {
                    nearest.clear();
                    length = size;
                }
                if size == length {
                    nearest.push(scope);
                }
            }
            if nearest
                .iter()
                .any(|scope| scope.requires_original_lambda_source_context)
            {
                let first = *nearest.first()?;
                let receipt = first.original_logical_lambda_namespace.as_ref()?;
                if nearest.iter().any(|scope| {
                    !scope.requires_original_lambda_source_context
                        || scope.body_span != first.body_span
                        || scope.original_logical_lambda_namespace.as_ref() != Some(receipt)
                }) {
                    return None;
                }
                let frame = crate::command_binding::SourceOriginalVariableFrame::from_logical_lambda_source(
                    std::sync::Arc::clone(receipt.logical_source_owner()?), input,
                )?;
                let span = first.body_span?;
                if span.start().checked_add(1) != Some(frame.source().base())
                    || span.end()
                        != frame
                            .source()
                            .base()
                            .checked_add(u32::try_from(frame.source().text.bytes().len()).ok()?)?
                    || image.bytes().get(span.start() as usize) != Some(&b'{')
                {
                    return None;
                }
                let selected = self
                    .retained_command_realm()?
                    .original_logical_variable_frame_at_offset(input, image, offset)?;
                return (selected == frame).then_some(selected);
            }
            return self
                .retained_command_realm()?
                .original_logical_variable_frame_at_offset(input, image, offset);
        }
        self.retained_command_realm()?
            .source_bindings_ref()
            .original_variable_frame_at_offset(offset, config)
    }
}

#[cfg(test)]
mod computed_alias_exclusion_tests {
    /// tclsh 9.0.4: `namespace upvar $ns counter local` with ns `::mypkg`
    /// links `counter`; its local word names only that alias. A written tail
    /// spelling either cell, or a computed tail, keeps the barrier.
    #[test]
    fn an_alias_local_word_is_excluded_only_when_its_written_tail_rules_out_both_cells() {
        let cells = ["::mypkg::version", "::mypkg::release"];
        for (tail, excluded) in [
            ("counter", true),
            ("version", false),
            ("release", false),
            ("$t", false),
            ("{counter}", false),
        ] {
            let source =
                format!("proc show {{ns t}} {{\n    namespace upvar $ns {tail} local\n}}\n");
            let analysis = crate::analyser::Analyser::new().analyse(&source, "tcl8.6");
            assert_eq!(
                analysis.original_variable_alias_coverage_excluding(&cells),
                excluded,
                "{tail}"
            );
        }
    }
}
