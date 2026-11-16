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

//! Registry symbol-definer commands: the lightweight named definition each records.

use super::super::state::Analyser;
use super::super::types::DefinedSymbol;
use super::qualify;
use tcl_lexer::{Span, Token, TokenType};

/// Condense a definition's description argument into a single-line outline
/// detail: the first non-empty line, trimmed, and length-capped so a verbose
/// multi-line description doesn't bloat the outline entry.
fn summarise_detail(description: &str) -> String {
    const MAX: usize = 80;
    let line = description
        .lines()
        .map(str::trim)
        .find(|l| !l.is_empty())
        .unwrap_or("")
        .trim();
    if line.chars().count() > MAX {
        let truncated: String = line.chars().take(MAX - 1).collect();
        format!("{truncated}…")
    } else {
        line.to_string()
    }
}

impl Analyser {
    /// Record a lightweight named definition for a registry *symbol-definer*
    /// command (`tcltest::test NAME …`, …) so it appears in the document /
    /// workspace outline alongside procs, classes, and variables.
    ///
    /// Everything command-specific is registry data: which argument holds the
    /// name, which holds the description, and the outline category all come from
    /// the command's [`tcl_registry::SymbolDef`] — there is no `cmd == "test"`
    /// arm here, so registering the next such command (a benchmark runner, a
    /// custom test wrapper) is a one-line spec change.
    ///
    /// Names use original-word values and checked Logical declaration provenance.
    /// Unresolved substitutions cannot supply outline identities.
    pub(in super::super) fn handle_defines_symbol(
        &mut self,
        advice: &crate::registry_invocation::OriginalSymbolDeclarationAdvice,
        args: &[String],
        arg_tokens: &[Token],
        arg_single: &[bool],
        command_token: Token,
        scope_path: &[usize],
    ) {
        if !self.registry_symbol_placement_is_valid(advice, args, arg_tokens, arg_single) {
            return;
        }
        let sym = advice.symbol;
        // A command that defines only in one form (the `testConstraint NAME
        // value` setter, not the `testConstraint NAME` getter) records a symbol
        // only when its defining argument is present.
        if let Some(req) = sym.requires_arg
            && args.get(req as usize).is_none()
        {
            return;
        }
        let name_idx = sym.name_arg as usize;
        let (Some(name_tok), Some(name_text), Some(name_single)) = (
            arg_tokens.get(name_idx).copied(),
            args.get(name_idx),
            arg_single.get(name_idx).copied(),
        ) else {
            return;
        };
        let vendor_name = self
            .result
            .original_vendor_source_name_in_source(
                &tcl_lexer::SourceImage::document(&self.source),
                self.file_lexer_config(),
                name_tok.span,
            )
            .cloned();
        let original_name_input = self
            .head_identities
            .original_written_name_input_at_span(name_tok.span, self.file_lexer_config())
            .or_else(|| {
                Some(
                    crate::signature_scan::scope::SignatureSourceNameInput::OriginalWord(
                        self.original_static_source_name_at_span(name_tok.span)?
                            .name_input()
                            .clone(),
                    ),
                )
            });
        let name = self.defined_symbol_reporting_name(
            original_name_input.as_ref(),
            vendor_name.as_ref(),
            name_text,
            name_tok,
            name_single,
            scope_path,
        );
        let Some(name) = name else {
            return;
        };
        if name.is_empty() {
            return;
        }

        // The description argument, but only when it too resolves to a constant.
        let detail = sym.detail_arg.and_then(|d| {
            let di = d as usize;
            let tok = arg_tokens.get(di).copied()?;
            let text = args.get(di)?;
            let single = arg_single.get(di).copied().unwrap_or(false);
            self.resolve_const_word(text, tok, single)
                .map(|d| summarise_detail(&d))
        });

        // Fold range: the name token through the end of the last argument.
        let end = arg_tokens
            .last()
            .map_or(name_tok.span.end(), |t| t.span.end())
            .max(name_tok.span.end());
        let full_span = Span::new(name_tok.span.start(), end);

        // A registry definer called inside `proc ::ns::p {}` creates its symbol
        // in `::ns` (the proc's defining namespace), not `::` — the
        // command-resolution rule, not the lexical one.
        let ns_prefix = self.command_resolution_namespace(scope_path);
        let qualified = qualify(&ns_prefix, &name);

        let symbol = DefinedSymbol {
            original_name_input,
            name,
            qualified_name: qualified,
            kind: sym.kind,
            name_span: name_tok.span,
            full_span,
            detail,
        };
        self.retain_vendor_symbol_metadata(vendor_name, &symbol);
        self.retain_defined_symbol_metadata(advice, command_token, scope_path, &symbol);
        self.result.all_defined_symbols.push(symbol.clone());
        let path = scope_path.to_vec();
        if let Some(scope) = super::super::scope::scope_at_mut(&mut self.result.global_scope, &path)
        {
            scope.defined_symbols.push(symbol);
        }
    }

    fn retain_vendor_symbol_metadata(
        &mut self,
        vendor_name: Option<crate::signature_scan::vendor_name::VendorSourceNameOccurrence>,
        symbol: &DefinedSymbol,
    ) {
        if let Some(original) = vendor_name {
            let record = crate::signature_scan::vendor_name::VendorSourceDeclarationMetadata::new(
                &original,
                tcl_syntax::naming::VendorSourceNamePurpose::SourceName,
                symbol.clone(),
            );
            if !self
                .result
                .original_vendor_symbol_metadata
                .contains(&record)
            {
                self.result.original_vendor_symbol_metadata.push(record);
            }
        }
    }

    pub(super) fn defined_symbol_reporting_name(
        &self,
        original_name_input: Option<&crate::signature_scan::scope::SignatureSourceNameInput>,
        vendor_name: Option<&crate::signature_scan::vendor_name::VendorSourceNameOccurrence>,
        name_text: &str,
        name_tok: Token,
        name_single: bool,
        _scope_path: &[usize],
    ) -> Option<String> {
        if let Some(input) = original_name_input {
            std::str::from_utf8(input.bytes())
                .ok()
                .map(str::to_owned)
                .or_else(|| {
                    tcl_syntax::backslash::native_literal_source_word(
                        input.bytes(),
                        tcl_lexer::SourceChannel::Document,
                        self.lexer_config(),
                        input.policy().string_protocol(),
                    )
                })
        } else if let Some(original) = vendor_name {
            // Registry-selected declaration advice retains the original hosted
            // source spelling when materialisation is unavailable. The label
            // supplies no canonical units or runtime identity.
            let input = original.name_input();
            input
                .literal_units(tcl_syntax::naming::VendorSourceNamePurpose::SourceName)
                .and_then(|bytes| std::str::from_utf8(bytes).ok())
                .map(str::to_owned)
                .or_else(|| {
                    let span = input.original_word().content_span().ok()?;
                    std::str::from_utf8(input.source_image().bytes().get(span.as_range())?)
                        .ok()
                        .map(str::to_owned)
                })
        } else if self.result.allows_lexical_declaration_advice() {
            // Explicit logical-only declaration advice has no native recipe.
            self.resolve_const_word(name_text, name_tok, name_single)
        } else {
            None
        }
    }

    fn retain_defined_symbol_metadata(
        &mut self,
        advice: &crate::registry_invocation::OriginalSymbolDeclarationAdvice,
        command_token: Token,
        scope_path: &[usize],
        symbol: &DefinedSymbol,
    ) {
        if let (Some(input), Some(head)) = (
            symbol.original_name_input.as_ref(),
            self.original_static_source_name_at_span(command_token.span),
        ) && let Some(registry) = self.result.resolved_registry().or(self.registry.as_deref())
            && let Some(record) =
                crate::signature_scan::symbol_name::OriginalSourceSymbolDeclaration::new(
                    head,
                    input,
                    advice.symbol,
                    super::super::scope::scope_at(&self.result.global_scope, scope_path)
                        .and_then(|scope| scope.naming_scope.as_ref()),
                    advice.conditional_metadata.as_ref(),
                    symbol,
                    registry,
                )
            && !self.result.original_symbol_metadata.contains(&record)
        {
            self.result.original_symbol_metadata.push(record);
        }
    }

    fn registry_symbol_placement_is_valid(
        &self,
        advice: &crate::registry_invocation::OriginalSymbolDeclarationAdvice,
        args: &[String],
        arg_tokens: &[Token],
        arg_single: &[bool],
    ) -> bool {
        // A registry symbol definer that is top-level-only in iRules does not
        // become a definition merely because its invalid nested spelling was
        // parsed. In particular, nested event-handler syntax must not leak
        // into document/workspace symbol inventories. Keep this driven by the
        // placement trait shared with IRULE5006, never by a command name.
        let cmd_name = advice.command.as_str();
        if self.profile.is_irules()
            && self.body_depth > 0
            && advice
                .traits
                .contains(tcl_registry::Traits::IRULES_TOP_LEVEL_ONLY)
        {
            return false;
        }
        let sym = advice.symbol;
        // An iRules event symbol is a declaration only when the shared
        // source-aware owner accepts its complete top-level shape. In
        // particular, a bare or quoted body must not turn an otherwise
        // ordinary command into an outline event.
        if self.profile.is_irules() && sym.kind == tcl_registry::DefinedSymbolKind::Event {
            let words: Vec<&str> = args.iter().map(String::as_str).collect();
            let valid = self.body_depth == 0
                && self.registry.as_deref().is_some_and(|registry| {
                    tcl_registry::events::closed_braced_argument_words(
                        &self.source,
                        arg_tokens,
                        arg_single,
                    )
                    .as_deref()
                    .and_then(|closed| {
                        tcl_registry::events::IrulesDeclarationArguments::new(
                            &words, arg_tokens, arg_single, closed,
                        )
                    })
                    .and_then(|arguments| {
                        registry.irules_top_level_declaration_shape(cmd_name, arguments)
                    })
                    .is_some_and(|declaration| {
                        matches!(
                            declaration,
                            tcl_registry::events::IrulesTopLevelDeclaration::Event { .. }
                        )
                    })
                });
            if !valid {
                return false;
            }
        }
        true
    }

    /// Resolve a single literal or original scalar substitution through checked value provenance.
    pub(super) fn resolve_const_word(
        &self,
        text: &str,
        tok: Token,
        is_single: bool,
    ) -> Option<String> {
        if !is_single {
            return None;
        }
        match tok.kind {
            TokenType::Str | TokenType::Esc => Some(text.to_string()),
            TokenType::Var => self
                .lookup_logical_const_substitution(tok)
                .map(|(value, _)| value.to_owned())
                .or_else(|| self.frozen_value_word(tok)),
            _ => None,
        }
    }
}
