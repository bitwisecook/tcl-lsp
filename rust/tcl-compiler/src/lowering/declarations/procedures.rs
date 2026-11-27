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

//! `proc` declarations: names and formals, the declared body, dynamic names and bodies.

use super::super::Lowerer;
use super::super::script_bodies::over_depth_script;
use crate::ir::{CommandTokens, Procedure, Script, Statement};
use tcl_registry::events::{IrulesCommandPlacement, IrulesExecutionContext};
use tcl_source::segment::SegmentedCommand;
use tcl_syntax::word_rules::WordValueRules;

/// Qualify a procedure name relative to a namespace.
pub(in super::super) fn qualify_proc_name(namespace: &str, proc_name: &str) -> String {
    tcl_syntax::naming::qualify(namespace, proc_name)
}

/// The namespace a `proc`'s **body** resolves names against: the namespace the
/// proc is *defined in* (the qualifier prefix of its own qualified name), not
/// the lexical namespace the `proc` command was written in.
///
/// The two agree for every proc whose name word carries no qualifier — the
/// overwhelmingly common case, including `proc outer {} { proc inner … }` at
/// top level — and diverge exactly when the name word is itself qualified, so
/// the definition site and the defining namespace are different namespaces.
///
/// Observed identically on tclsh 9.0.4 and 8.6.16 (`self class`-style
/// introspection is not involved; this is plain `info commands` after
/// running the outer proc):
///
/// ```tcl
/// namespace eval ::a {}
/// proc a::outer {} { proc helper {x} { return $x } ; return [namespace current] }
/// a::outer                        ;# -> ::a           (the body's frame namespace)
/// info commands ::helper          ;# -> {}            (NOT the lexical ::)
/// info commands ::a::helper       ;# -> ::a::helper
///
/// namespace eval ::b::c {}
/// proc b::c::o2 {} { proc h2 {y} { return $y } }
/// b::c::o2 ; info commands ::b::c::h2   ;# -> ::b::c::h2, and ::h2 is empty
///
/// # The *lexical* enclosing `namespace eval` loses to the name's own qualifier:
/// namespace eval ::e {}
/// namespace eval ::d { proc ::e::o3 {} { proc h3 {z} { return $z } } }
/// ::e::o3 ; info commands ::e::h3       ;# -> ::e::h3, ::d::h3 and ::h3 empty
///
/// # Nesting composes — each level re-homes to its own defining namespace:
/// namespace eval ::g {}
/// proc g::o7 {} { proc o7b {} { proc h7 {u} { return $u } } }
/// g::o7 ; ::g::o7b ; info commands ::g::h7   ;# -> ::g::h7, ::h7 empty
/// ```
///
/// An absolutely-written nested name (`proc ::abs {q} {…}`) is unaffected —
/// [`qualify_proc_name`] already short-circuits on the leading `::`.
///
/// `qualified` is always rooted (it comes from [`qualify_proc_name`], which
/// routes every result through `normalise_qualified_name`), so the holder is
/// never the empty "relative" marker [`tcl_syntax::naming::key_holder_and_tail`]
/// returns for a bare simple name; `lexical_fallback` covers that
/// impossible-by-construction case rather than silently producing `""`.
#[cfg(test)]
pub(in super::super) fn proc_body_namespace(qualified: &str, lexical_fallback: &str) -> String {
    let (holder, _) = tcl_syntax::naming::key_holder_and_tail(qualified);
    if holder.is_empty() {
        lexical_fallback.to_string()
    } else {
        holder.to_string()
    }
}

/// Configured list names from a brace-collapsed compatibility word.
#[cfg(test)]
pub(in super::super) fn parse_var_list_names(
    list_text: &str,
    rules: WordValueRules,
) -> Option<Vec<String>> {
    rules.split_word_names(list_text)
}

/// Split a Tcl *formal parameter* list word into its parameter names.
///
/// This is the `proc` / `apply`-lambda / TclOO-method parameter word, a
/// different grammar from a loop variable list: it has two list levels, the
/// outer one holding a specifier per parameter and each specifier itself a
/// `name ?default?` list. So `{x {y 2} args}` is the three parameters `x`, `y`
/// (defaulting to `2`), and `args`; and `proc p {a\ b} {}` is the *single*
/// parameter `a` defaulting to `b` (tclsh 9.0: `info args p` → `a`), not the
/// two names a whitespace split would produce.
/// [`tcl_syntax::formal_params::parse_formal_parameters_in`] owns the selected
/// engine grammar and
/// is what the VM, the WASM runtime, and `signature_scan` all decode with.
///
/// Only the names reach the IR — [`Procedure::params`] is name-only, and
/// `params_raw` keeps the source text for consumers that need the defaults.
///
/// An unavailable grammar or a parameter list refused by that engine declines.
/// Jim keeps its own lenient list and name grammar; original link spelling
/// in a name-only header supplies no caller-link or ordinary-cell activation.
pub(in super::super) fn parse_formal_param_names(
    param_text: &str,
    rules: WordValueRules,
    grammar: Option<tcl_dialect::ParameterGrammar>,
) -> Option<Vec<String>> {
    let collapsed = rules.collapse_braced_word(param_text);
    let parameters =
        tcl_syntax::formal_params::parse_formal_parameters_in(&collapsed, grammar?).ok()?;
    Some(
        parameters
            .into_iter()
            .map(|parameter| parameter.name)
            .collect(),
    )
}

/// Retain original creation/publication geometry without borrowing compiler policy.
fn original_procedure_publication(
    tokens: &CommandTokens,
) -> Option<crate::signature_scan::scope::SignatureSourceCommand> {
    use crate::signature_scan::scope::{SignatureSourceCommand, SignatureSourceNameInput};
    let binding = tokens.source_binding.as_ref()?;
    let input = binding.original_written_name_input(tokens, 1)?;
    let SignatureSourceNameInput::OriginalWord(key) = &input else {
        return None;
    };
    let scope = binding.original_operand_naming_scope(&input)?;
    SignatureSourceCommand::procedure_from_key(&scope, key)
}

impl Lowerer<'_> {
    /// Retain a conditional Logical procedure directly from its sealed source
    /// recipe. Native parameter/publication/entry receipts remain unavailable.
    pub(in super::super) fn try_lower_logical_procedure_declaration(
        &mut self,
        seg: &SegmentedCommand,
        namespace: &str,
        tokens: &CommandTokens,
    ) -> Option<Statement> {
        if self.compilation_scope == tcl_runtime_api::SourceCompilationScope::EnteredSource {
            return None;
        }
        let input = self
            .invocation_metadata_context()?
            .source_analysis_input()?;
        let declaration = self
            .source_bindings
            .as_ref()?
            .original_logical_procedure_declaration(tokens, self.registry, input)?;
        let qualified = declaration.qualified_name()?.to_owned();
        let name = declaration.name()?.to_owned();
        let params_raw = declaration.parameters_text()?.to_owned();
        let params = declaration
            .parameters()
            .iter()
            .map(|parameter| parameter.name.clone())
            .collect();
        let source = declaration.source().clone();
        let body_offset = source.base();
        let body_source = source.try_text().ok()?.to_owned();
        let body = self
            .in_procedure_frame(Some(IrulesExecutionContext::ProcedureBody), |lowerer| {
                lowerer.lower_original_body(source, Some(declaration.namespace()))
            });
        if !self.suppress_proc_register {
            let publication = original_procedure_publication(tokens);
            if publication.as_ref().is_some_and(|publication| {
                self.module
                    .procedures
                    .values()
                    .any(|previous| previous.publication.as_ref() == Some(publication))
            }) {
                self.module.redefined_procedures.insert(qualified.clone());
            }
            self.module.retain_procedure(Procedure {
                name,
                qualified_name: qualified,
                publication,
                declaration_site: tokens
                    .source_binding
                    .as_ref()
                    .and_then(|binding| binding.invocation_site())
                    .cloned(),
                body_kind: crate::ir::ProcedureBodyKind::Named,
                params,
                span: seg.span,
                body,
                params_raw,
                body_source: Some(body_source),
                body_offset,
                namespace_scoped: self.in_namespace_eval,
                base_priority: 500,
            });
        }
        Some(self.lower_default(seg, namespace))
    }

    pub(in super::super) fn try_lower_proc_declaration(
        &mut self,
        seg: &SegmentedCommand,
        namespace: &str,
        canonical_command: &str,
    ) -> Option<Statement> {
        if seg.args().len() != 3 || seg.arg_tokens().len() < 3 {
            return None;
        }
        if self
            .registry
            .profile()
            .is_some_and(tcl_dialect::DialectProfile::is_irules)
        {
            if self
                .registry
                .irules_command_placement(canonical_command, self.irules_execution_context)
                != IrulesCommandPlacement::Allowed
            {
                return None;
            }
            let arg_refs: Vec<&str> = seg.args().iter().map(String::as_str).collect();
            let closed = tcl_registry::events::closed_braced_argument_words(
                &self.source,
                seg.arg_tokens(),
                seg.arg_single_token(),
            )?;
            let declaration = tcl_registry::events::IrulesDeclarationArguments::new(
                &arg_refs,
                seg.arg_tokens(),
                seg.arg_single_token(),
                &closed,
            )
            .and_then(|arguments| {
                self.registry
                    .irules_top_level_declaration_shape(canonical_command, arguments)
            });
            if !matches!(
                declaration,
                Some(tcl_registry::events::IrulesTopLevelDeclaration::Procedure { .. })
            ) {
                return None;
            }
        }
        Some(self.lower_proc(seg, namespace))
    }

    /// Lower `proc name params body`.
    ///
    /// In order:
    ///   1. Empty-simple-name barrier (`proc ::ns:: {…} {…}`).
    ///   2. Dynamic name resolution (`proc $x …` / `proc [cmd] …`).
    ///   3. Dynamic body / params check, with const-map body
    ///      materialisation when possible.
    ///   4. IR registration + emit the runtime `proc` call.
    pub(in super::super) fn lower_proc(
        &mut self,
        seg: &SegmentedCommand,
        namespace: &str,
    ) -> Statement {
        if self.compilation_scope == tcl_runtime_api::SourceCompilationScope::EnteredSource {
            return self.lower_default(seg, namespace);
        }
        let args_borrow = seg.args();
        let proc_name_initial = &args_borrow[0];

        // Empty simple names need runtime publication; preserve the original
        // invocation instead of registering a lifted procedure.
        if tcl_syntax::naming::ends_with_separator(proc_name_initial.as_bytes()) {
            return Statement::Barrier {
                span: seg.span,
                reason: "empty-simple-name proc".into(),
                command: seg.name().into(),
                canonical_command: None,
                args: args_borrow.to_vec(),
                tokens: Some(self.cmd_tokens(seg)),
            };
        }
        // Resolve a dynamic proc name.
        let (proc_name_owned, args_owned, name_was_substituted) =
            match self.resolve_dynamic_proc_name(seg, args_borrow) {
                Ok(triple) => triple,
                Err(barrier) => return *barrier,
            };
        let args: &[String] = &args_owned;
        let proc_name = &proc_name_owned;
        // Check for a dynamic body / params, materialising the body via the
        // const-map where possible.
        let (materialised_body, body_is_dynamic, body_offset) =
            match self.check_proc_body_dynamic(seg, args, name_was_substituted) {
                Ok(triple) => triple,
                Err(barrier) => return *barrier,
            };

        // Lower the body (materialised, static, or empty for
        // dynamic-with-resolved-name), register the Procedure, and emit the
        // runtime `proc` Call.
        // A parameter list Tcl would reject (malformed, an overlong specifier,
        // an array-element or qualified name) creates no procedure: leave it to
        // the runtime `proc`, which raises the error, rather than registering a
        // Procedure whose parameters we guessed at.
        let Some(params) = parse_formal_param_names(
            &args[1],
            WordValueRules::from_config(&self.config),
            self.invocation_dialect
                .and_then(tcl_native_model::InvocationDialect::parameter_grammar),
        ) else {
            return Statement::Barrier {
                span: seg.span,
                reason: "malformed proc params".into(),
                command: seg.name().into(),
                canonical_command: None,
                args: seg.args().to_vec(),
                tokens: Some(self.cmd_tokens(seg)),
            };
        };
        let tokens = self.cmd_tokens(seg);
        let Some(publication) = original_procedure_publication(&tokens) else {
            return self.lower_default(seg, namespace);
        };
        let Some(qualified) = publication.reported_full_name() else {
            return self.lower_default(seg, namespace);
        };
        let body_text = &args[2];
        let body = self.lower_declared_proc_body(
            seg,
            materialised_body,
            body_is_dynamic,
            body_text,
            body_offset,
            namespace,
        );

        // A `proc` defined inside a TclOO method body is created
        // at method-call time in the runtime receiver namespace, not at script
        // load — so it must not be lifted into `module.procedures`
        // (codegen would otherwise emit it unconditionally). The body
        // was still lowered above for analysis; only the premature static
        // registration is suppressed.
        if !self.suppress_proc_register {
            if self
                .module
                .procedures
                .values()
                .any(|previous| previous.publication.as_ref() == Some(&publication))
            {
                self.module.redefined_procedures.insert(qualified.clone());
            }
            self.module.retain_procedure(Procedure {
                name: proc_name.clone(),
                qualified_name: qualified,
                publication: Some(publication),
                declaration_site: tokens
                    .source_binding
                    .as_ref()
                    .and_then(|binding| binding.invocation_site())
                    .cloned(),
                body_kind: crate::ir::ProcedureBodyKind::Named,
                params,
                span: seg.span,
                body,
                params_raw: args[1].clone(),
                body_source: Some(args[2].clone()),
                body_offset,
                namespace_scoped: self.in_namespace_eval,
                base_priority: 500,
            });
        }

        Statement::Call {
            span: seg.span,
            command: seg.name().into(),
            canonical_command: None,
            args: seg.args().to_vec(),
            defs: vec![],
            reads: vec![],
            reads_own_defs: false,
            safe_on_uninit: false,
            tokens: Some(self.cmd_tokens(seg)),
            foreach_groups: None,
        }
    }

    /// Prefer the declaration's exact retained script and execution frame.
    fn lower_declared_proc_body(
        &mut self,
        seg: &SegmentedCommand,
        materialised_body: Option<String>,
        body_is_dynamic: bool,
        body_text: &str,
        body_offset: u32,
        body_namespace: &str,
    ) -> Script {
        let retained = seg
            .arg_tokens()
            .get(2)
            .and_then(|token| {
                self.source_bindings
                    .as_ref()?
                    .executed_script_for_word(token.span)
                    .cloned()
            })
            .or_else(|| {
                self.source_bindings
                    .as_ref()?
                    .deferred_rule_declaration_candidates(seg.span.start())?
                    .iter()
                    .find_map(|candidate| {
                        matches!(
                            candidate.declaration,
                            tcl_registry::events::IrulesTopLevelDeclaration::Procedure { .. }
                        )
                        .then(|| candidate.body.clone())
                        .flatten()
                    })
            })
            .or_else(|| {
                // A declaration replaced before any call never enters its body,
                // yet `proc` created it from this exact script: C Tcl runs it in
                // the declaration's namespace if a caller reaches it first.
                let tokens = self.cmd_tokens(seg);
                let declaration = tokens.source_binding.as_ref()?.invocation_site()?;
                let mut bodies = self
                    .source_bindings
                    .as_ref()?
                    .procedure_implementation_bodies()
                    .filter(|body| &body.allocation.site == declaration);
                let first = bodies.next()?.source.clone();
                bodies.all(|body| body.source == &first).then_some(first)
            });
        if let Some(source) = retained {
            let declaration = self
                .cmd_tokens(seg)
                .source_binding
                .as_ref()
                .and_then(|binding| binding.invocation_site())
                .cloned();
            let body_namespace = self
                .source_bindings
                .as_ref()
                .and_then(|bindings| {
                    let declaration = declaration.as_ref()?;
                    let mut bodies = bindings.procedure_implementation_bodies().filter(|body| {
                        &body.allocation.site == declaration && body.source == &source
                    });
                    let first = bodies.next()?.namespace_key;
                    bodies
                        .all(|body| body.namespace_key == first)
                        .then(|| first.clone())
                })
                .or_else(|| {
                    self.source_bindings
                        .as_ref()?
                        .executed_script_namespace_context(&source)
                });
            self.in_procedure_frame(Some(IrulesExecutionContext::ProcedureBody), |lowerer| {
                lowerer.lower_original_body(source, body_namespace.as_ref())
            })
        } else if self.native_entry.is_some() {
            // A native declaration without retained body geometry cannot be
            // reanalysed under a namespace reconstructed from its label.
            over_depth_script(body_offset, body_text.len())
        } else {
            self.lower_proc_body(
                materialised_body,
                body_is_dynamic,
                body_text,
                body_offset,
                body_namespace,
            )
        }
    }

    /// Lower one procedure body in its fresh procedure frame.
    fn lower_proc_body(
        &mut self,
        materialised_body: Option<String>,
        body_is_dynamic: bool,
        body_text: &str,
        body_offset: u32,
        body_namespace: &str,
    ) -> Script {
        // ``lower_body`` otherwise inherits the enclosing scope's tracked
        // scalars, which is sound for control flow but not a proc's fresh
        // runtime frame.
        self.in_procedure_frame(Some(IrulesExecutionContext::ProcedureBody), |lowerer| {
            if let Some(text) = materialised_body {
                lowerer.lower_script_in_irules_context(
                    &text,
                    body_namespace,
                    IrulesExecutionContext::ProcedureBody,
                )
            } else if body_is_dynamic {
                // The resolved body value is dynamic; retain the runtime call but
                // do not compile the literal `$body` spelling as Tcl source.
                Script::default()
            } else if (lowerer.body_cache.is_some() || lowerer.original_body_cache.is_some()) && {
                lowerer.proc_depth == 1
                    && !lowerer
                        .registry
                        .profile()
                        .is_some_and(tcl_dialect::DialectProfile::is_irules)
                    && lowerer.source_bindings.as_ref().is_some_and(|bindings| {
                        bindings.can_reuse_isolated_body(
                            body_text,
                            body_offset,
                            body_namespace,
                            lowerer.config,
                            lowerer.registry,
                        )
                    })
            } {
                // Only context-free Tcl bodies can use the offset-zero body cache.
                let (mut script, issued) = if let Some(cache) = lowerer.original_body_cache {
                    let template = cache(body_text, body_namespace);
                    let issued = template.matches(
                        body_text,
                        body_namespace,
                        lowerer.registry,
                        lowerer.config,
                        lowerer.dialect,
                    );
                    (template.body, issued)
                } else {
                    (
                        (lowerer.body_cache.expect("selected body cache"))(
                            body_text,
                            body_namespace,
                        ),
                        false,
                    )
                };
                crate::lattice_rebase::rebase_script(&mut script, i64::from(body_offset));
                if lowerer
                    .source_bindings
                    .as_ref()
                    .is_some_and(|bindings| bindings.restore_script_bindings(&mut script))
                {
                    if !issued && let Some(integrity) = lowerer.optimiser_lowering_integrity {
                        integrity.set(false);
                    }
                    script
                } else {
                    lowerer.lower_body_in_irules_context(
                        body_text,
                        body_offset,
                        body_namespace,
                        IrulesExecutionContext::ProcedureBody,
                    )
                }
            } else {
                lowerer.lower_body_in_irules_context(
                    body_text,
                    body_offset,
                    body_namespace,
                    IrulesExecutionContext::ProcedureBody,
                )
            }
        })
    }

    /// Resolve a single variable name from its original Native argument value
    /// or quiet Logical source binding. Unknown and compound names retain
    /// runtime dispatch; literal names keep their original argument vector.
    fn resolve_dynamic_proc_name(
        &mut self,
        seg: &SegmentedCommand,
        args_borrow: &[String],
    ) -> Result<(String, Vec<String>, bool), Box<Statement>> {
        let proc_name_initial = &args_borrow[0];
        if !proc_name_initial.contains('$') && !proc_name_initial.contains('[') {
            return Ok((proc_name_initial.clone(), args_borrow.to_vec(), false));
        }

        let arg_tokens = seg.arg_tokens();
        let single_token_proc_name = seg.single_token_word.get(1).copied().unwrap_or(false);
        let resolved = if !proc_name_initial.contains('[')
            && single_token_proc_name
            && arg_tokens
                .first()
                .is_some_and(|t| t.kind == tcl_lexer::TokenType::Var)
        {
            self.original_native_written_argument_value(seg, 0)
                .or_else(|| self.original_authored_written_argument_value(seg, 0))
                .or_else(|| self.const_map_lookup(proc_name_initial))
        } else {
            None
        };

        let Some(literal) = resolved else {
            return Err(Box::new(Statement::Barrier {
                span: seg.span,
                reason: "dynamic proc name".into(),
                command: seg.name().into(),
                canonical_command: None,
                args: args_borrow.to_vec(),
                tokens: Some(self.cmd_tokens(seg)),
            }));
        };

        let mut args_owned = args_borrow.to_vec();
        args_owned[0].clone_from(&literal);
        Ok((literal, args_owned, true))
    }

    /// Check whether the proc body and/or params are dynamic, and
    /// attempt to materialise the body from the const-map.
    ///
    /// Returns:
    ///   * `Ok((Some(text), _, body_offset))` — body materialised
    ///     successfully; caller should lower `text` as a fresh
    ///     script.
    ///   * `Ok((None, body_is_dynamic, body_offset))` — body is
    ///     static (lower from source) or dynamic-with-name-resolved
    ///     (lower as empty Script).
    ///   * `Err(barrier)` — params dynamic, or body dynamic without
    ///     name substitution.  Caller propagates the barrier.
    fn check_proc_body_dynamic(
        &mut self,
        seg: &SegmentedCommand,
        args: &[String],
        name_was_substituted: bool,
    ) -> Result<(Option<String>, bool, u32), Box<Statement>> {
        let arg_tokens = seg.arg_tokens();
        let params_tok = arg_tokens[1];
        let body_tok = arg_tokens[2];
        // `single_token_word[0]` covers the command itself; args[0..]
        // live at indices `[1..]`.  See `SegmentedCommand::arg_single_token`.
        let body_is_single = seg.single_token_word.get(3).copied().unwrap_or(false);
        let params_is_single = seg.single_token_word.get(2).copied().unwrap_or(false);
        let body_is_dynamic = matches!(
            body_tok.kind,
            tcl_lexer::TokenType::Var | tcl_lexer::TokenType::Cmd
        ) || !body_is_single;
        let params_is_dynamic = matches!(
            params_tok.kind,
            tcl_lexer::TokenType::Var | tcl_lexer::TokenType::Cmd
        ) || !params_is_single;
        let materialised_body: Option<String> =
            if body_is_single && body_tok.kind == tcl_lexer::TokenType::Cmd {
                // `args[2]` is the `word_piece`-reconstructed source
                // — for a Cmd token that's `[subst -nocommands {…}]`
                // (with the brackets re-added).
                // `eval_subst_nocommands_body` segments its input as
                // a top-level Tcl command stream and expects the
                // inner content, so strip the outer `[…]` before
                // handing it over.
                self.original_native_written_argument_value(seg, 2)
                    .or_else(|| self.original_authored_written_argument_value(seg, 2))
                    .or_else(|| {
                        args[2]
                            .strip_prefix('[')
                            .and_then(|s| s.strip_suffix(']'))
                            .and_then(|inner| self.eval_subst_nocommands_body(inner))
                    })
            } else if body_is_single && body_tok.kind == tcl_lexer::TokenType::Var {
                self.original_native_written_argument_value(seg, 2)
                    .or_else(|| self.original_authored_written_argument_value(seg, 2))
                    .or_else(|| self.const_map_lookup(&args[2]))
            } else {
                None
            };
        if params_is_dynamic
            || (body_is_dynamic && materialised_body.is_none() && !name_was_substituted)
        {
            return Err(Box::new(Statement::Barrier {
                span: seg.span,
                reason: if params_is_dynamic {
                    "dynamic proc params"
                } else {
                    "dynamic proc body"
                }
                .into(),
                command: seg.name().into(),
                canonical_command: None,
                args: seg.args().to_vec(),
                tokens: Some(self.cmd_tokens(seg)),
            }));
        }
        let body_offset = body_tok.span.start() + u32::from(body_tok.content_offset);
        Ok((materialised_body, body_is_dynamic, body_offset))
    }
}
