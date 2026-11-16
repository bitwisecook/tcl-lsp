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

//! `define_var`, the alias link targets of a local and the W215 unreachable-name warning.

use super::super::state::Analyser;
use super::super::types::VarDef;
use super::{scope_at, scope_at_mut};
use tcl_core_types::DiagCode;
use tcl_lexer::{Span, Token, TokenType};

pub(in super::super) struct VariableLinkTarget {
    pub(in super::super) target: String,
    pub(in super::super) target_span: Span,
    pub(in super::super) local_span: Span,
    pub(in super::super) original_target_namespace:
        Option<crate::signature_scan::scope::SignatureNamespaceScope>,
    pub(in super::super) original_target_receiver:
        crate::signature_scan::variable_symbol::OriginalVariableSymbolReceiver,
}

impl Analyser {
    /// Record a variable definition in the scope at `scope_path`.
    ///
    /// New definitions are inserted into the scope's `variables` map
    /// and registered into ``result.all_variables`` keyed by
    /// ``"<scope_name>::<base_name>"``.  Re-defining an existing
    /// variable does not overwrite — it only escalates the
    /// `warn_if_unused` flag.
    ///
    /// `tok` is the token of the word that **spells the name**, and its kind
    /// decides how that spelling reads: a brace-quoted word (`TokenType::Str`
    /// — `set {$n} 1`, `foreach {$a} …`) substitutes nothing, so its content
    /// is the literal name, `$` and all.  Tcl keeps `{$n}` and `n` apart
    /// (tclsh 9.0.4 / 8.6.14: `set {$n} v; info exists {$n}` → 1 while
    /// `info exists n` → 0); normalising the `$` away collapsed them into one
    /// `VarDef`, so Find-References answered for both and a rename of `n`
    /// rewrote the `{$n}` word. The array-element suffix still
    /// comes off — `{arr($i)}` is element `$i` of the array `arr`.
    pub fn define_var(
        &mut self,
        name: &str,
        tok: Token,
        scope_path: &[usize],
        warn_if_unused: bool,
        definition_span: Option<Span>,
    ) {
        let braced_literal = tok.kind == TokenType::Str;
        let base_name = crate::naming::normalise_var_name_braced(name, braced_literal);
        if base_name.is_empty() {
            return;
        }
        let base_owned = base_name.to_string();
        let span = definition_span.unwrap_or(tok.span);
        // A structural rebind (the per-item isolated pass re-binding params /
        // instance vars) records no occurrence: the shell walk already did,
        // and the rebind's placeholder token would add a phantom zero-width
        // ref for a `::`-qualified parameter name.
        if !self.structural_rebind {
            self.record_qualified_var_ref(base_name, span, scope_path, false, true);
            // Isolated per-item body: a definition landing in the body's own
            // (throwaway) root scope is a fixed global cell the whole-file
            // walk would have written into the *document's* global scope, so
            // capture it for the graft to replay there — the write-side twin
            // of `capture_global_reads`.
            if scope_path.is_empty()
                && let Some(captured) = self.capture_global_defs.as_mut()
            {
                captured.push((name.to_string(), tok, span));
            }
        }
        // A `set arr(idx) …` definition records the element index on the array.
        let element = crate::naming::split_array_name_braced(name, braced_literal)
            .1
            .filter(|e| !e.is_empty())
            .map(ToString::to_string);

        let path = scope_path.to_vec();
        // First definition of this variable → the W215 reachability check
        // fires here.
        // Done with a short immutable borrow before the mutable scope handle.
        let is_first_def = scope_at(&self.result.global_scope, &path)
            .is_some_and(|s| !s.variables.contains_key(base_name));
        if is_first_def && !self.structural_rebind {
            self.emit_w215_unreachable_name(base_name, element.as_deref(), tok, span, &path);
        }
        let Some(scope) = scope_at_mut(&mut self.result.global_scope, &path) else {
            return;
        };

        if let Some(existing) = scope.variables.get_mut(&base_owned) {
            // Re-definition (`set x` twice) does not overwrite the original
            // declaration span, but its span is recorded as a reference so
            // find-references / rename see every assignment, and it escalates
            // the unused flag.  Array indices accumulate.
            //
            // The last-reference check makes re-definition idempotent per
            // token span: a write site is recorded once even when both its
            // dedicated handler (`incr` / `append` / `dict update`) and the
            // registry `VarWrite`-role walk (`handle_var_binding_command`)
            // bind it — the two run back-to-back for the same word, and a
            // duplicated reference becomes a duplicated (hence overlapping)
            // rename edit downstream.
            if span != existing.definition_span && existing.references.last() != Some(&span) {
                existing.references.push(span);
            }
            if warn_if_unused {
                existing.warn_if_unused = true;
            }
            if let Some(e) = element {
                existing.array_indices.insert(e);
            }
            if let Some(global) =
                self.result
                    .all_variables
                    .get_mut(&tcl_syntax::naming::scope_variable_key(
                        &scope.name,
                        &base_owned,
                    ))
            {
                *global = scope.variables[&base_owned].clone();
            }
        } else {
            let mut indices = std::collections::BTreeSet::new();
            if let Some(e) = element {
                indices.insert(e);
            }
            let var = VarDef {
                name: base_owned.clone(),
                definition_span: span,
                references: Vec::new(),
                warn_if_unused,
                array_indices: indices,
                link_target: None,
                link_target_span: None,
            };
            scope
                .original_variable_declaration_spans
                .insert(base_owned.clone(), span);
            scope.variables.insert(base_owned.clone(), var.clone());
            let key = tcl_syntax::naming::scope_variable_key(&scope.name, &base_owned);
            self.result.all_variables.insert(key, var);
        }
    }

    /// Record that the local `name` in the scope at `scope_path` aliases the
    /// namespace/global cell `target` (a `global` / `variable` / `namespace
    /// upvar` binding), so Find-References / Rename can unify every alias of the
    /// same cell.  A no-op if the variable wasn't defined (call after
    /// [`Self::define_var`]).
    ///
    /// `target_span` is the span of the **word that names the cell**, which is
    /// not always the declaration word: `variable v` / `global ::ns::v` name
    /// the cell with the very word that introduces the local alias, but
    /// `namespace upvar ::ns v local` / `upvar #0 ::ns::v local` name it one
    /// word earlier.  Recording it is what lets a rename of the cell rewrite
    /// the right word — see [`VarDef::link_target_span`].
    pub(in super::super) fn set_var_link_target(
        &mut self,
        name: &str,
        scope_path: &[usize],
        target: VariableLinkTarget,
    ) {
        let VariableLinkTarget {
            target,
            target_span,
            local_span,
            original_target_namespace,
            original_target_receiver,
        } = target;
        let _ = self.record_original_variable_alias_target(
            local_span,
            target_span,
            original_target_namespace,
            original_target_receiver,
        );
        self.set_var_link_report(name, scope_path, target, target_span);
    }

    pub(in super::super) fn set_var_link_report(
        &mut self,
        name: &str,
        scope_path: &[usize],
        target: String,
        target_span: Span,
    ) {
        let base = crate::naming::normalise_var_name(name).to_string();
        if let Some(scope) = scope_at_mut(&mut self.result.global_scope, scope_path)
            && let Some(v) = scope.variables.get_mut(&base)
        {
            v.link_target = Some(target);
            v.link_target_span = Some(target_span);
        }
    }

    pub(in super::super) fn record_original_variable_alias_target(
        &mut self,
        local_span: Span,
        target_span: Span,
        original_target_namespace: Option<crate::signature_scan::scope::SignatureNamespaceScope>,
        original_target_receiver: crate::signature_scan::variable_symbol::OriginalVariableSymbolReceiver,
    ) -> Option<crate::signature_scan::variable_symbol::SignatureSourceVariableOccurrence> {
        self.result
            .original_variable_symbols
            .retain(|site| site.span() != local_span && site.span() != target_span);
        self.result.original_variable_alias_sites.push(local_span);
        self.result.original_variable_alias_sites.push(target_span);
        #[cfg(debug_assertions)]
        if tcl_dialect::trace_env::var_os("TCL_LSP_TRACE_ALIAS_TEMPLATE_SOURCE").is_some() {
            eprintln!(
                "ALIAS_SOURCE_LINK local={} remote={} namespace={} input={}",
                local_span.start(),
                target_span.start(),
                original_target_namespace.is_some(),
                self.original_qualified_variable_input(target_span, false)
                    .is_some()
            );
        }
        let original = original_target_namespace.and_then(|namespace| {
            let name = self.original_qualified_variable_input(target_span, false)?;
            let input = self.resolved_analysis_input();
            if input.has_logical_source_name_context() {
                let scope = self.head_identities.original_logical_variable_scope_at(
                    &input, &tcl_lexer::SourceImage::document(&self.source), target_span, &name,
                )?;
                return crate::signature_scan::variable_symbol::SignatureSourceVariableOccurrence::from_logical_alias_target(
                    target_span, name, namespace, original_target_receiver, scope,
                );
            }
            crate::signature_scan::variable_symbol::SignatureSourceVariableOccurrence::from_original_input(
                target_span, name, namespace,
                crate::signature_scan::variable_symbol::OriginalVariableSymbolPurpose::NamespaceStorage, original_target_receiver, true,
            )
        });
        if let Some(site) = original.as_ref() {
            if let Some(receipt) = self.original_variable_alias_receipt(local_span, site) {
                self.result
                    .original_variable_name_unknowns
                    .retain(|&span| span != local_span && span != target_span);
                if !self
                    .result
                    .original_variable_alias_receipts
                    .contains(&receipt)
                {
                    self.result.original_variable_alias_receipts.push(receipt);
                }
            }
            self.result.original_variable_symbols.push(site.clone());
        }

        original
    }

    pub(in super::super) fn define_original_namespace_alias_target(
        &mut self,
        occurrence: &crate::signature_scan::variable_symbol::SignatureSourceVariableOccurrence,
    ) {
        let Some(key) = occurrence.symbol().checked_c_namespace_key() else {
            return;
        };
        let span = occurrence.span();
        if !occurrence.is_declaration()
            || span.is_empty()
            || !self.result.original_variable_symbols.contains(occurrence)
        {
            return;
        }
        let input = occurrence.original_name_input();
        let selected = input
            .policy()
            .recipe()
            .combined_variable_input(input.bytes());
        let element = selected
            .element()
            .and_then(|element| std::str::from_utf8(element.selected()).ok())
            .filter(|element| !element.is_empty())
            .map(str::to_owned);
        let scope = &mut self.result.global_scope;
        if let Some(variable) = scope.variables.get_mut(&key) {
            if variable.definition_span != span && variable.references.last() != Some(&span) {
                variable.references.push(span);
            }
            if let Some(element) = element {
                variable.array_indices.insert(element);
            }
        } else {
            let variable = VarDef {
                name: key.clone(),
                definition_span: span,
                references: Vec::new(),
                warn_if_unused: false,
                array_indices: element.into_iter().collect(),
                link_target: None,
                link_target_span: None,
            };
            scope
                .original_variable_declaration_spans
                .insert(key.clone(), span);
            scope.variables.insert(key.clone(), variable);
        }
        self.result.all_variables.insert(
            tcl_syntax::naming::scope_variable_key(&scope.name, &key),
            scope.variables[&key].clone(),
        );
    }

    fn original_variable_alias_receipt(
        &self,
        local_span: Span,
        target: &crate::signature_scan::variable_symbol::SignatureSourceVariableOccurrence,
    ) -> Option<crate::signature_scan::variable_symbol::OriginalVariableAliasReceipt> {
        use crate::registry_invocation::DeclarationVariableAliasPurpose as Purpose;
        #[cfg(debug_assertions)]
        if tcl_dialect::trace_env::var_os("TCL_LSP_TRACE_ALIAS_TEMPLATE_SOURCE").is_some() {
            eprintln!(
                "ALIAS_SOURCE_RECEIPT local={} remote={} frame={} input={} operands={:?}",
                local_span.start(),
                target.span().start(),
                self.head_identities
                    .source_bindings_ref()
                    .original_variable_alias_frame_at_span(local_span, self.file_lexer_config())
                    .is_some(),
                self.original_variable_operand_input(local_span).is_some(),
                self.result.original_variable_alias_operands
            );
        }

        let input = self.resolved_analysis_input();
        let image = tcl_lexer::SourceImage::document(&self.source);
        let local_input = self.original_variable_operand_input(local_span)?;
        let (frame, invocation_offset) = if input.has_logical_source_name_context() {
            let mut sources =
                self.result
                    .original_variable_alias_source_operands
                    .iter()
                    .filter(|source| {
                        source.operands().iter().any(|&(local, remote, _)| {
                            local == local_span && remote == target.span()
                        })
                    });
            let source = sources.next()?;
            let offset = source.invocation_offset()?;
            if !source.matches_source_context(
                &image,
                input.lexer_config(),
                input.borrowed_context_registry(),
            ) || sources.any(|other| {
                other.invocation_offset() != Some(offset)
                    || !other.matches_source_context(
                        &image,
                        input.lexer_config(),
                        input.borrowed_context_registry(),
                    )
            }) {
                return None;
            }
            let scope = self.head_identities.original_logical_variable_scope_at(
                &input,
                &image,
                local_span,
                &local_input,
            )?;
            (scope.alias_frame()?, offset)
        } else {
            self.head_identities
                .source_bindings_ref()
                .original_variable_alias_frame_at_span(local_span, self.file_lexer_config())?
        };
        let mut operands = self.result.original_variable_alias_operands.iter().filter(
            |&&(site, local, remote, _)| {
                site == invocation_offset && local == local_span && remote == target.span()
            },
        );
        let &(_, _, _, purpose) = operands.next()?;
        if operands.any(|operand| operand.3 != purpose) {
            return None;
        }
        let context = input.context_registry();
        if self
            .result
            .original_variable_alias_source_operands
            .iter()
            .any(|source| {
                source.invocation_offset() == Some(invocation_offset)
                    && source
                        .operands()
                        .iter()
                        .any(|&(local, remote, _)| local == local_span && remote == target.span())
                    && !source.matches_source_context(&image, self.file_lexer_config(), &context)
            })
        {
            return None;
        }
        if local_input.policy() != target.symbol().policy() {
            return None;
        }
        let local = crate::signature_scan::variable_symbol::original_variable_alias_local_name(
            &local_input,
            purpose,
        )?;
        Some(
            crate::signature_scan::variable_symbol::OriginalVariableAliasReceipt::new(
                frame,
                local,
                target.symbol().clone(),
                target.original_name_input().clone(),
                local_input,
                matches!(purpose, Purpose::Global | Purpose::NamespaceVariable),
                (invocation_offset, local_span),
            ),
        )
    }

    pub(in super::super) fn original_variable_namespace_at(
        &self,
        scope_path: &[usize],
    ) -> Option<crate::signature_scan::scope::SignatureNamespaceScope> {
        scope_at(&self.result.global_scope, scope_path)?
            .naming_scope
            .clone()
    }

    pub(in super::super) fn original_variable_root_namespace(
        &self,
    ) -> Option<crate::signature_scan::scope::SignatureNamespaceScope> {
        Some(crate::signature_scan::scope::SignatureNamespaceScope::root(
            Some(self.declaration_name_policy()?),
        ))
    }

    pub(super) fn original_static_variable_operand_input(
        &self,
        span: Span,
    ) -> Option<crate::signature_scan::scope::SignatureSourceNameInput> {
        use crate::signature_scan::scope::SignatureSourceNameInput;
        self.head_identities
            .original_source_name_at_span(
                span,
                self.file_lexer_config(),
                self.word_rules(),
                self.declaration_name_policy()?,
            )
            .or_else(|| self.original_static_source_name_at_span(span).cloned())
            .map(|original| SignatureSourceNameInput::OriginalWord(original.name_input().clone()))
    }

    /// Readonly name units from the actual invocation before constructing
    /// source namespace geometry. Static declaration fallbacks keep their
    /// existing independent source/policy owner; no variable receiver or
    /// runtime frame is created by this projection.
    pub(in super::super) fn original_variable_operand_input_at_invocation(
        &self,
        invocation_offset: u32,
        span: Span,
    ) -> Option<crate::signature_scan::scope::SignatureSourceNameInput> {
        // Implementation contract: naming.source.original-point-operand-projection
        // docs/design/analysis/name-resolution-proofs/original-point-operand-projection.md
        let policy = self.declaration_name_policy()?;
        self.head_identities
            .original_written_name_input_at_invocation_span(
                &tcl_lexer::SourceImage::document(&self.source),
                self.file_lexer_config(),
                invocation_offset,
                span,
            )
            .or_else(|| self.original_static_variable_operand_input(span))
            .filter(|input| input.policy() == policy)
    }

    pub(in super::super) fn original_variable_operand_input(
        &self,
        span: Span,
    ) -> Option<crate::signature_scan::scope::SignatureSourceNameInput> {
        self.original_qualified_variable_input(span, false)
    }

    /// **W215.** Emit when a variable's runtime name (or array element
    /// index) cannot be reached via `$`-substitution, even though it can
    /// be created/read via `set` / `info exists` / `upvar`.  The runtime
    /// name applies Tcl backslash substitution unless the word was
    /// braced (`STR`), since `{...}` preserves every byte verbatim.
    ///
    /// An unreachable name is usually a stray-delimiter typo of an
    /// ordinary nearby name (`set "a}b" 1` for `set ab 1`), so the
    /// message carries a "; did you mean 'X'?" suffix naming the closest
    /// variable already defined in the same scope, when one sits within
    /// the length-scaled edit budget.
    fn emit_w215_unreachable_name(
        &mut self,
        base_name: &str,
        element: Option<&str>,
        tok: Token,
        site_span: Span,
        scope_path: &[usize],
    ) {
        use super::super::types::Severity;
        use crate::naming::is_brace_substitutable;
        use std::borrow::Cow;

        let braced = tok.kind == TokenType::Str;

        // An unbraced word carrying a live `$var` / `[cmd]` substitution
        // (e.g. `set ns::$k 1`) is not a single literal token: `word_piece`
        // re-braces the substitution piece when reconstructing the word's
        // display text (`ns::$k` -> `"ns::${k}"`, so an adjacent literal
        // suffix can't be misread as part of the variable name), and that
        // reconstruction is what `base_name` is built from. Its `{`/`}`
        // characters are reconstruction artefacts, not text the analyser can
        // attribute to the *runtime* name — the substituted value is
        // unknown until the script actually runs, and for any ordinary
        // value (`k = "client_addr"`) the resulting name is a perfectly
        // ordinary, `$`-reachable one (`ns::client_addr`). Firing W215 on the
        // *name* here would flag the reconstruction, not a real defect —
        // skip that half of the check whenever the source word itself (not
        // the reconstructed text) contains an unresolved substitution.
        // The array-element `)` check below is unaffected: a
        // dynamic index round-trips verbatim (no re-bracing), so it carries
        // no such artefact.
        let name_is_static = braced
            || !self
                .source
                .get(tok.span.start() as usize..tok.span.end() as usize)
                .unwrap_or("")
                .contains(['$', '[']);

        let subst = |s: &str| -> String {
            if s.contains('\\') && !braced {
                tcl_lexer::backslash_subst(s).into_owned()
            } else {
                s.to_string()
            }
        };

        let runtime_name = subst(base_name);
        // The `${…}` delimiting rule is dialect-dependent (Tcl 9 tracks
        // nested braces and `\X` pairs; the 8.x family stops at the first
        // literal `}`) — resolve it from the active dialect's lexer config.
        let nesting = self.lexer_config().braced_var.nests();
        if name_is_static && !is_brace_substitutable(&runtime_name, nesting) {
            let detail = if runtime_name.contains('}') {
                if nesting {
                    "the brace form ``${name}`` ends at the first unbalanced ``}`` not preceded by ``\\`` (and the bare form stops at the first non-word character)"
                } else {
                    "the brace form ``${name}`` ends at the first ``}`` (and the bare form stops at the first non-word character)"
                }
            } else if runtime_name.ends_with('\\') {
                "the brace form ``${name}`` would read the trailing ``\\`` as the start of a 2-char escape and run out of input -- missing close-brace"
            } else if runtime_name.contains('{') {
                "the brace form ``${name}`` has unbalanced ``{`` and runs out of input -- missing close-brace"
            } else {
                "the brace form ``${name}`` cannot match this name"
            };
            let mut message = format!(
                "variable name ``{runtime_name}`` is not reachable via $-substitution; \
                 it can still be created/read via ``set name`` / ``[set \"name\"]`` / \
                 ``info exists`` / ``upvar``, but {detail}"
            );
            let candidates = self.scope_variable_names(scope_path);
            let suggestions = crate::text::suggest_similar(
                &runtime_name,
                candidates
                    .iter()
                    .map(String::as_str)
                    .filter(|name| *name != runtime_name),
                1,
                crate::text::scaled_max_distance_strict(&runtime_name),
            );
            if let Some(best) = suggestions.first() {
                use std::fmt::Write as _;
                let _ = write!(message, "; did you mean '{best}'?");
            }
            self.result
                .diagnostics
                .push(crate::analyser::types::Diagnostic::new(
                    DiagCode::W215,
                    site_span,
                    message,
                    Severity::Warning,
                ));
        }

        if let Some(elem) = element {
            let runtime_element: Cow<str> = if elem.contains('\\') && !braced {
                Cow::Owned(tcl_lexer::backslash_subst(elem).into_owned())
            } else {
                Cow::Borrowed(elem)
            };
            if runtime_element.contains(')') {
                self.result
                    .diagnostics
                    .push(crate::analyser::types::Diagnostic::new(
                        DiagCode::W215,
                        site_span,
                        "array element index contains ')'; the element can be created \
                              and read via ``set arr(idx) ...`` / ``[set \"arr(idx)\"]``, but \
                              is not reachable via $-substitution (``$arr(idx)`` reads up \
                              to the first ``)`` and stops there)"
                            .to_string(),
                        Severity::Warning,
                    ));
            }
        }
    }
}
