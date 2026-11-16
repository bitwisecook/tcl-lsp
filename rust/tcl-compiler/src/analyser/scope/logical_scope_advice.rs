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

//! `LogicalVariableScopeAdvice`: the declaration tables Logical source advice may read at an offset, and the read-only dictionary keys it infers.

use super::super::state::Analyser;
use super::super::types::{Scope, ScopeKind, VarDef};
use super::command_namespaces::innermost_containing_child;
use super::{scope_at, scope_at_mut};
use std::collections::HashMap;
use tcl_lexer::Span;

fn original_dictionary_key_spans(
    value: &str,
    original: Option<Span>,
) -> HashMap<String, Option<Span>> {
    let mut original_keys = std::collections::HashMap::new();
    if let Some(content) = original {
        let mut offset = 0;
        let mut ordinal = 0;
        while let Ok(Some(element)) = tcl_syntax::list::find_element(value, offset) {
            if ordinal % 2 == 0 && element.literal {
                let key = &value[element.value.clone()];
                let span = u32::try_from(element.value.start)
                    .ok()
                    .zip(u32::try_from(element.value.end).ok())
                    .and_then(|(start, end)| {
                        Some(Span::new(
                            content.start().checked_add(start)?,
                            content.start().checked_add(end)?,
                        ))
                    });
                original_keys
                    .entry(key.to_owned())
                    .and_modify(|entry| *entry = None)
                    .or_insert(span);
            }
            ordinal += 1;
            offset = element.next;
        }
    }
    original_keys
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum LogicalVariableScopeKind {
    Root,
    Namespace,
    UnknownNamespace,
    Local,
    UnknownCaller,
}

/// Eligible variable declaration scopes from the current Logical source.
/// Cell identities, entered links and editable symbols require their own owners.
#[derive(Debug)]
pub struct LogicalVariableScopeAdvice<'a> {
    pub(super) analysis: &'a super::super::types::AnalysisResult,
    pub(super) scopes: Vec<&'a Scope>,
    pub(super) eligible: Vec<usize>,
    pub(super) kind: LogicalVariableScopeKind,
    pub(super) namespace: Option<String>,
    pub(super) image: tcl_lexer::SourceImage,
    pub(super) config: tcl_lexer::LexerConfig,
}

/// A dictionary key inferred from an original checked value, without a variable declaration span.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReadonlyDictionaryKeyVariable {
    pub(super) name: String,
    pub(super) image: tcl_lexer::SourceImage,
    pub(super) input: super::super::ResolvedAnalysisInput,
    receiver_scope: Vec<usize>,
    original_key: Option<Span>,
    original_declaration: Option<Span>,
}

impl ReadonlyDictionaryKeyVariable {
    /// Source-value label, independently of entered variable storage.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Exact original data-key interior, when unchanged literal correspondence exists.
    /// This is source geometry and supplies no variable rename permission.
    #[must_use]
    pub const fn original_key_span(&self) -> Option<Span> {
        self.original_key
    }
}

impl<'a> LogicalVariableScopeAdvice<'a> {
    /// Root declaration table, independently of its lookup eligibility.
    #[must_use]
    pub fn root(&self) -> &'a Scope {
        &self.analysis.global_scope
    }

    /// Eligible source tables in outer-to-inner order.
    pub fn eligible_scopes(&self) -> impl DoubleEndedIterator<Item = &'a Scope> + '_ {
        self.eligible.iter().map(|&index| self.scopes[index])
    }

    /// Fully rooted candidates from other authentic namespace declaration tables.
    /// Each original declaration supplies its own retained namespace coordinate;
    /// reporting scope names cannot construct insertion text or establish a cell.
    #[must_use]
    pub fn cross_namespace_variable_names(&self) -> Vec<String> {
        let mut pending = vec![self.root()];
        let mut names = Vec::new();
        while let Some(scope) = pending.pop() {
            pending.extend(&scope.children);
            if !matches!(scope.kind, ScopeKind::Global | ScopeKind::Namespace)
                || self
                    .eligible_scopes()
                    .any(|current| std::ptr::eq(current, scope))
            {
                continue;
            }
            for (name, declaration) in &scope.variables {
                let Some(original) = self.analysis.logical_variable_scope_advice(
                    &self.image,
                    self.config,
                    declaration.definition_span.start(),
                ) else {
                    continue;
                };
                if !matches!(
                    original.kind,
                    LogicalVariableScopeKind::Root | LogicalVariableScopeKind::Namespace
                ) || !original
                    .eligible_scopes()
                    .any(|current| std::ptr::eq(current, scope))
                {
                    continue;
                }
                let Some(namespace) = original.namespace() else {
                    continue;
                };
                let coordinate = if crate::naming::unroot_rooted_key(name).is_some() {
                    name.clone()
                } else {
                    tcl_syntax::naming::rooted_member_key(namespace, name)
                };
                if self
                    .variable_definition(&coordinate)
                    .is_some_and(|selected| std::ptr::eq(selected, declaration))
                {
                    names.push(coordinate);
                }
            }
        }
        names.sort_unstable();
        names.dedup();
        names
    }

    /// Inferred source names from the eligible Logical tables, without editable declarations.
    #[must_use]
    pub fn readonly_variable_names(&self) -> Vec<&'a str> {
        self.eligible_scopes()
            .flat_map(|scope| {
                scope
                    .readonly_dictionary_keys
                    .values()
                    .filter(move |key| {
                        key.image == self.image
                            && self.analysis.resolved_input.as_ref() == Some(&key.input)
                            && scope_at(&self.analysis.global_scope, &key.receiver_scope)
                                .is_some_and(|retained| std::ptr::eq(retained, scope))
                    })
                    .map(ReadonlyDictionaryKeyVariable::name)
            })
            .collect()
    }

    /// Inferred names held by the independent root table under current source currency.
    #[must_use]
    pub fn readonly_root_variable_names(&self) -> Vec<&'a str> {
        self.root()
            .readonly_dictionary_keys
            .values()
            .filter(|key| {
                key.image == self.image
                    && self.analysis.resolved_input.as_ref() == Some(&key.input)
                    && key.receiver_scope.is_empty()
            })
            .map(ReadonlyDictionaryKeyVariable::name)
            .collect()
    }

    /// Known source namespace; an unknown caller cannot supply one.
    #[must_use]
    pub fn namespace(&self) -> Option<&str> {
        self.namespace.as_deref()
    }

    /// A declaration table selected by this source frame's naming policy.
    /// The result establishes no entered storage cell or editable identity.
    #[must_use]
    pub fn variable_definition(&self, name: &str) -> Option<&'a VarDef> {
        if crate::naming::is_qualified(name.as_bytes())
            && let Some(qualified) = self.namespace_variable_coordinate(name)
        {
            let (holder, tail) = crate::naming::key_holder_and_tail(&qualified);
            return self.namespace_variable_definition(holder, tail);
        }
        self.variable_declaration_scope(name)?.variables.get(name)
    }

    fn namespace_variable_definition(&self, holder: &str, tail: &str) -> Option<&'a VarDef> {
        fn collect<'a>(
            advice: &LogicalVariableScopeAdvice<'a>,
            scope: &'a Scope,
            holder: &str,
            tail: &str,
            found: &mut Vec<&'a VarDef>,
        ) {
            if matches!(scope.kind, ScopeKind::Global | ScopeKind::Namespace)
                && let Some(definition) = scope.variables.get(tail)
                && let Some(frame) = advice.analysis.logical_variable_scope_advice(
                    &advice.image,
                    advice.config,
                    definition.definition_span.start(),
                )
                && matches!(
                    frame.kind,
                    LogicalVariableScopeKind::Root | LogicalVariableScopeKind::Namespace
                )
                && frame.namespace() == Some(holder)
                && frame
                    .eligible_scopes()
                    .any(|eligible| std::ptr::eq(eligible, scope))
            {
                found.push(definition);
            }
            for child in &scope.children {
                collect(advice, child, holder, tail, found);
            }
        }
        let mut found = Vec::new();
        collect(self, self.root(), holder, tail, &mut found);
        let [definition] = found.as_slice() else {
            return None;
        };
        Some(*definition)
    }

    fn readonly_key_is_current(&self, scope: &Scope, name: &str) -> bool {
        scope.readonly_dictionary_keys.get(name).is_some_and(|key| {
            key.image == self.image
                && self.analysis.resolved_input.as_ref() == Some(&key.input)
                && scope_at(&self.analysis.global_scope, &key.receiver_scope)
                    .is_some_and(|retained| std::ptr::eq(retained, scope))
        })
    }

    pub(super) fn variable_declaration_scope(&self, name: &str) -> Option<&'a Scope> {
        let current = || {
            self.eligible_scopes().rev().find(|scope| {
                scope.variables.contains_key(name)
                    || (self.readonly_key_is_current(scope, name)
                        && scope.readonly_dictionary_keys[name]
                            .original_declaration
                            .is_none())
            })
        };
        if !crate::naming::is_qualified(name.as_bytes()) && self.is_namespace_frame() {
            return crate::naming::variable_namespace_lookup_bases(
                false,
                self,
                self.namespace_global_fallback(),
            )
            .find_map(|base| match base {
                crate::naming::VariableLookupBase::Context(_) => current(),
                crate::naming::VariableLookupBase::Root => self
                    .root()
                    .variables
                    .contains_key(name)
                    .then(|| self.root()),
            });
        }
        current()
    }

    /// Namespace variable coordinate from this checked Logical source frame.
    /// Local names and relative names in unknown callers supply no coordinate.
    #[must_use]
    pub fn namespace_variable_coordinate(&self, written: &str) -> Option<String> {
        let namespace = if crate::naming::unroot_rooted_key(written).is_some() {
            "::"
        } else {
            if !crate::naming::is_qualified(written.as_bytes())
                && !matches!(
                    self.kind,
                    LogicalVariableScopeKind::Root | LogicalVariableScopeKind::Namespace
                )
            {
                return None;
            }
            self.namespace()?
        };
        Some(crate::naming::qualify(namespace, written))
    }

    /// Namespace tables use their selected root alternative independently.
    #[must_use]
    pub fn is_namespace_frame(&self) -> bool {
        self.kind == LogicalVariableScopeKind::Namespace
    }

    /// Root names require explicit rooted insertion outside a root frame.
    #[must_use]
    pub fn root_names_require_qualification(&self) -> bool {
        match self.kind {
            LogicalVariableScopeKind::Root => false,
            LogicalVariableScopeKind::Namespace => !self.analysis.ns_var_global_fallback(),
            LogicalVariableScopeKind::UnknownNamespace
            | LogicalVariableScopeKind::Local
            | LogicalVariableScopeKind::UnknownCaller => true,
        }
    }

    /// Fallback policy from the same independently retained Logical input.
    #[must_use]
    pub fn namespace_global_fallback(&self) -> bool {
        self.analysis.ns_var_global_fallback()
    }
}

impl super::super::types::AnalysisResult {
    /// A complete retained Logical word reproduces this readonly input.
    /// Native words, computed values and unretained source vectors cannot use this correspondence.
    #[must_use]
    pub fn logical_original_word_input_matches_source(
        &self,
        image: &tcl_lexer::SourceImage,
        config: tcl_lexer::LexerConfig,
        span: Span,
        input: &crate::signature_scan::scope::SignatureSourceNameInput,
    ) -> bool {
        use crate::signature_scan::scope::SignatureSourceNameInput;
        let SignatureSourceNameInput::OriginalWord(retained) = input else {
            return false;
        };
        if !self.allows_lexical_declaration_advice()
            || !self.matches_original_source_image(image, config)
        {
            return false;
        }
        let Some(resolved) = self.resolved_input.as_ref() else {
            return false;
        };
        if !resolved.has_logical_source_name_context() {
            return false;
        }
        let Some(policy) = tcl_registry::InvocationDialect::of_profile(resolved.analyser_profile())
            .authored_name_policy()
        else {
            return false;
        };
        if input.policy() != policy {
            return false;
        }
        let Some((word, _)) = self
            .retained_command_realm()
            .and_then(|realm| realm.original_written_word_at_span_in_source(image, span, config))
        else {
            return false;
        };
        let rules = tcl_syntax::word_rules::WordValueRules::from_config(&config);
        crate::signature_scan::scope::SignatureSourceNameKey::from_original_logical_source_word(
            &word, rules, policy,
        )
        .as_ref()
            == Some(retained)
    }

    /// Current source scope eligibility for independently retained Logical advice.
    /// Original Native/hosted bodies and readonly alias receipts cannot issue it.
    #[must_use]
    pub fn logical_variable_scope_advice(
        &self,
        image: &tcl_lexer::SourceImage,
        config: tcl_lexer::LexerConfig,
        offset: u32,
    ) -> Option<LogicalVariableScopeAdvice<'_>> {
        if !self.allows_lexical_declaration_advice()
            || !self.matches_original_source_image(image, config)
            || usize::try_from(offset).ok()? > image.bytes().len()
        {
            return None;
        }
        let registry = self.resolved_registry()?;
        let mut scopes = vec![&self.global_scope];
        let mut cursor = &self.global_scope;
        while let Some(child) = innermost_containing_child(cursor, offset) {
            scopes.push(child);
            cursor = child;
        }
        for scope in &scopes {
            if scope.is_independent_interpreter_source_body() {
                return None;
            }
            match (
                &self.global_scope.original_interpreter_source_domain,
                &scope.original_interpreter_source_domain,
            ) {
                (None, None) => {}
                (Some(root), Some(current)) if std::sync::Arc::ptr_eq(root, current) => {}
                _ => return None,
            }
        }
        let mut eligible = vec![0];
        let mut kind = LogicalVariableScopeKind::Root;
        let mut namespace = Some("::".to_owned());
        for (index, scope) in scopes.iter().enumerate().skip(1) {
            match scope.kind {
                ScopeKind::Global => eligible.push(index),
                ScopeKind::Namespace => {
                    eligible.clear();
                    eligible.push(index);
                    kind = LogicalVariableScopeKind::Namespace;
                    namespace = if namespace.is_none()
                        && tcl_syntax::naming::unroot_rooted_key(&scope.name).is_none()
                    {
                        None
                    } else {
                        scope.naming_scope.as_ref().and_then(
                            crate::signature_scan::scope::SignatureNamespaceScope::display,
                        )
                    };
                    if namespace.is_none() {
                        kind = LogicalVariableScopeKind::UnknownNamespace;
                    }
                    if namespace
                        .as_deref()
                        .is_some_and(|name| tcl_syntax::naming::unroot_rooted_key(name) == Some(""))
                    {
                        eligible.insert(0, 0);
                        kind = LogicalVariableScopeKind::Root;
                    }
                }
                ScopeKind::Proc | ScopeKind::Method => {
                    eligible.clear();
                    eligible.push(index);
                    kind = LogicalVariableScopeKind::Local;
                    namespace = if namespace.is_none()
                        && tcl_syntax::naming::unroot_rooted_key(&scope.name).is_none()
                    {
                        None
                    } else {
                        scope.naming_scope.as_ref().and_then(
                            crate::signature_scan::scope::SignatureNamespaceScope::display,
                        )
                    };
                }
                ScopeKind::Uplevel => {
                    let level = tcl_registry::FrameLevel::parse_in(&scope.name, registry);
                    if level.is_some_and(tcl_registry::FrameLevel::is_current_frame) {
                        eligible.push(index);
                    } else if level.is_some_and(tcl_registry::FrameLevel::is_global_frame) {
                        eligible.clear();
                        eligible.extend([0, index]);
                        kind = LogicalVariableScopeKind::Root;
                        namespace = Some("::".to_owned());
                    } else {
                        eligible.clear();
                        eligible.push(index);
                        kind = LogicalVariableScopeKind::UnknownCaller;
                        namespace = None;
                    }
                }
            }
        }
        Some(LogicalVariableScopeAdvice {
            analysis: self,
            scopes,
            eligible,
            kind,
            namespace,
            image: image.clone(),
            config,
        })
    }
}

impl Analyser {
    pub(in super::super) fn retain_readonly_dictionary_keys(
        &mut self,
        value: &str,
        value_span: Span,
        write_span: Span,
        receiver_scope: &[usize],
    ) {
        let Ok(elements) = tcl_syntax::list::split_list_in(value, self.file_lexer_config().escapes)
        else {
            return;
        };
        if elements.len() % 2 != 0 {
            return;
        }
        let Some(input) = self.result.resolved_input.clone() else {
            return;
        };
        let Some(image) = self
            .result
            .retained_command_realm()
            .and_then(|realm| realm.original_source_image())
            .cloned()
        else {
            return;
        };
        let config = self.file_lexer_config();
        let original = self
            .result
            .retained_command_realm()
            .and_then(|realm| {
                realm.original_written_word_at_span_in_source(&image, value_span, config)
            })
            .and_then(|(word, _)| {
                let content = word.content_span().ok()?;
                (image.bytes().get(content.as_range()) == Some(value.as_bytes())).then_some(content)
            });
        let original_keys = original_dictionary_key_spans(value, original);
        let declarations = elements
            .as_chunks::<2>()
            .0
            .iter()
            .map(|pair| {
                let name = pair[0].to_string();
                let declaration = self
                    .result
                    .logical_variable_scope_advice(&image, config, write_span.start())
                    .and_then(|advice| advice.variable_definition(&name))
                    .map(|definition| definition.definition_span);
                (name, declaration)
            })
            .collect::<std::collections::HashMap<_, _>>();
        for declaration in declarations.values().flatten() {
            for (path, records) in &mut self.logical_const_string_provenance {
                let changed = records
                    .iter()
                    .filter(|(_, record)| record.declaration == *declaration)
                    .map(|(name, _)| name.clone())
                    .collect::<Vec<_>>();
                for name in changed {
                    records.remove(&name);
                    if let Some(values) = self.const_strings.get_mut(path) {
                        values.remove(&name);
                    }
                }
            }
        }
        for declaration in declarations.values().flatten() {
            for (path, records) in &mut self.logical_interp_path_provenance {
                let changed = records
                    .iter()
                    .filter(|(_, record)| record.declaration == *declaration)
                    .map(|(name, _)| name.clone())
                    .collect::<Vec<_>>();
                for name in changed {
                    records.remove(&name);
                    if let Some(values) = self.interp_var_bindings.get_mut(path) {
                        values.remove(&name);
                    }
                }
            }
        }
        let Some(scope) = scope_at_mut(&mut self.result.global_scope, receiver_scope) else {
            return;
        };
        for pair in elements.as_chunks::<2>().0 {
            let name = pair[0].to_string();
            scope.readonly_dictionary_keys.insert(
                name.clone(),
                ReadonlyDictionaryKeyVariable {
                    original_key: original_keys.get(&name).copied().flatten(),
                    original_declaration: declarations.get(&name).copied().flatten(),
                    name,
                    image: image.clone(),
                    input: input.clone(),
                    receiver_scope: receiver_scope.to_vec(),
                },
            );
        }
    }
}
