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

//! Script parameters at a call site: a braced argument the callee runs as a
//! script is analysed as a body in the frame the callee's contract names.
//!
//! Two owners must agree before the walk. The binding owner must have run
//! the word as a script while walking the call (its source walk maps a
//! formal bound from a braced literal back to that word), which is what
//! gives the analyser the word's commands. The callee's own body must give
//! the formal a [`ScriptParameter`] contract, which places the script: the
//! calling frame, a fresh frame, or the global one.

use super::super::state::Analyser;
use super::super::types::{ScriptParameter, ScriptParameterFrame, ScriptParameterTiming};
use std::collections::HashMap;
use std::sync::Arc;
use tcl_lexer::{Span, Token, TokenType};

impl Analyser {
    /// Keep a proc body so a later call can ask for its formals' script
    /// contracts. Inference runs only when a call passes a script literal.
    pub(super) fn retain_script_parameter_body(
        &mut self,
        params: &[crate::signature_scan::types::ParamDef],
        body_text: &str,
        body_span: Span,
    ) {
        self.script_parameter_contracts.remove(&body_span);
        if params.is_empty() {
            self.script_parameter_bodies.remove(&body_span);
            return;
        }
        self.script_parameter_bodies
            .insert(body_span, (params.to_vec(), Arc::from(body_text)));
    }

    /// The script contracts of the proc whose body is `body_span`, from the
    /// deep trait pass (a wrapper usually runs its script inside `catch` or
    /// `if`), inferred once per body and only for a body some call reaches.
    fn script_parameter_contracts(
        &mut self,
        body_span: Span,
    ) -> Option<Arc<HashMap<String, ScriptParameter>>> {
        if let Some(contracts) = self.script_parameter_contracts.get(&body_span) {
            return Some(Arc::clone(contracts));
        }
        let (params, body) = self.script_parameter_bodies.get(&body_span)?.clone();
        let mut contracts = self
            .with_proc_trait_scan(&params, &body, body_span, |names, text, env, offset| {
                let deep = super::super::param_traits::infer_param_traits_deep_in_document(
                    names,
                    text,
                    env,
                    &self.source,
                    &self.result,
                    offset,
                );
                let mut traits = super::super::param_traits::merge_traits(
                    super::super::param_traits::infer_param_traits(names, text, env),
                    deep,
                );
                super::super::param_traits::take_script_parameters(&mut traits)
            })
            .unwrap_or_default();
        // A wrapper some call has entered keeps no positioned proof for a
        // formal read nested in its own `catch` or condition, so read the
        // body as its own uncalled declaration. The binding owner has
        // separately shown that the call runs the argument as a script; the
        // contract only places it.
        if contracts.is_empty() {
            contracts = self
                .declared_script_parameters(&params, &body)
                .unwrap_or_default();
        }
        let contracts = Arc::new(contracts);
        self.script_parameter_contracts
            .insert(body_span, Arc::clone(&contracts));
        Some(contracts)
    }

    /// The script contracts of a body read as the one uncalled declaration
    /// of a document holding nothing else.
    fn declared_script_parameters(
        &self,
        params: &[crate::signature_scan::types::ParamDef],
        body: &str,
    ) -> Option<HashMap<String, ScriptParameter>> {
        let registry = self.registry.as_deref()?;
        let profile = registry.profile()?;
        let config = self.file_lexer_config();
        let names: Vec<&str> = params.iter().map(|param| param.name.as_str()).collect();
        let formals = tcl_syntax::list::join_list(names.iter().copied());
        let header = tcl_syntax::list::join_list(["proc", "__script_parameters", &formals]);
        let word = tcl_syntax::list::join_list([body]);
        let source = format!("{header} {word}");
        let start = u32::try_from(header.len() + 1).ok()?;
        // A braced token ends on its last inner character.
        let end = u32::try_from(source.len() - 1).ok()?;
        let identities = crate::realm::document_realm_bindings(&source, profile, registry);
        let script = identities.executed_script_for_word(Span::new(start, end))?;
        if script.try_text().ok()? != body {
            return None;
        }
        let env = super::super::param_traits::TraitScanEnv {
            surface: tcl_registry::model::DocumentCommandSurface::new(
                registry,
                self.declared_commands.as_ref(),
            ),
            config,
            identities: &identities,
            executed_source: Some(script),
            original_logical_body: None,
        };
        let mut traits = super::super::param_traits::merge_traits(
            super::super::param_traits::infer_param_traits(&names, body, env),
            super::super::param_traits::infer_param_traits_deep(&names, body, env),
        );
        Some(super::super::param_traits::take_script_parameters(
            &mut traits,
        ))
    }

    /// Analyse each braced argument the call's one selected procedure runs
    /// as a script, in the frame that procedure's contract names.
    ///
    /// Abstains on `{*}` expansion, an alias prefix, rename lineage, an
    /// argument a variadic `args` formal absorbs, a word the binding owner did
    /// not run, and a contract whose frame is unknown or two or more levels up.
    pub(in crate::analyser) fn dispatch_script_parameter_arguments(
        &mut self,
        args: &[String],
        arg_tokens: &[Token],
        cmd_tok: Token,
        scope_path: &[usize],
    ) {
        if args.len() != arg_tokens.len()
            || !arg_tokens.iter().any(|token| {
                token.kind == TokenType::Str
                    && self
                        .head_identities
                        .executed_script_for_word(token.span)
                        .is_some()
            })
        {
            return;
        }
        let Some(call) =
            crate::registry_invocation::source_structure::source_original_procedure_call_at(
                &self.source,
                &self.result,
                cmd_tok.span.start(),
            )
        else {
            return;
        };
        let words = call.original_words();
        if call.captured_argument_count() != 0
            || call.has_lookup_operand_effects()
            || !call.lineage().is_empty()
            || words.len() != args.len() + 1
            || words.iter().any(|word| word.group().expand)
        {
            return;
        }
        let Some(declaration) = call.declaration(&self.result) else {
            return;
        };
        let proc = declaration.metadata();
        if proc.params_computed {
            return;
        }
        // The dialect's formal grammar places each argument; one a variadic
        // formal absorbs has no formal of its own.
        let Some(grammar) = self
            .registry
            .as_deref()
            .and_then(tcl_registry::CommandRegistry::profile)
            .and_then(|profile| {
                tcl_native_model::InvocationDialect::of_profile(profile).parameter_grammar()
            })
        else {
            return;
        };
        let parameters: Vec<_> = proc
            .params
            .iter()
            .map(|param| tcl_syntax::formal_params::FormalParameter {
                name: param.name.clone(),
                default: param.default_value.clone(),
            })
            .collect();
        let Ok(bindings) =
            tcl_syntax::formal_params::bind_formal_arguments(&parameters, args.len(), grammar)
        else {
            return;
        };
        let formals: Vec<(usize, String)> = bindings
            .into_iter()
            .filter_map(|binding| match binding {
                tcl_syntax::formal_params::FormalArgumentBinding::Value {
                    parameter,
                    argument,
                } => Some((argument, parameters.get(parameter)?.name.clone())),
                _ => None,
            })
            .collect();
        let namespace = tcl_syntax::naming::reported_qname_namespace(
            &proc.qualified_name,
            tcl_syntax::naming::ReportedQNameNamespacePurpose::AfterLeadingRoot,
        );
        let body_span = proc.body_span;
        let Some(contracts) = self.script_parameter_contracts(body_span) else {
            return;
        };
        for (index, formal) in &formals {
            let (Some(text), Some(&token)) = (args.get(*index), arg_tokens.get(*index)) else {
                continue;
            };
            if token.kind != TokenType::Str
                || self
                    .head_identities
                    .executed_script_for_word(token.span)
                    .is_none()
            {
                continue;
            }
            if let Some(&contract) = contracts.get(formal) {
                self.analyse_script_argument(contract, text, token, scope_path, &namespace);
            }
        }
    }

    fn analyse_script_argument(
        &mut self,
        contract: ScriptParameter,
        text: &str,
        token: Token,
        scope_path: &[usize],
        namespace: &str,
    ) {
        let deferred = contract.timing == ScriptParameterTiming::Deferred;
        let path: Vec<usize> = match contract.frame {
            ScriptParameterFrame::Caller(1) if !deferred => scope_path.to_vec(),
            ScriptParameterFrame::Global => Vec::new(),
            ScriptParameterFrame::Own if !deferred => {
                self.fresh_script_parameter_scope(namespace, token)
            }
            _ => return,
        };
        // A later callback never dominates the code after the call.
        if deferred {
            self.conditional_depth += 1;
            self.control_flow_body_depth += 1;
        }
        self.analyse_body(text, token, &path);
        if deferred {
            self.conditional_depth -= 1;
            self.control_flow_body_depth -= 1;
        }
    }

    /// A fresh activation for a script the callee runs in its own frame,
    /// where none of the caller's locals exist.
    fn fresh_script_parameter_scope(&mut self, namespace: &str, token: Token) -> Vec<usize> {
        let scope_name = format!("script@{}", token.span.start());
        let path = super::super::per_item::reconstruct_proc_scope(
            &mut self.result.global_scope,
            namespace,
            &scope_name,
            super::super::types::ScopeKind::Proc,
        );
        if let Some(scope) = super::super::scope::scope_at_mut(&mut self.result.global_scope, &path)
        {
            scope.body_span = Some(token.span);
        }
        path
    }
}
