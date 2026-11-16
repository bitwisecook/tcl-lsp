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

//! The constant strings and interpreter paths a variable is known to hold, and the Logical lookups over them.

use super::super::state::Analyser;
use super::scope_at;
use tcl_lexer::{Span, Token, TokenType};

#[derive(Debug, Clone)]
pub(in super::super) struct LogicalConstStringProvenance {
    pub(super) image: tcl_lexer::SourceImage,
    pub(super) input: super::super::ResolvedAnalysisInput,
    pub(super) declaration: Span,
    pub(super) value: String,
    value_span: Span,
}

#[derive(Debug, Clone)]
pub(in super::super) struct LogicalInterpreterPathProvenance {
    pub(super) image: tcl_lexer::SourceImage,
    pub(super) input: super::super::ResolvedAnalysisInput,
    pub(super) declaration: Span,
    producer: Span,
    key: String,
}

impl Analyser {
    /// Record a constant string assignment for `var_name` in the
    /// scope at `scope_path`.
    pub fn set_const_string(
        &mut self,
        var_name: &str,
        value: String,
        value_span: Span,
        scope_path: &[usize],
    ) {
        let provenance = self.result.resolved_input.as_ref().and_then(|input| {
            let declaration = scope_at(&self.result.global_scope, scope_path)?
                .variables
                .get(var_name)?;
            let image = self
                .result
                .retained_command_realm()?
                .original_source_image()?
                .clone();
            (self.result.allows_lexical_declaration_advice() && declaration.link_target.is_none())
                .then(|| LogicalConstStringProvenance {
                    image,
                    input: input.clone(),
                    declaration: declaration.definition_span,
                    value: value.clone(),
                    value_span,
                })
        });
        let records = self
            .logical_const_string_provenance
            .entry(scope_path.to_vec())
            .or_default();
        if let Some(provenance) = provenance {
            records.insert(var_name.to_owned(), provenance);
        } else {
            records.remove(var_name);
        }
        self.const_strings
            .entry(scope_path.to_vec())
            .or_default()
            .insert(var_name.to_string(), (value, value_span));
        // Conditional writes cannot establish a value after the branch joins.
        if self.conditional_depth > 0 {
            self.nondominating_consts
                .entry(scope_path.to_vec())
                .or_default()
                .insert(var_name.to_string());
        } else if let Some(set) = self.nondominating_consts.get_mut(scope_path) {
            set.remove(var_name);
        }
    }

    /// Remove constant-value knowledge for `var_name` (re-assigned
    /// dynamically).
    pub fn clear_const_string(&mut self, var_name: &str, scope_path: &[usize]) {
        if let Some(map) = self.logical_const_string_provenance.get_mut(scope_path) {
            map.remove(var_name);
        }
        if let Some(map) = self.const_strings.get_mut(scope_path) {
            map.remove(var_name);
        }
        if let Some(set) = self.nondominating_consts.get_mut(scope_path) {
            set.remove(var_name);
        }
    }

    /// A dominating value from the exact declaration selected by Logical source advice.
    /// Linked cells and array elements require independent value provenance.
    pub(in super::super) fn lookup_logical_const_string_at(
        &self,
        written: &str,
        span: Span,
    ) -> Option<(&str, Span)> {
        let image = tcl_lexer::SourceImage::document(&self.source);
        let advice = self.result.logical_variable_scope_advice(
            &image,
            self.file_lexer_config(),
            span.start(),
        )?;
        let declaration = advice.variable_definition(written)?;
        if declaration.link_target.is_some() || !declaration.array_indices.is_empty() {
            return None;
        }
        let input = self.result.resolved_input.as_ref()?;
        let mut candidates =
            self.logical_const_string_provenance
                .iter()
                .flat_map(|(path, records)| {
                    records.iter().filter_map(move |(name, provenance)| {
                        let scope = scope_at(&self.result.global_scope, path)?;
                        let retained = scope.variables.get(name)?;
                        if !std::ptr::eq(retained, declaration)
                            || provenance.declaration != declaration.definition_span
                            || &provenance.input != input
                            || !self.result.matches_original_source_image(
                                &provenance.image,
                                self.file_lexer_config(),
                            )
                            || self
                                .nondominating_consts
                                .get(path)
                                .is_some_and(|names| names.contains(name))
                        {
                            return None;
                        }
                        let (value, value_span) = self.const_strings.get(path)?.get(name)?;
                        (value == &provenance.value
                            && *value_span == provenance.value_span
                            && value_span.end() < span.start())
                        .then_some((value.as_str(), *value_span))
                    })
                });
        let value = candidates.next()?;
        candidates.next().is_none().then_some(value)
    }

    /// A scalar substitution retains its complete original word and braced-name form.
    pub(in super::super) fn lookup_logical_const_substitution(
        &self,
        token: Token,
    ) -> Option<(&str, Span)> {
        if token.kind != TokenType::Var {
            return None;
        }
        let word = self.retained_value_word(token)?;
        let arena = word.executable_parts();
        let [component] = arena.list(arena.root()) else {
            return None;
        };
        let (name, span) = self.logical_variable_value_component(arena, component)?;
        self.lookup_logical_const_string_at(&name, span)
    }

    pub(in super::super) fn retained_value_word(
        &self,
        token: Token,
    ) -> Option<tcl_lexer::NativeWord> {
        let config = self.file_lexer_config();
        let image = crate::registry_invocation::source_structure::original_query_document_image(
            &self.source,
            &self.result,
            config,
        );
        self.result
            .matches_original_source_image(&image, config)
            .then_some(())?;
        self.result
            .retained_command_realm()?
            .original_written_word_at_span_in_source(&image, token.span, config)
            .map(|(word, _)| word)
    }

    pub(in super::super) fn logical_variable_value_component(
        &self,
        arena: &tcl_lexer::ExecutablePartArena,
        component: &tcl_lexer::SpannedExecutablePart,
    ) -> Option<(String, Span)> {
        if !self.result.allows_lexical_declaration_advice()
            || !self
                .result
                .matches_original_source_image(arena.image(), arena.config())
            || arena.image().bytes() != self.source.as_bytes()
        {
            return None;
        }
        let reference = tcl_lexer::word_parts::scan_var_ref(
            arena.image().bytes(),
            usize::try_from(component.span.start()).ok()?,
            arena.config(),
        )
        .ok()??;
        (u32::try_from(reference.next).ok()? == component.span.end()).then_some(())?;
        let span = reference.source_span(
            arena.image().bytes(),
            usize::try_from(component.span.start()).ok()?,
            0,
        )?;
        let root = crate::signature_scan::variable_name::SignatureSourceVariableRoot::from_original_logical_source_executable(
            arena, arena.image(), arena.config(), span, self.word_rules(), self.declaration_name_policy()?,
        )?;
        if root.is_separate_array_root() {
            return None;
        }
        Some((root.display()?.to_owned(), span))
    }

    pub(in super::super) fn lookup_logical_const_component(
        &self,
        arena: &tcl_lexer::ExecutablePartArena,
        component: &tcl_lexer::SpannedExecutablePart,
    ) -> Option<String> {
        let (name, span) = self.logical_variable_value_component(arena, component)?;
        self.lookup_logical_const_string_at(&name, span)
            .map(|(value, _)| value.to_owned())
    }

    pub(in super::super) fn resolve_logical_value_word(&self, token: Token) -> Option<String> {
        let word = self.retained_value_word(token)?;
        let bytes = tcl_syntax::word_rules::original_word_unicode_value_with_variables(
            &word,
            |arena, component| {
                self.lookup_logical_const_component(arena, component)
                    .map(String::into_bytes)
            },
        )?;
        String::from_utf8(bytes).ok()
    }

    /// The value the interpretation froze for the whole word at `token`, on
    /// a profile whose words it evaluates. A word it never reached, or
    /// reached with two values, has none.
    pub(in super::super) fn frozen_value_word(&self, token: Token) -> Option<String> {
        let config = self.file_lexer_config();
        let image = crate::registry_invocation::source_structure::original_query_document_image(
            &self.source,
            &self.result,
            config,
        );
        self.result
            .matches_original_source_image(&image, config)
            .then_some(())?;
        self.result
            .retained_command_realm()?
            .frozen_written_word_value_at_span_in_source(&image, token.span, config)
    }

    /// Record `set VAR [interp create ...]`'s resolved interpreter-domain
    /// `key` as `var_name`'s value in the scope at `scope_path` — the
    /// interpreter-value-flow analogue of
    /// [`Self::set_const_string`], scope-chain-aware for the same reason:
    /// two unrelated procs binding the same variable name to different
    /// interpreters must never collide.
    pub fn set_interp_var_binding(&mut self, var_name: &str, key: String, scope_path: &[usize]) {
        self.logical_interp_path_provenance
            .entry(scope_path.to_vec())
            .or_default()
            .remove(var_name);
        self.interp_var_bindings
            .entry(scope_path.to_vec())
            .or_default()
            .insert(var_name.to_string(), key);
    }

    pub(in super::super) fn set_original_logical_interp_var_binding(
        &mut self,
        var_name: &str,
        key: String,
        producer: Token,
        scope_path: &[usize],
    ) {
        self.set_interp_var_binding(var_name, key.clone(), scope_path);
        let provenance = (|| {
            if !self.result.allows_lexical_declaration_advice() || self.conditional_depth != 0 {
                return None;
            }
            let declaration = scope_at(&self.result.global_scope, scope_path)?
                .variables
                .get(var_name)?;
            if declaration.link_target.is_some() || !declaration.array_indices.is_empty() {
                return None;
            }
            self.retained_value_word(producer)?;
            Some(LogicalInterpreterPathProvenance {
                image: self
                    .result
                    .retained_command_realm()?
                    .original_source_image()?
                    .clone(),
                input: self.result.resolved_input.as_ref()?.clone(),
                declaration: declaration.definition_span,
                producer: producer.span,
                key,
            })
        })();
        if let Some(provenance) = provenance {
            self.logical_interp_path_provenance
                .entry(scope_path.to_vec())
                .or_default()
                .insert(var_name.to_owned(), provenance);
        }
    }

    pub(in super::super) fn lookup_logical_interp_path(&self, token: Token) -> Option<String> {
        let word = self.retained_value_word(token)?;
        let arena = word.executable_parts();
        let [component] = arena.list(arena.root()) else {
            return None;
        };
        let (name, span) = self.logical_variable_value_component(arena, component)?;
        let image = tcl_lexer::SourceImage::document(&self.source);
        let advice = self.result.logical_variable_scope_advice(
            &image,
            self.file_lexer_config(),
            span.start(),
        )?;
        let declaration = advice.variable_definition(&name)?;
        if declaration.link_target.is_some() || !declaration.array_indices.is_empty() {
            return None;
        }
        let input = self.result.resolved_input.as_ref()?;
        let mut found = self
            .logical_interp_path_provenance
            .iter()
            .flat_map(|(path, records)| {
                records.iter().filter_map(move |(written, provenance)| {
                    let selected = scope_at(&self.result.global_scope, path)?
                        .variables
                        .get(written)?;
                    if !std::ptr::eq(selected, declaration)
                        || provenance.declaration != declaration.definition_span
                        || &provenance.input != input
                        || provenance.producer.end() >= span.start()
                        || !self.result.matches_original_source_image(
                            &provenance.image,
                            self.file_lexer_config(),
                        )
                        || self.interp_var_bindings.get(path)?.get(written)? != &provenance.key
                    {
                        return None;
                    }
                    Some(provenance.key.as_str())
                })
            });
        let key = found.next()?;
        found.next().is_none().then(|| key.to_owned())
    }

    /// Remove interpreter-binding knowledge for `var_name` (reassigned to
    /// something else — the reassigned value stops resolving as an
    /// interpreter handle, exactly as [`Self::clear_const_string`] does
    /// for a reassigned constant string).
    pub fn clear_interp_var_binding(&mut self, var_name: &str, scope_path: &[usize]) {
        if let Some(records) = self.logical_interp_path_provenance.get_mut(scope_path) {
            records.remove(var_name);
        }
        if let Some(map) = self.interp_var_bindings.get_mut(scope_path) {
            map.remove(var_name);
        }
    }
}
