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

//! W211 unused variables and W214 unused parameters, with the dispatch-protocol peers that excuse a parameter.

use super::dead_stores::{
    original_unrepresented_use_advice, written_namespace_storage, written_object_storage,
};
use super::find_case_mismatch;
use crate::analyser::state::Analyser;
use crate::analyser::types::Severity;
use rustc_hash::FxHashMap;
use std::collections::HashSet;
use tcl_core_types::DiagCode;

#[derive(Clone, PartialEq, Eq, Hash)]
struct OriginalDispatchProtocol {
    policy: tcl_syntax::naming::NamePolicyProtocol,
    namespace: crate::signature_scan::scope::SignatureNamespaceScope,
    pub(super) params: Vec<String>,
}

struct OriginalDispatchPeer {
    pub(super) protocol: OriginalDispatchProtocol,
    body_span: tcl_lexer::Span,
    publication: crate::signature_scan::scope::SignatureSourceCommand,
}

fn original_dispatch_site(
    module: &crate::ir::Module,
    site: &super::super::super::state::VarCommandSite,
    registry: &tcl_registry::CommandRegistry,
) -> bool {
    let mut retained = false;
    for (script, _) in module.executable_script_roots() {
        crate::ir::for_each_statement(script, &mut |statement| {
            let Some(tokens) = statement.tokens() else {
                return;
            };
            let Some(head) = tokens.words().first() else {
                return;
            };
            if head.source().span != site.cmd_span
                || head.source().provenance != crate::ir::Provenance::Source
                || tokens.words().len() != site.argc + 1
                || !head
                    .sole_variable_substitution()
                    .is_some_and(|(spelling, _)| {
                        tcl_lexer::word_parts::whole_var_ref(
                            spelling.as_bytes(),
                            module.lexer_config,
                        )
                        .ok()
                        .flatten()
                        .is_some_and(|reference| {
                            reference.index.is_none() && reference.name == site.var_name.as_bytes()
                        })
                    })
                || tokens.synthetic.is_some()
                || !tokens.original_words_match_argv_text()
            {
                return;
            }
            retained |= tokens.source_binding.as_ref().is_some_and(|binding| {
                binding
                    .original_argument_metadata_for_module(tokens, module, registry)
                    .is_some()
            });
        });
    }
    retained
}

/// Named command outputs and synthetic spans do not identify unused assignments.
fn unused_variable_definition_span(
    fu: &crate::compilation_unit::FunctionUnit,
    definition: &crate::def_use::DefSite,
) -> Option<tcl_lexer::Span> {
    let block = fu.cfg.block_by_name(&definition.block)?;
    let idx = usize::try_from(definition.statement_index).ok()?;
    let stmt = block.statements.get(idx)?;
    // Only pure assignments are reportable as "set but never used".
    // A variable written by a command (`scan` / `binary scan` /
    // `regexp -> capture`, etc.) or a barrier is a command output the
    // user may legitimately ignore; `Statement::Call` /
    // `Statement::Barrier` defs are skipped.  This is deliberate
    // policy, not a gap: a destructuring writer's surplus output
    // (`binary scan $d H2H* type rest` with
    // `rest` unread) is how Tcl spells "ignore the remainder" — there
    // is no `_` placeholder — so flagging it would punish the idiom.
    if matches!(
        stmt,
        crate::ir::Statement::Call { .. } | crate::ir::Statement::Barrier { .. }
    ) {
        return None;
    }
    // The CFG span is relative to the unit's `base_offset`.
    let cmd_span = fu.abs_span(stmt.span());
    if cmd_span.is_empty() {
        return None;
    }
    Some(cmd_span)
}

impl Analyser {
    /// W211 — unused-variable hint.
    ///
    /// Fires when an
    /// assignment's variable has no live uses **and** no other
    /// SSA version is live (so the variable is entirely unused
    /// — distinct from W220's overwritten-before-read case).
    ///
    /// Three filters apply:
    ///
    /// 1. **Scope aliases** (``global`` / ``upvar``) — writes
    ///    are visible in the aliased scope, so a "no local use"
    ///    verdict is unsafe.
    /// 2. **Textual references** — variable names that appear
    ///    inside a ``"$x"`` string interpolation or a
    ///    ``Return`` value are kept live; the def-use builder
    ///    doesn't track those reads.
    /// 3. **Empty spans** — synthetic IR statements with no
    ///    user-visible source text.
    ///
    /// "Did you mean…?" suggestions use case-insensitive
    /// matching against the function's defined-variable set.
    pub(in super::super) fn emit_unused_variable_diagnostics(
        &mut self,
        fu: &crate::compilation_unit::FunctionUnit,
        defined_vars: &HashSet<String>,
        scope_aliases: &HashSet<String>,
        textually_referenced: &HashSet<String>,
        cell_facts: &super::super::helpers::DiagnosticCellFacts,
    ) {
        use crate::def_use::DefKind;
        // A dynamic read (`foreach v [info locals] {… [set $v] …}`) reaches
        // every local by a name no literal `$x` token spells, so "set but
        // never used" is unprovable.  Abstain toward silence.
        if fu.dynamic_names.reads {
            return;
        }
        let generation = self.analysis_context();
        let registry = self.registry.as_deref().unwrap_or(generation.commands());
        let declaration_layout = std::cell::OnceCell::new();
        // W211 is a per-variable verdict ("the variable is set but never
        // used"), not per-assignment: a variable set several times and never
        // read fires once, at its earliest definition. Collect the earliest
        // reportable span per variable, then emit a single W211 per unused
        // variable.
        let mut earliest: std::collections::HashMap<String, tcl_lexer::Span> =
            std::collections::HashMap::new();
        let observed_by_unseen_calls = names_observed_by_unseen_calls(fu);
        for chain in fu.def_use.chains.values() {
            if !chain.is_dead() || chain.definition.kind != DefKind::Statement {
                continue;
            }
            let (cell_name, version) = &chain.key;
            let Some(symbol) = fu.def_use.symbol(&fu.ssa, *cell_name) else {
                continue;
            };
            let Some(cell_name) = fu.ssa.cell_key(symbol) else {
                continue;
            };
            if super::super::helpers::definition_has_cell_fact(
                fu,
                &chain.definition,
                cell_name,
                &cell_facts.externally_read,
                registry,
            ) {
                continue;
            }
            let var = fu.ssa.var_name(symbol);
            // Namespace storage is visible beyond this unit. Local keys with
            // qualified-looking labels do not acquire namespace membership.
            if written_namespace_storage(cell_name, var) || written_object_storage(cell_name) {
                continue;
            }
            if scope_aliases.contains(var) {
                continue;
            }
            if textually_referenced.contains(var)
                || original_unrepresented_use_advice(
                    fu,
                    &chain.definition,
                    var,
                    &self.profile_registry(),
                    &declaration_layout,
                )
            {
                continue;
            }
            if observed_by_unseen_calls.contains(var) {
                continue;
            }
            // A synthetic may-def (base refresh / element fan) is not a
            // write the user made — the element's own chain reports.
            if fu.ssa.is_synthetic_def(
                &chain.definition.block,
                chain.definition.statement_index,
                cell_name,
            ) {
                continue;
            }
            // Interpreter-provided special variables (``auto_path``, ``env``,
            // …) are consumed by the runtime even when the script never reads
            // them, so a bare ``set auto_path …`` is not an unused variable.
            // Dialect-aware via the special-variable registry.
            if super::super::helpers::special_variable_definition(
                fu,
                &chain.definition,
                cell_name,
                registry,
            )
            .is_some_and(|simple| {
                registry.is_externally_read(
                    &simple,
                    Some(self.analysis_context().context().authoring_query()),
                )
            }) {
                continue;
            }
            // Only emit when no other SSA version of this var is
            // live — the W220 path handles overwritten cases.
            let any_other_live = fu
                .def_use
                .chains
                .iter()
                .any(|(k, c)| k.0 == symbol && k.1 != *version && !c.is_dead());
            if any_other_live {
                continue;
            }
            let Some(cmd_span) = unused_variable_definition_span(fu, &chain.definition) else {
                continue;
            };
            // Anchor at the variable name (the assignment target), not the
            // command-start column.
            let span = self.narrow_to_assigned_name(cmd_span).unwrap_or(cmd_span);
            earliest
                .entry(var.to_owned())
                .and_modify(|s| {
                    if span.start() < s.start() {
                        *s = span;
                    }
                })
                .or_insert(span);
        }
        self.emit_unused_variable_spans(earliest, defined_vars);
    }

    fn emit_unused_variable_spans(
        &mut self,
        earliest: std::collections::HashMap<String, tcl_lexer::Span>,
        defined_vars: &HashSet<String>,
    ) {
        use std::fmt::Write as _;

        let mut entries: Vec<(String, tcl_lexer::Span)> = earliest.into_iter().collect();
        entries.sort_by_key(|(_, span)| span.start());
        // A variable that is set-but-never-used gets a W211 at its assignment's
        // name token. The dead-store pass (W220), which ran first, already
        // anchored a "never read" hint at the *same* token for that single
        // assignment — a redundant double-emit. W211 ("never used at all") is
        // the more informative message, so drop the co-located W220. Keyed on
        // the exact span, so a genuinely distinct dead store of a
        // multiply-assigned variable is untouched.
        let w211_spans: std::collections::HashSet<tcl_lexer::Span> =
            entries.iter().map(|(_, span)| *span).collect();
        self.result
            .diagnostics
            .retain(|d| !(d.code == DiagCode::W220 && w211_spans.contains(&d.span)));
        for (var, span) in entries {
            let mut message = format!("Variable '{var}' is set but never used");
            if let Some(similar) = find_case_mismatch(&var, defined_vars) {
                let _ = write!(message, "; did you mean '{similar}'?");
            }
            self.result
                .diagnostics
                .push(crate::analyser::types::Diagnostic::new(
                    DiagCode::W211,
                    span,
                    message,
                    Severity::Hint,
                ));
        }
    }

    /// Original parameter name geometry for this body, without a label join.
    fn param_name_sources_for(
        &self,
        fu: &crate::compilation_unit::FunctionUnit,
        ir_proc: &crate::ir::Procedure,
        module: Option<&crate::ir::Module>,
    ) -> Vec<crate::signature_scan::formal_parameters::OriginalSourceFormalName> {
        let select = || {
            crate::source_graph::current_analysis(&self.source, &self.result)?;
            if fu.source_metadata_input() != self.result.resolved_input.as_ref() {
                return None;
            }
            let site = ir_proc.declaration_site.as_ref()?;
            let mut declarations = self
                .result
                .original_procedure_declarations()
                .filter(|declaration| declaration.declaration_site() == site);
            let declaration = declarations.next()?;
            if declarations.next().is_some() {
                return None;
            }
            let (id, body) = crate::source_graph::procedure_body_for_declaration(
                &self.source,
                &self.result,
                module?,
                declaration,
            )?;
            if fu.body_id != Some(id) || !std::ptr::eq(body, ir_proc) {
                return None;
            }
            let fields = self.result.original_procedure_formal_name_sources(
                declaration,
                self.result.resolved_registry()?,
            )?;
            fields
                .iter()
                .map(crate::signature_scan::formal_parameters::OriginalSourceFormalName::name)
                .eq(ir_proc.params.iter().map(String::as_bytes))
                .then_some(fields)
        };
        select().unwrap_or_default()
    }

    fn param_reference_candidates_for(
        &self,
        fu: &crate::compilation_unit::FunctionUnit,
        procedure: &crate::ir::Procedure,
        module: Option<&crate::ir::Module>,
    ) -> Option<std::collections::BTreeSet<String>> {
        crate::source_graph::current_analysis(&self.source, &self.result)?;
        if fu.source_metadata_input() != self.result.resolved_input.as_ref() {
            return None;
        }
        let site = procedure.declaration_site.as_ref()?;
        let mut declarations = self
            .result
            .original_procedure_declarations()
            .filter(|declaration| declaration.declaration_site() == site);
        let declaration = declarations.next()?;
        if declarations.next().is_some() {
            return None;
        }
        let (id, original) = crate::source_graph::procedure_body_for_declaration(
            &self.source,
            &self.result,
            module?,
            declaration,
        )?;
        if fu.body_id != Some(id) || !std::ptr::eq(original, procedure) {
            return None;
        }
        let selected = crate::registry_invocation::source_structure::source_registry_words_at(
            &self.source,
            &self.result,
            site.offset,
        )?;
        let context = self.result.resolved_input.as_ref()?.context_registry();
        let mut bodies = selected
            .source_script_bodies(&context)
            .into_iter()
            .filter(|body| {
                body.original_container().group().span == declaration.metadata().body_span
            });
        let body = bodies.next()?;
        if bodies.next().is_some() {
            return None;
        }
        crate::registry_invocation::original_body_reference_candidates(
            &self.source,
            &self.result,
            &body,
        )
    }

    /// W214 — unused-parameter hint.
    ///
    /// For every parameter
    /// declared in `ir_proc.params`, check whether any def-use
    /// chain for the parameter (any SSA version) has live uses.
    /// When all chains are dead, the parameter is unused —
    /// emit a Hint at the parameter's *name* span (falling back to the
    /// proc's span when the name can't be located), so each unused param
    /// gets its own tight squiggle instead of stacking on the whole proc.
    pub(in super::super) fn emit_unused_param_diagnostics(
        &mut self,
        fu: &crate::compilation_unit::FunctionUnit,
        ir_proc: &crate::ir::Procedure,
        module: Option<&crate::ir::Module>,
    ) {
        // Empty-body procs (``proc foo {a b} {}``) are signature
        // placeholders — stubs declaring an API whose implementation
        // lives elsewhere.  Every parameter is necessarily "unused"
        // since there is no body to use it, so flagging is pure noise.
        if ir_proc.body.statements.is_empty() {
            return;
        }
        // Per-parameter name spans (index-aligned with `ir_proc.params`); an
        // empty result means we couldn't isolate the list and fall back to the
        // whole-definition span below.
        let param_sources = self.param_name_sources_for(fu, ir_proc, module);
        let reference_candidates = self.param_reference_candidates_for(fu, ir_proc, module);
        let mut unused: Vec<(usize, String)> = Vec::new();
        for (idx, param) in ir_proc.params.iter().enumerate() {
            // Tcl's variadic ``args`` parameter is conventionally
            // declared even when unused (as a "consume the rest"
            // marker).  Skip it from W214.
            if tcl_syntax::formal_params::is_variadic_formal_name(param) {
                continue;
            }
            // Quoted source keywords are positional protocol markers.
            if param_sources.get(idx).is_some_and(
                crate::signature_scan::formal_parameters::OriginalSourceFormalName::quoted_marker,
            ) {
                continue;
            }
            let any_live = fu.def_use.chains.iter().any(|(key, chain)| {
                fu.ssa
                    .var_symbol(param)
                    .is_some_and(|symbol| fu.def_use.symbol(&fu.ssa, key.0) == Some(symbol))
                    && !chain.is_dead()
            });
            if any_live {
                continue;
            }
            // Source candidates protect nested reads without claiming a physical
            // cell. Only independently selected inherited bodies retain this scope.
            if reference_candidates
                .as_ref()
                .is_some_and(|names| names.contains(param))
            {
                continue;
            }
            unused.push((idx, param.clone()));
        }
        if unused.is_empty() {
            return;
        }
        let protocol_params = module
            .and_then(|module| {
                let id = fu.body_id?;
                if !module.procedure_ids.owns(id)
                    || !std::ptr::eq(module.procedures.get(&id)?, ir_proc)
                {
                    return None;
                }
                let peers = self.original_dispatch_peers(module);
                let peer = peers.get(&id)?;
                self.dispatch_protocol_signatures(module, &peers)
                    .contains(&peer.protocol)
                    .then(|| peer.protocol.params.iter().cloned().collect::<HashSet<_>>())
            })
            .unwrap_or_default();
        for (idx, param) in unused {
            if protocol_params.contains(&param) {
                continue;
            }
            let message = format!(
                "Parameter '{param}' of proc '{name}' is unused",
                name = ir_proc.qualified_name,
            );
            self.result
                .diagnostics
                .push(crate::analyser::types::Diagnostic::new(
                    DiagCode::W214,
                    param_sources
                        .get(idx)
                        .and_then(crate::signature_scan::formal_parameters::OriginalSourceFormalName::source_span)
                        .unwrap_or(ir_proc.span),
                    message,
                    Severity::Hint,
                ));
        }
    }

    fn original_dispatch_peers(
        &self,
        module: &crate::ir::Module,
    ) -> FxHashMap<crate::ir::ProcedureId, OriginalDispatchPeer> {
        let mut peers = FxHashMap::default();
        if crate::source_graph::current_analysis(&self.source, &self.result).is_none() {
            return peers;
        }
        for declaration in self.result.original_procedure_declarations() {
            let Some((id, body)) = crate::source_graph::procedure_body_for_declaration(
                &self.source,
                &self.result,
                module,
                declaration,
            ) else {
                continue;
            };
            if body.publication.as_ref() != Some(declaration.name()) {
                continue;
            }
            let Some(namespace) = declaration.name().body_scope() else {
                continue;
            };
            let params = body
                .params
                .iter()
                .take_while(|name| !tcl_syntax::formal_params::is_variadic_formal_name(name))
                .cloned()
                .collect::<Vec<_>>();
            if params.is_empty() {
                continue;
            }
            peers.insert(
                id,
                OriginalDispatchPeer {
                    protocol: OriginalDispatchProtocol {
                        policy: declaration.name().policy(),
                        namespace,
                        params,
                    },
                    body_span: declaration.metadata().body_span,
                    publication: declaration.name().clone(),
                },
            );
        }
        peers
    }

    /// Suppress protocol parameters only for distinct original publications
    /// with an authentic, arity-compatible dispatcher in the same source scope.
    fn dispatch_protocol_signatures(
        &self,
        module: &crate::ir::Module,
        peers: &FxHashMap<crate::ir::ProcedureId, OriginalDispatchPeer>,
    ) -> HashSet<OriginalDispatchProtocol> {
        let Some(registry) = self.result.resolved_registry() else {
            return HashSet::new();
        };
        let mut groups: FxHashMap<OriginalDispatchProtocol, HashSet<_>> = FxHashMap::default();
        for peer in peers.values() {
            groups
                .entry(peer.protocol.clone())
                .or_default()
                .insert(&peer.publication);
        }
        groups
            .into_iter()
            .filter(|(_, publications)| publications.len() >= 3)
            .map(|(protocol, _)| protocol)
            .filter(|protocol| {
                self.var_command_sites.iter().any(|site| {
                    if site.in_method
                        || site.has_expand
                        || site.argc < protocol.params.len()
                        || !original_dispatch_site(module, site, registry)
                    {
                        return false;
                    }
                    let Some(namespace) = self.original_dispatch_namespace(site, peers) else {
                        return false;
                    };
                    namespace == &protocol.namespace
                        || protocol
                            .namespace
                            .is_strict_ancestor_of(namespace, protocol.policy)
                            == Some(true)
                })
            })
            .collect()
    }

    fn original_dispatch_namespace<'a>(
        &'a self,
        site: &super::super::super::state::VarCommandSite,
        peers: &'a FxHashMap<crate::ir::ProcedureId, OriginalDispatchPeer>,
    ) -> Option<&'a crate::signature_scan::scope::SignatureNamespaceScope> {
        let mut containing = peers
            .values()
            .filter(|peer| {
                peer.body_span.start() <= site.cmd_span.start()
                    && site.cmd_span.end() <= peer.body_span.end()
            })
            .collect::<Vec<_>>();
        containing.sort_by_key(|peer| peer.body_span.as_range().len());
        if let Some(first) = containing.first() {
            if containing
                .iter()
                .skip(1)
                .any(|other| other.body_span == first.body_span && other.protocol != first.protocol)
            {
                return None;
            }
            return Some(&first.protocol.namespace);
        }
        self.result
            .original_namespace_scope_at(site.cmd_span.start())
    }
}

/// The names some version of which a call to a command the module cannot see
/// holds where it runs: that code may read the name at any version it holds
/// there, so such a name is never unused.
fn names_observed_by_unseen_calls(fu: &crate::compilation_unit::FunctionUnit) -> HashSet<&str> {
    fu.def_use
        .chains
        .keys()
        .filter_map(|(cell, held)| {
            let symbol = fu.def_use.symbol(&fu.ssa, *cell)?;
            fu.ssa
                .is_observed_by_unseen_call(symbol, *held)
                .then(|| fu.ssa.var_name(symbol))
        })
        .collect()
}

/// Authored fixture adapter; production supplies its actual declaration/body owner.
#[cfg(test)]
pub(in super::super) fn body_references_param(
    body: &str,
    param: &str,
    config: tcl_lexer::LexerConfig,
) -> bool {
    let profile = tcl_dialect::DialectProfile::plain_tcl();
    let context =
        crate::environment_ingress::resolve_environment("tcl8.6").default_context_registry();
    let input =
        crate::analyser::ResolvedAnalysisInput::new(profile, profile, context.clone(), config);
    let parameters = tcl_syntax::list::join_list([param]);
    let source = tcl_syntax::list::join_list(["proc", "__w214__", &parameters, body]);
    let analysis = Analyser::new()
        .with_resolved_input(input)
        .analyse(&source, "tcl");
    let Some(selected) = crate::registry_invocation::source_structure::source_registry_words_at(
        &source, &analysis, 0,
    ) else {
        return false;
    };
    let mut bodies = selected.source_script_bodies(&context).into_iter();
    let Some(body) = bodies.next() else {
        return false;
    };
    bodies.next().is_none()
        && crate::registry_invocation::original_body_reference_candidates(&source, &analysis, &body)
            .is_some_and(|names| names.contains(param))
}
