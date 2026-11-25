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

//! `proc`: the declaration's names and metadata, its parameter traits and the walk of its body in a new scope.

use super::super::scope::scope_at_mut;
use super::super::state::Analyser;
use super::super::types::ProcDef;
use super::super::utils::{param_name_spans_for_token, parse_param_list};
use super::finite_foreach_source::FiniteNameLookup;
use super::{qualify, scope_name_for_routine};
use crate::signature_scan::types::ParamDef;
use tcl_lexer::{Span, Token, TokenType};

struct ResolvedProcedureReportingName {
    pub(super) finite: Option<(crate::signature_scan::scope::SignatureSourceCommand, String)>,
    pub(super) resolved: String,
}

/// The three per-proc facts [`Analyser::infer_proc_param_traits`] derives from
/// one view of a proc body: the per-parameter trait map, the caller-frame
/// parameter names ([`super::super::types::ProcDef::caller_frame_params`]), and the
/// literal caller-frame targets
/// ([`super::super::types::ProcDef::caller_frame_literals`], name → written-through-alias).
pub(super) type ProcParamFacts = (
    std::collections::HashMap<String, std::collections::HashSet<super::super::types::ProcArgTrait>>,
    std::collections::HashSet<String>,
    std::collections::HashMap<String, bool>,
);

/// Bundled arguments for [`Analyser::walk_proc_body_in_new_scope`] — kept
/// under clippy's argument-count limit (mirrors `TaintScan` in `taint.rs`).
#[derive(Clone, Copy)]
#[allow(clippy::struct_field_names)]
struct ProcBodyWalkArgs<'a> {
    /// Parent scope path, *before* the new proc scope is pushed.
    pub(super) path: &'a [usize],
    /// The routine's name after [`Analyser::resolve_dynamic_word`] — the
    /// `Scope` name is derived from it via [`scope_name_for_routine`], never
    /// from the raw written word.
    resolved_name: &'a str,
    pub(super) body_span: Span,
    /// The parameter-list word's token.
    params_tok: Token,
    name_tok: Token,
    pub(super) params: &'a [crate::signature_scan::types::ParamDef],
    /// The static-variable list word and its token, when the definer takes one
    /// and the call supplies it.
    statics: Option<(&'a str, Token)>,
    body_text: &'a str,
    body_tok: Token,
    ns_prefix: &'a str,
}

/// One element of a static-variable list.
struct StaticVariable {
    /// The name the body declares.
    pub(super) name: String,
    /// Whether the element refers to the enclosing scope's variable of that
    /// name, which is read when the procedure is defined: a bare `name` or
    /// `&name`. A `{name value}` pair names its own initial value.
    reads_enclosing: bool,
}

/// The variables a static-variable list declares: each element is a bare
/// `name`, a `{name value}` pair, or `&name`, which refers to the enclosing
/// scope's variable of that name and declares the same name in the body.
fn static_variables(
    text: &str,
    rules: tcl_syntax::word_rules::WordValueRules,
) -> Vec<StaticVariable> {
    parse_param_list(text, rules)
        .into_iter()
        .map(|param| {
            let reads_enclosing = !param.has_default;
            match param.name.strip_prefix('&') {
                Some(referenced) => StaticVariable {
                    name: referenced.to_owned(),
                    reads_enclosing,
                },
                None => StaticVariable {
                    name: param.name,
                    reads_enclosing,
                },
            }
        })
        .collect()
}

impl Analyser {
    pub(super) fn procedure_definition_doc(&mut self, body: Option<&str>) -> String {
        let mut doc = std::mem::take(&mut self.last_comment);
        if doc.is_empty()
            && let Some(body) = body
        {
            doc = super::super::utils::extract_body_docstring(body);
        }
        doc
    }

    fn retain_unknown_procedure_info(
        &mut self,
        qualified: &str,
        body: &str,
        params: &[ParamDef],
        body_tok: Token,
    ) {
        if self.defines_global_unresolved_handler(qualified) {
            let info = self.extract_unknown_proc_info_at(
                body,
                params,
                Some(body_tok.span.start() + u32::from(body_tok.content_offset)),
            );
            self.result.unknown_proc_info = Some(info);
        }
    }

    pub(super) fn procedure_reporting_names(
        &self,
        original_name: Option<&crate::signature_scan::original_name::SourceOriginalNameOccurrence>,
        name_word: Token,
        scope_path: &[usize],
        ns_prefix: &str,
        resolved_name: &str,
    ) -> (
        Option<crate::signature_scan::scope::SignatureSourceCommand>,
        String,
        String,
    ) {
        let source_name = original_name.and_then(|original| {
            crate::signature_scan::scope::SignatureSourceCommand::procedure_from_key(
                &self.declaration_namespace_scope(scope_path)?,
                original.name_input(),
            )
        });
        let source_name = source_name
            .or_else(|| self.original_interp_procedure_source_name(name_word, scope_path));
        let qualified = source_name
            .as_ref()
            .and_then(|name| {
                match super::super::scope::scope_at(&self.result.global_scope, scope_path)
                    .and_then(|scope| scope.original_interpreter_source_domain.as_ref())
                {
                    Some(domain) => domain.reported_command(name.slot()),
                    None => name.reported_full_name(),
                }
            })
            .unwrap_or_else(|| qualify(ns_prefix, resolved_name));
        let simple = source_name
            .as_ref()
            .and_then(crate::signature_scan::scope::SignatureSourceCommand::simple_name)
            .unwrap_or_else(|| crate::naming::key_tail(&qualified).to_string());
        (source_name, qualified, simple)
    }

    fn declared_procedure_parameters(
        &self,
        text: &str,
        token: Token,
        single: bool,
    ) -> (Vec<ParamDef>, bool) {
        let params_computed =
            !crate::signature_scan::params::param_word_is_literal(token.kind, single);
        let params = if params_computed {
            Vec::new()
        } else {
            parse_param_list(text, self.word_rules())
        };
        (params, params_computed)
    }

    fn declared_procedure_formal_count(
        &self,
        token: Token,
    ) -> crate::signature_scan::formal_count::SourceFormalCount {
        use crate::signature_scan::formal_count::SourceFormalCount;
        let Some(dialect) = self.original_source_invocation_dialect() else {
            return SourceFormalCount::Unknown;
        };
        if dialect.authored_name_policy().is_none() {
            return SourceFormalCount::Authored(
                dialect
                    .parameter_grammar()
                    .unwrap_or(tcl_dialect::ParameterGrammar::Tcl),
            );
        }
        self.original_static_source_name_at_span(token.span)
            .map_or(SourceFormalCount::Unknown, |original| {
                SourceFormalCount::from_original_input(original.name_input(), dialect)
            })
    }

    pub(super) fn original_procedure_declaration_name(
        &self,
        name_tok: Token,
    ) -> Option<crate::signature_scan::original_name::SourceOriginalNameOccurrence> {
        self.declaration_name_policy().and_then(|policy| {
            self.head_identities
                .original_source_name_at_span(
                    name_tok.span,
                    self.file_lexer_config(),
                    self.word_rules(),
                    policy,
                )
                .or_else(|| {
                    // The selected procedure role supplies declaration advice;
                    // retaining its original static word requires no completed
                    // command world or reached installation.
                    let original = self.original_static_source_name_at_span(name_tok.span)?;
                    let key = original.name_input();
                    (key.policy() == policy
                        && key.lexer_config() == self.file_lexer_config()
                        && key.word_value_rules() == self.word_rules())
                    .then(|| original.clone())
                })
        })
    }

    pub(super) fn retain_procedure_declaration_metadata(
        &mut self,
        original_name: Option<&crate::signature_scan::original_name::SourceOriginalNameOccurrence>,
        proc: &ProcDef,
        name_span: Span,
    ) {
        if self.selected_logical_declaration_advice()
            && let Some(declaration) =
                super::super::types::LogicalSourceProcedureDeclaration::capture(
                    &self.result,
                    &self.source,
                    self.resolved_analysis_input(),
                    proc,
                )
        {
            self.result.logical_procedure_declarations.push(declaration);
        }
        if let (Some(original), Some(name)) = (
            crate::registry_invocation::source_structure::trace_jim_source_option(
                "jim.proc.original_name",
                original_name,
            ),
            crate::registry_invocation::source_structure::trace_jim_source_option(
                "jim.proc.source_name",
                proc.source_name.as_ref(),
            ),
        ) && let Some(record) =
            crate::registry_invocation::source_structure::trace_jim_source_option(
                "jim.proc.declaration_record",
                crate::signature_scan::original_name::SourceDeclarationMetadata::new(
                    original,
                    name.clone(),
                    proc.clone(),
                ),
            )
            && !self.result.original_procedure_metadata.contains(&record)
        {
            self.result.original_procedure_metadata.push(record);
        }
        if let Some(original) = self.result.original_vendor_source_name_in_source(
            &tcl_lexer::SourceImage::document(&self.source),
            self.file_lexer_config(),
            name_span,
        ) {
            let record = crate::signature_scan::vendor_name::VendorSourceDeclarationMetadata::new(
                original,
                tcl_syntax::naming::VendorSourceNamePurpose::ProcedureName,
                proc.clone(),
            );
            if !self
                .result
                .original_vendor_procedure_metadata
                .contains(&record)
            {
                self.result.original_vendor_procedure_metadata.push(record);
            }
        }
    }

    /// Handle the `proc` command: `proc NAME PARAMS BODY`.
    ///
    /// Returns `true` when the command was handled (callers use the
    /// bool to decide whether further processing is needed), `false`
    /// when the input doesn't match the expected shape.
    ///
    /// Records the [`ProcDef`] in
    /// both `scope.procs` (keyed by simple name) and
    /// `result.all_procs` (keyed by qualified name).  When the
    /// body is a braced literal, opens a fresh
    /// [`super::super::types::ScopeKind::Proc`] child scope, defines each parameter
    /// in it, and re-segments the body via
    /// [`crate::segmenter::segment_commands_with_offset`] —
    /// every body command is dispatched through
    /// [`Analyser::process_command`] with the new scope path.
    /// Body recursion does **not** invoke segmenter recovery —
    /// that fires only at the top level.
    /// Dynamic bodies (`$body`, `[gen]`) skip the walk because
    /// they cannot be statically re-segmented; the proc record
    /// is still emitted so downstream consumers see the
    /// signature.
    ///
    /// Infer per-parameter traits for a proc body.  Always runs
    /// the shallow pass (`infer_param_traits`); when
    /// [`Self::deep_param_traits`] is set, also runs the
    /// recursive deep pass (`infer_param_traits_deep`) and
    /// unions both via [`super::super::param_traits::merge_traits`].
    /// Threads the analyser's pre-built dialect-aware registry
    /// through so iRules-only `arg_role_resolver` callbacks
    /// (e.g. `when`) fire on body args.  When [`Self::registry`]
    /// is `None` (outside an active `analyse` run, e.g. a unit-
    /// test harness) we skip the inference rather than pay the
    /// cost of a fresh `build_default` on every proc.
    ///
    /// Returns the trait map together with
    /// [`ProcDef::caller_frame_params`](super::super::types::ProcDef::caller_frame_params)
    /// — the strictly narrower "this parameter's value names a variable in the
    /// *immediate caller's* frame" fact, which the trait map alone cannot
    /// answer because it records no frame level — and
    /// [`ProcDef::caller_frame_literals`](super::super::types::ProcDef::caller_frame_literals),
    /// the literal caller-frame names the body spells itself (`upvar 1 name
    /// name`).  Computed here, from the same
    /// body text and the same dialect config, so the three can never be
    /// derived from different views of the proc.
    pub(super) fn infer_proc_param_traits(
        &self,
        params: &[crate::signature_scan::types::ParamDef],
        body_text: &str,
        body_span: tcl_lexer::Span,
    ) -> ProcParamFacts {
        let Some(registry) = self.registry.as_deref() else {
            return (
                std::collections::HashMap::new(),
                std::collections::HashSet::new(),
                std::collections::HashMap::new(),
            );
        };
        let param_names: Vec<&str> = params.iter().map(|p| p.name.as_str()).collect();
        // One environment for all four scans, so the traits, the caller-frame
        // params, and the caller-frame literals can never be derived from
        // different views of the proc *or* of the document's command bindings.
        let input = self.resolved_analysis_input();
        let original_logical_body = self
            .head_identities
            .original_logical_procedure_body_cards(&input)
            .and_then(|cards| {
                let mut matching = cards.into_iter().filter(|body| {
                    body.matches_source(&tcl_lexer::SourceImage::document(&self.source), &input)
                        && body.scope().scope_body().original_container().group().span == body_span
                        && body
                            .parameters()
                            .iter()
                            .map(|formal| formal.name.as_str())
                            .eq(param_names.iter().copied())
                });
                let body = matching.next()?;
                matching.next().is_none().then_some(body)
            });
        let env = super::super::param_traits::TraitScanEnv {
            surface: tcl_registry::model::DocumentCommandSurface::new(
                registry,
                self.declared_commands.as_ref(),
            ),
            config: self.file_lexer_config(),
            identities: &self.head_identities,
            executed_source: self.head_identities.executed_script_for_word(body_span),
            original_logical_body: original_logical_body.as_ref(),
        };
        let original_body_offset = self.result.retained_command_realm().and_then(|realm| {
            let config = self.file_lexer_config();
            let image = realm
                .original_source_image()
                .filter(|image| image.bytes() == self.source.as_bytes())?;
            let (word, _) =
                realm.original_written_word_at_span_in_source(image, body_span, config)?;
            word.content_span().ok().map(tcl_lexer::Span::start)
        });
        #[cfg(debug_assertions)]
        if tcl_dialect::trace_env::var_os("TCL_LSP_TRACE_ORIGINAL_PARAM_ROLES").is_some() {
            eprintln!(
                "ORIGINAL_PARAM_ROLES begin body={}..{} parameters={} executed_source={} deep={}",
                body_span.start(),
                body_span.end(),
                params.len(),
                env.executed_source.is_some(),
                self.deep_param_traits
            );
        }
        let body_text = match env.executed_source {
            Some(source) => {
                let Ok(text) = source.try_text() else {
                    return (
                        std::collections::HashMap::default(),
                        std::collections::HashSet::default(),
                        std::collections::HashMap::default(),
                    );
                };
                text
            }
            None => body_text,
        };
        let super::super::param_traits::ShallowProcedureTraits {
            traits: shallow,
            caller_frame_params,
            caller_frame_literals,
        } = super::super::param_traits::infer_param_traits_with_caller_frame(
            &param_names,
            body_text,
            env,
        );
        let traits = if self.deep_param_traits {
            let deep = super::super::param_traits::infer_param_traits_deep_in_document(
                &param_names,
                body_text,
                env,
                &self.source,
                &self.result,
                original_body_offset,
            );
            super::super::param_traits::merge_traits(shallow, deep)
        } else {
            shallow
        };
        #[cfg(debug_assertions)]
        if tcl_dialect::trace_env::var_os("TCL_LSP_TRACE_ORIGINAL_PARAM_ROLES").is_some() {
            eprintln!(
                "ORIGINAL_PARAM_ROLES end body={} traits={} caller_parameters={} caller_literals={}",
                body_span.start(),
                traits.len(),
                caller_frame_params.len(),
                caller_frame_literals.len()
            );
        }
        (traits, caller_frame_params, caller_frame_literals)
    }

    fn procedure_words_for_analyser(
        &self,
        command: &str,
        args: &[String],
        arg_tokens: &[Token],
        source_head: Option<Token>,
    ) -> Option<tcl_registry::ProcedureWords> {
        if let Some(head) = source_head {
            let words = self.original_procedure_source_words_at(head, args, arg_tokens);
            // The interpretation never enters a child interpreter's script
            // or the recovered tail of a document that did not parse, and the
            // lowering keeps no command inside a concatenated eval-family
            // script, so a declaration there has no retained words: its
            // layout is the written head's in the registry. A computed name
            // stays refused, since its value may name another namespace.
            let unreached_script = (!self.interp_path_stack.is_empty()
                || self.recovered_source
                || self.concatenated_script)
                && args
                    .first()
                    .is_some_and(|name| !crate::naming::is_dynamic_word(name))
                && !self
                    .head_identities
                    .source_bindings_ref()
                    .reached_site(head.span.start());
            if words.is_some() || !unreached_script {
                return words;
            }
        }
        // Legacy direct callers retain their existing Native-shape projection.
        let arg_words: Vec<&str> = args.iter().map(String::as_str).collect();
        Some(
            self.registry
                .as_deref()
                .and_then(|registry| registry.procedure_definition_words(command, &arg_words))
                .unwrap_or(tcl_registry::ProcedureWords::TCL_PROC),
        )
    }

    pub(super) fn original_procedure_source_words_at(
        &self,
        head: Token,
        args: &[String],
        arg_tokens: &[Token],
    ) -> Option<tcl_registry::ProcedureWords> {
        use crate::registry_invocation::source_structure::trace_jim_source_option as observed;
        let words = observed(
            "analyser.proc.words",
            crate::registry_invocation::source_structure::source_registry_words_at(
                &self.source,
                &self.result,
                head.span.start(),
            ),
        )?;
        Self::original_procedure_source_arguments_match(&words, args, arg_tokens)?;
        let context = self.analysis_context();
        let positions = observed(
            "analyser.proc.schema",
            words.with_source_schema(&context, |schema| {
                observed(
                    "analyser.proc.hook",
                    (schema.semantics.analyser_hook
                        == Some(tcl_registry::hooks::AnalyserHookId::Proc))
                    .then_some(()),
                )?;
                observed(
                    "analyser.proc.shape",
                    schema.authored_source_procedure_arguments(),
                )
            }),
        )??;
        let mut statics = observed("analyser.proc.roles", words.roles())?
            .iter()
            .filter_map(|&(position, role)| {
                (role == tcl_registry::ArgRole::StaticVarList).then_some(position)
            });
        let static_at = statics.next();
        if statics.next().is_some() {
            observed::<()>("analyser.proc.statics", None);
            return None;
        }
        let required = [positions.name, positions.parameters, positions.body];
        if required.iter().any(|&index| index >= args.len())
            || required[0] == required[1]
            || required[0] == required[2]
            || required[1] == required[2]
            || static_at.is_some_and(|index| index >= args.len() || required.contains(&index))
        {
            observed::<()>("analyser.proc.positions", None);
            return None;
        }
        observed(
            "analyser.proc.complete",
            Some(tcl_registry::ProcedureWords {
                name: positions.name,
                params: positions.parameters,
                statics: static_at,
                body: positions.body,
            }),
        )
    }

    fn original_procedure_source_arguments_match(
        words: &crate::registry_invocation::source_structure::OriginalRegistryWords,
        args: &[String],
        arg_tokens: &[Token],
    ) -> Option<()> {
        use crate::registry_invocation::source_structure::trace_jim_source_option as observed;
        if words.arguments().len() != args.len() || arg_tokens.len() != args.len() {
            observed::<()>("analyser.proc.argument_count", None);
            return None;
        }
        for (index, (text, token)) in args.iter().zip(arg_tokens).enumerate() {
            if words.origins().get(index + 1)
                != Some(&crate::registry_invocation::InvocationWordOrigin::Written(
                    index + 1,
                ))
            {
                observed::<()>("analyser.proc.origin", None);
                return None;
            }
            let word = observed("analyser.proc.word", words.original_argument_word(index))?;
            if word.group().expand {
                observed::<()>("analyser.proc.expansion", None);
                return None;
            }
            let projection = crate::segmenter::segment_commands_with_offset_and_config(
                observed("analyser.proc.text", word.try_text().ok())?,
                word.word_span().start(),
                word.config().nested(),
            );
            let [original] = projection.as_slice() else {
                observed::<()>("analyser.proc.projection", None);
                return None;
            };
            if observed(
                "analyser.proc.argv",
                (original.argv == [*token]).then_some(()),
            )
            .is_none()
                || observed(
                    "analyser.proc.value",
                    (original.texts == [text.as_str()]).then_some(()),
                )
                .is_none()
                || observed(
                    "analyser.proc.tokens",
                    (original.all_tokens == word.tokens()).then_some(()),
                )
                .is_none()
            {
                return None;
            }
        }
        Some(())
    }

    /// Handle a `proc name params body` definition: record the procedure
    /// (name, parameter list, body, and harvested doc-comment) in the
    /// analysis result and run per-parameter trait inference. Returns `true`
    /// when the definition had the full three-argument shape this handler
    /// consumes.
    ///
    /// Dispatched via [`tcl_registry::hooks::AnalyserHookId::Proc`].
    pub fn handle_proc_command(
        &mut self,
        cmd_name: &str,
        args: &[String],
        arg_tokens: &[Token],
        arg_single: &[bool],
        scope_path: &[usize],
    ) -> bool {
        self.handle_proc_command_with_source_head(
            cmd_name, args, arg_tokens, arg_single, scope_path, None,
        )
    }

    pub(in super::super) fn handle_proc_command_at(
        &mut self,
        cmd_name: &str,
        args: &[String],
        arg_tokens: &[Token],
        arg_single: &[bool],
        scope_path: &[usize],
        source_head: Token,
    ) -> bool {
        self.handle_proc_command_with_source_head(
            cmd_name,
            args,
            arg_tokens,
            arg_single,
            scope_path,
            Some(source_head),
        )
    }

    fn resolve_proc_name(
        &self,
        raw_name: &str,
        name_tok: Token,
        arg_single: &[bool],
        scope_path: &[usize],
        source_head: Option<Token>,
    ) -> Option<ResolvedProcedureReportingName> {
        let finite_name = match self.finite_foreach_names.lookup(
            source_head,
            name_tok,
            scope_path,
            self.body_depth,
        ) {
            FiniteNameLookup::Ordinary => None,
            FiniteNameLookup::Refused => return None,
            FiniteNameLookup::Candidate(receipt) => {
                Some(receipt.reborrow(self, name_tok, scope_path)?)
            }
        };
        // A constant-foldable dynamic name (`proc ::$wtype {args} {...}` with
        // `wtype` a known constant) resolves the same way `rename`'s operands
        // already do (`tk/library/accessibility.tcl`'s
        // rename-away-and-reinstall idiom names its wrapper proc this way).
        // An unresolvable dynamic name falls back to the raw written text
        // unchanged — this only *improves* the resolvable case, the same
        // scope boundary `resolve_dynamic_word`'s other callers keep.
        let resolved_name = finite_name.as_ref().map_or_else(
            || {
                self.resolve_dynamic_word(
                    raw_name,
                    Some(name_tok),
                    arg_single.first().copied().unwrap_or(false),
                    scope_path,
                )
                .or_else(|| self.literal_foreach_name(raw_name, name_tok))
                .unwrap_or_else(|| raw_name.to_owned())
            },
            |(_, text)| text.clone(),
        );
        Some(ResolvedProcedureReportingName {
            finite: finite_name,
            resolved: resolved_name,
        })
    }

    /// A name word whose only substitutions read the variable of the literal
    /// `foreach` iteration being walked, spelled with that iteration's
    /// element: `foreach c {a {bar baz}} {proc ::$c {} {}}` declares `::a`
    /// and `::bar baz` (tclsh 8.6.18, 9.0.4).
    fn literal_foreach_name(&self, text: &str, name: Token) -> Option<String> {
        let (variable, value, _) =
            self.literal_foreach_bindings
                .iter()
                .rev()
                .find(|(_, _, body)| {
                    body.start() <= name.span.start() && name.span.end() <= body.end()
                })?;
        let text = text.replace(&format!("${{{variable}}}"), value);
        let reference = format!("${variable}");
        let mut spelled = String::with_capacity(text.len());
        let mut rest = text.as_str();
        while let Some(at) = rest.find(&reference) {
            let after = &rest[at + reference.len()..];
            spelled.push_str(&rest[..at]);
            if after
                .chars()
                .next()
                .is_some_and(|next| next.is_ascii_alphanumeric() || matches!(next, '_' | ':' | '('))
            {
                spelled.push_str(&reference);
            } else {
                spelled.push_str(value);
            }
            rest = after;
        }
        spelled.push_str(rest);
        (!crate::naming::is_dynamic_word(&spelled)).then_some(spelled)
    }

    fn procedure_reporting_body_span(&self, body: Token, finite: bool) -> Span {
        if finite {
            tcl_lexer::word_span_at(&*self.source, body.span)
        } else {
            body.span
        }
    }

    fn handle_proc_command_with_source_head(
        &mut self,
        cmd_name: &str,
        args: &[String],
        arg_tokens: &[Token],
        arg_single: &[bool],
        scope_path: &[usize],
        source_head: Option<Token>,
    ) -> bool {
        // iRules procedures are declaration-only, exact three-word forms at
        // file scope. An invalid declaration is consumed but deliberately
        // contributes no symbol, binding, scope, or executable body facts.
        if self.profile.is_irules()
            && !self.is_valid_irules_procedure_declaration(args, arg_tokens, arg_single)
        {
            return true;
        }
        let Some(words) =
            self.procedure_words_for_analyser(cmd_name, args, arg_tokens, source_head)
        else {
            return false;
        };
        let last = words.name.max(words.params).max(words.body);
        if args.len() <= last || arg_tokens.len() <= last {
            return false;
        }

        let raw_name = &args[words.name];
        let name_tok = arg_tokens[words.name];
        let Some(reporting) =
            self.resolve_proc_name(raw_name, name_tok, arg_single, scope_path, source_head)
        else {
            return false;
        };
        let ResolvedProcedureReportingName {
            finite: finite_name,
            resolved: resolved_name,
        } = reporting;
        // Home the proc to the *command-resolution* namespace, not the purely
        // lexical one: a proc defined inside another proc's body homes to that
        // enclosing proc's **defining** namespace (the prefix of its qualified
        // name), the way Tcl resolves the `proc` command at run time.  The
        // purely lexical namespace walk skips proc scopes and so homed a
        // nested `proc helper` under `proc a::outer` to `::helper`, overwriting
        // the real global `::helper` in `all_procs`; the command-resolution
        // namespace homes it to `::a::helper`.
        let ns_prefix = self.command_resolution_namespace(scope_path);
        // `qualify` takes the constructed (rooted) namespace key verbatim and
        // `key_tail` inverts the construction — a `char`-pattern colon trim or
        // an `rsplit("::")` here would collapse a lone-colon name.
        let original_name = self.original_procedure_declaration_name(name_tok);
        let (mut source_name, mut qualified, mut simple) = self.procedure_reporting_names(
            original_name.as_ref(),
            name_tok,
            scope_path,
            &ns_prefix,
            &resolved_name,
        );
        let finite_reporting = finite_name.is_some();
        if let Some((name, _)) = finite_name {
            let (Some(full), Some(tail)) = (name.reported_full_name(), name.simple_name()) else {
                return false;
            };
            source_name = Some(name);
            qualified = full;
            simple = tail;
        }
        let name_span = name_tok.span;
        let body_tok = arg_tokens[words.body];
        let body_span = self.procedure_reporting_body_span(body_tok, finite_reporting);

        // **W113** — proc name shadows a built-in command.
        self.emit_w113_proc_shadows_builtin(
            &resolved_name,
            &qualified,
            name_span,
            source_name.as_ref(),
        );
        // **W314** — the name has no absolute written form.
        self.emit_w314_no_absolute_name(raw_name, name_span);

        // A **computed** parameter-list word (`proc p [makeargs] {…}`,
        // `proc q $params {…}`) builds the formals from a run-time value, so
        // nothing about them is knowable: the proc's params are unmodelled.
        // Reading the unresolved word as a one-parameter
        // literal registered a `VarDef` literally named `"[makeargs]"` and
        // made the call-site arity checker demand exactly one argument, which
        // tclsh 9.0.4 / 8.6.16 contradict (`proc makeargs {} {return {a b}}`;
        // `proc p [makeargs] {…}`; `info args p` → `a b`; `p 1 2` runs).
        //
        // The literalness rule itself lives in
        // `signature_scan::params::param_word_is_literal` — the one predicate
        // this tier, the signature-scan tier, and the LSP's cursor classifier
        // all share.
        let (params, params_computed) = self.declared_procedure_parameters(
            &args[words.params],
            arg_tokens[words.params],
            arg_single.get(words.params).copied().unwrap_or(true),
        );
        // Doc string: prefer the preceding-comment harvest from
        // the segmenter; fall back to ``extract_body_docstring``
        // (leading comment block at the top of the body).
        let doc = self.procedure_definition_doc(Some(&args[words.body]));

        // When a user defines the *global* unresolved-command handler,
        // inspect the body to determine which commands it can resolve. The
        // result gates W123 (unresolved command) file-wide — with a
        // user-supplied handler in place we cannot statically prove a
        // command is truly unresolved.
        self.retain_unknown_procedure_info(&qualified, &args[words.body], &params, body_tok);

        let body_text = &args[words.body];
        let (param_traits, caller_frame_params, caller_frame_literals) =
            self.infer_proc_param_traits(&params, body_text, body_tok.span);

        let proc = ProcDef {
            source_name,
            name: simple,
            qualified_name: qualified.clone(),
            params: params.clone(),
            params_computed,
            formal_count: self.declared_procedure_formal_count(arg_tokens[words.params]),
            name_span,
            body_span,
            doc,
            param_traits,
            caller_frame_params,
            caller_frame_literals,
        };

        // Register globally and in the current scope. ``scope.procs``
        // is keyed by the *simple* (unqualified) proc name (so
        // per-scope lookup and shadowing rules work locally), while
        // ``result.all_procs`` is keyed by the fully-qualified
        // name. The full qualified name is still on
        // ``ProcDef.qualified_name`` for callers that need it.
        self.retain_procedure_declaration_metadata(original_name.as_ref(), &proc, name_span);
        self.register_proc_definition(&qualified, &proc, name_span);
        self.record_two_word_proc_member(&resolved_name, &proc, scope_path);
        let simple_key = proc.name.clone();
        let path = scope_path.to_vec();
        if let Some(scope) = scope_at_mut(&mut self.result.global_scope, &path) {
            scope.procs.insert(simple_key.clone(), proc);
        }
        // Original command-transition source advice tracks this definition
        // independently of the reporting procedure table.

        // Walk the body in a fresh proc scope when the body is a
        // braced literal. The *resolved* name is the proc-scope name:
        // ``define_var`` keys ``result.all_variables`` on
        // ``"<scope_name>::<var>"``, and the scope name is also what
        // ``advance_command_resolution_namespace`` reads the body's namespace
        // off — see ``scope_name_for_routine``.
        if body_tok.kind == TokenType::Str {
            self.walk_proc_body_in_new_scope(ProcBodyWalkArgs {
                path: &path,
                resolved_name: &resolved_name,
                body_span,
                params_tok: arg_tokens[words.params],
                name_tok,
                params: &params,
                statics: words.statics.map(|at| (args[at].as_str(), arg_tokens[at])),
                body_text: &args[words.body],
                body_tok,
                ns_prefix: &ns_prefix,
            });
        }

        true
    }

    fn is_valid_irules_procedure_declaration(
        &self,
        args: &[String],
        arg_tokens: &[Token],
        arg_single: &[bool],
    ) -> bool {
        if self.body_depth != 0 {
            return false;
        }
        let words: Vec<&str> = args.iter().map(String::as_str).collect();
        let generation;
        let registry = if let Some(stashed) = self.registry.as_deref() {
            stashed
        } else {
            generation = self.analysis_context();
            generation.commands()
        };
        let closed = tcl_registry::events::closed_braced_argument_words(
            &self.source,
            arg_tokens,
            arg_single,
        );
        closed
            .as_deref()
            .and_then(|closed| {
                tcl_registry::events::IrulesDeclarationArguments::new(
                    &words, arg_tokens, arg_single, closed,
                )
            })
            .and_then(|arguments| {
                registry.irules_top_level_declaration(
                    "proc",
                    arguments,
                    &tcl_registry::events::EventRegistry::build(),
                )
            })
            .is_some_and(|declaration| {
                matches!(
                    declaration,
                    tcl_registry::events::IrulesTopLevelDeclaration::Procedure { .. }
                )
            })
    }

    /// Record one `proc` declaration into the whole-document tables —
    /// `all_procs` (keyed by qualified name, last redefinition winning, which
    /// is plain Tcl's own semantics), the never-deduplicated
    /// `proc_declaration_sites` span list, and, when this declaration
    /// displaces an earlier one of the same qualified name,
    /// `superseded_procs`.
    ///
    /// The displaced definition is kept because "last wins" is only true
    /// *from the second declaration onward*: a call written between the two
    /// reaches the first one, with the first one's span and parameter list.
    /// Oracle (tclsh 8.6.16 and 9.0.4): with `proc p {} {return first}`,
    /// `p` between the declarations returns `first`, and after a later
    /// `proc p {a} {…}` the same bare `p` fails `wrong # args: should be "p
    /// a"`. Dropping the first definition on the map insert made
    /// go-to-definition from the in-between call jump to the *later* header
    /// and left its arity unknowable.
    ///
    /// `superseded_procs` stays empty for every document without a
    /// redefinition, so the common case pays one `HashMap::insert` return
    /// value and nothing else.
    pub(super) fn register_proc_definition(
        &mut self,
        qualified: &str,
        proc: &ProcDef,
        name_span: tcl_lexer::Span,
    ) {
        if let Some(previous) = self.result.all_procs.insert(qualified.into(), proc.clone()) {
            self.result
                .superseded_procs
                .entry(qualified.to_string())
                .or_default()
                .push(previous);
        }
        self.result
            .proc_declaration_sites
            .push((qualified.to_string(), name_span));
    }

    /// Register the same physical procedure scope for ordinary and optlist
    /// definers, before either an inline walk or a deferred body is scheduled.
    pub(super) fn push_proc_body_scope(
        &mut self,
        path: &[usize],
        name: &str,
        span: Span,
    ) -> Vec<usize> {
        let parent = scope_at_mut(&mut self.result.global_scope, path)
            .expect("scope_path resolved when registering proc must still resolve");
        let mut child =
            super::super::types::Scope::new(super::super::types::ScopeKind::Proc, name.to_owned());
        child.body_span = Some(span);
        child
            .original_interpreter_source_domain
            .clone_from(&parent.original_interpreter_source_domain);
        child.naming_scope = parent
            .procs
            .values()
            .find(|proc| proc.body_span == span)
            .and_then(|proc| proc.source_name.as_ref())
            .and_then(crate::signature_scan::scope::SignatureSourceCommand::body_scope);
        let index = parent.children.len();
        parent.children.push(child);
        let mut child_path = path.to_vec();
        child_path.push(index);
        child_path
    }

    /// Walks a `proc`'s body in a freshly created child scope: binds formal
    /// parameters as locals, then recurses into the body (or, on the
    /// per-item shell pass, defers it). Split out of
    /// [`Self::handle_proc_command`] to keep that function's line count
    /// down; `ctx.path` is the *parent* scope path (before the new proc
    /// scope is pushed).
    fn walk_proc_body_in_new_scope(&mut self, ctx: ProcBodyWalkArgs<'_>) {
        let ProcBodyWalkArgs {
            path,
            resolved_name,
            body_span,
            params_tok,
            name_tok,
            params,
            statics,
            body_text,
            body_tok,
            ns_prefix,
        } = ctx;
        // One scope key for both the inline and the deferred path — a
        // divergence here would make the per-item and whole-file walks
        // disagree about which namespace the body runs in.
        let scope_name = scope_name_for_routine(resolved_name);
        let child_path = self.push_proc_body_scope(path, scope_name, body_span);
        self.record_original_vendor_variable_body(
            name_tok.span,
            body_tok.span,
            Some(params_tok.span),
            crate::signature_scan::vendor_variable::VendorSourceVariableBodyKind::Procedure,
        );

        // Parameters become locals in the proc scope. Each param's
        // definition range is anchored to its *name* in the param-list
        // literal so go-to-definition / references / rename on
        // a formal parameter resolve to the parameter, not the proc name.
        // The spans are recovered from the raw param-list word token; any
        // param whose name can't be located falls back to the proc name
        // token.
        let param_spans = param_name_spans_for_token(&self.source, params_tok);
        for (i, p) in params.iter().enumerate() {
            self.define_var(
                &p.name,
                name_tok,
                &child_path,
                false,
                param_spans.get(i).copied(),
            );
        }
        self.emit_w218_args_not_final(params, &param_spans, params_tok);

        // Static variables are locals of the body too: declared once when the
        // procedure is defined, kept between calls, and never an argument.
        let static_names = statics.map_or_else(Vec::new, |(text, tok)| {
            let variables = static_variables(text, self.word_rules());
            let spans = param_name_spans_for_token(&self.source, tok);
            for (i, variable) in variables.iter().enumerate() {
                let span = spans.get(i).copied();
                self.define_var(&variable.name, tok, &child_path, false, span);
                // The definition reads the enclosing variable, so it is used
                // there whether or not the body reads its own copy.
                if variable.reads_enclosing
                    && let Some(span) = span
                {
                    let named = self.source.get(span.as_range()).unwrap_or_default();
                    let skipped = u32::try_from(named.len() - named.trim_start_matches('&').len())
                        .unwrap_or(0);
                    self.record_var_read(
                        &variable.name,
                        Span::new(span.start() + skipped, span.end()),
                        path,
                    );
                }
            }
            variables
                .into_iter()
                .map(|variable| variable.name)
                .collect()
        });

        // Save / restore `last_comment` around the body walk so a
        // doc-comment inside the proc body doesn't bleed to whatever
        // follows the proc at the outer scope. Same treatment for
        // `current_event`: a `proc` body is a new call frame, so W142's
        // event-body-only bare-`return` restriction must not leak in from
        // an enclosing `when` — see `emit_w142_context_gate`'s doc comment.
        let (saved_comment, saved_event) = (
            std::mem::take(&mut self.last_comment),
            self.current_event.take(),
        );

        // Body recursion via the shared helper.  Re-segments
        // the body (no recovery — top-level only) and
        // dispatches each command at the new proc scope path.
        // Per-item shell pass: defer the body (its scope is already
        // created with params; a second pass fills it in place).
        let body_text: std::sync::Arc<str> = std::sync::Arc::from(body_text);
        if self.defer_proc_bodies {
            let safe_interp_ctx = self.safe_interp_ctx_snapshot();
            self.deferred_bodies
                .push(super::super::per_item::DeferredBody {
                    resolved_input: Some(self.resolved_analysis_input()),
                    body_text,
                    body_tok,
                    scope_path: child_path.clone(),
                    is_method: false,
                    oo_global_resolution: false,
                    namespace: ns_prefix.to_string(),
                    scope_name: scope_name.to_string(),
                    params: params.to_vec(),
                    seeded_variables: static_names,
                    // Attached later by `fill_deferred_bodies` for bodies with a
                    // fold candidate.
                    command_trust: None,
                    ensemble_targets: Vec::new(),
                    prefixless_ensembles: Vec::new(),
                    oo_defining_class: None,
                    safe_interp_ctx,
                });
        } else {
            self.analyse_body(&body_text, body_tok, &child_path);
        }

        (self.last_comment, self.current_event) = (saved_comment, saved_event);
    }

    /// Whether a `proc` defined as `qualified` is the interpreter's *global*
    /// unresolved-command handler — the one whose presence gates W123 for the
    /// whole file.
    ///
    /// Which command is the handler comes from
    /// [`tcl_registry::Traits::UNRESOLVED_COMMAND_HANDLER`], never a literal
    /// name here — the same registry query `tcl_compiler::unit_scope` uses for
    /// the interprocedural call-site seed.
    ///
    /// **Global only.** Tcl consults `::unknown` for a bare unresolved word
    /// regardless of the calling namespace, so a namespace-local
    /// `proc unknown` inside `namespace eval ::mylib { … }` is an ordinary
    /// proc that happens to share the name and must not suppress anything.
    /// tclsh8.6.14 and tclsh9.0.4 both confirm the split: with a global
    /// `proc unknown {args} {return handled}` a call to
    /// `totallyBogusCommand` returns `handled`, while with the same proc
    /// defined inside `namespace eval ::mylib` it still fails
    /// `invalid command name "totallyBogusCommand"`. (`namespace unknown
    /// NAME` registers a per-namespace handler explicitly; that path is
    /// modelled separately by its handler argument's
    /// [`tcl_registry::ArgRole::CommandPrefix`] role.)
    ///
    /// `::tcl::unknown` is admitted alongside `::unknown`: it is the Tcl
    /// library's own handler spelling, installed as the interpreter-wide
    /// handler rather than scoped to callers inside `::tcl`.
    fn defines_global_unresolved_handler(&self, qualified: &str) -> bool {
        let Some(registry) = self.registry.as_deref() else {
            return false;
        };
        let mut carriers =
            registry.commands_with_trait(tcl_registry::Traits::UNRESOLVED_COMMAND_HANDLER);
        carriers.sort_unstable();
        carriers.iter().any(|name| {
            qualified == crate::naming::qualify("::", name)
                || qualified == crate::naming::qualify("::tcl", name)
        })
    }
}
