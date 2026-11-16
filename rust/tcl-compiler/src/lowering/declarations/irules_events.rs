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

//! iRules `when` declarations and rule loaders: the original event word, its source geometry and the event body.

use super::super::Lowerer;
use super::super::commands::resolved_top_level_priority;
use crate::ir::{CommandTokens, Procedure, Statement};
use crate::segmenter::SegmentedCommand;
use tcl_registry::events::{IrulesCommandPlacement, IrulesExecutionContext};
use tcl_registry::hooks::LoweringHookId;

impl Lowerer<'_> {
    pub(in super::super) fn try_lower_when_declaration(
        &mut self,
        seg: &SegmentedCommand,
        namespace: &str,
        canonical_command: &str,
    ) -> Option<Statement> {
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
        let arguments = tcl_registry::events::IrulesDeclarationArguments::new(
            &arg_refs,
            seg.arg_tokens(),
            seg.arg_single_token(),
            &closed,
        )?;
        let tcl_registry::events::IrulesTopLevelDeclaration::Event {
            event,
            body_index,
            priority,
        } = self
            .registry
            .irules_top_level_declaration_shape(canonical_command, arguments)?
        else {
            return None;
        };
        Some(self.lower_when(seg, namespace, &event, body_index, priority))
    }

    /// Retain vendor rule declarations independently of native compiler protocol.
    /// The host declaration grammar supplies a deferred inventory, not an opcode
    /// or proof that runtime command installation necessarily completes.
    pub(in super::super) fn try_lower_rule_loader_declaration(
        &mut self,
        seg: &SegmentedCommand,
        namespace: &str,
        tokens: &CommandTokens,
    ) -> Option<Box<Statement>> {
        use tcl_dialect::model::InvocationRealm;
        use tcl_registry::events::IrulesTopLevelDeclaration;
        if !self
            .registry
            .profile()
            .is_some_and(tcl_dialect::DialectProfile::is_irules)
            || self.irules_execution_context != IrulesExecutionContext::TopLevel
        {
            return None;
        }
        let binding = tokens.source_binding.as_ref()?;
        if binding.invocation_realm() != Some(InvocationRealm::RuleLoader) {
            return None;
        }
        match self.retained_rule_loader_declaration(seg.span.start()) {
            Some(IrulesTopLevelDeclaration::Priority { value }) => {
                self.irules_priority = value;
                return Some(self.lower_default_boxed(seg, namespace));
            }
            Some(IrulesTopLevelDeclaration::Event {
                event,
                body_index,
                priority,
            }) => {
                return Some(Box::new(
                    self.lower_when(seg, namespace, &event, body_index, priority),
                ));
            }
            Some(IrulesTopLevelDeclaration::Procedure { .. }) => {
                // A validated alternative retains a deferred declaration only.
                // The original Call and its runtime/compilation residual remain.
                return Some(Box::new(self.lower_proc(seg, namespace)));
            }
            _ => {}
        }
        let target = binding.proved_handler_target()?;
        if !target.registry_backed || !target.prepended.is_empty() {
            return None;
        }
        let name = target.command.as_str();
        if let Some(value) =
            resolved_top_level_priority(self.registry, name, &binding.evaluated_argument_values)
        {
            self.irules_priority = value;
            return Some(self.lower_default_boxed(seg, namespace));
        }
        match self.registry.get(name)?.lowering_hook? {
            LoweringHookId::When => self
                .try_lower_when_declaration(seg, namespace, name)
                .map(Box::new),
            LoweringHookId::Proc => self
                .try_lower_proc_declaration(seg, namespace, name)
                .map(Box::new),
            _ => None,
        }
    }

    /// Body-bearing alternatives supply deferred inventory; file policy requires a
    /// closed declaration candidate. Neither projection licenses an opcode.
    fn retained_rule_loader_declaration(
        &self,
        offset: u32,
    ) -> Option<tcl_registry::events::IrulesTopLevelDeclaration> {
        use tcl_registry::events::IrulesTopLevelDeclaration;
        self.source_bindings
            .as_ref()?
            .deferred_rule_declaration_candidates(offset)?
            .iter()
            .find_map(|candidate| match &candidate.declaration {
                declaration @ IrulesTopLevelDeclaration::Event { .. } => Some(declaration.clone()),
                declaration @ IrulesTopLevelDeclaration::Procedure { .. }
                    if candidate.body.is_some() =>
                {
                    Some(declaration.clone())
                }
                declaration @ IrulesTopLevelDeclaration::Priority { .. } if !candidate.unknown => {
                    Some(declaration.clone())
                }
                _ => None,
            })
    }

    // Preserve each queried Option; this reports source ownership only.
    fn trace_report_event_option<T>(value: Option<T>, stage: &str) -> Option<T> {
        if tcl_dialect::trace_env::var_os("TCL_LSP_TRACE_EVENT_OPERAND").is_some() {
            eprintln!(
                "REPORT_EVENT_SOURCE stage={stage} available={}",
                value.is_some()
            );
        }
        value
    }

    fn trace_report_event_refusal(value: bool, stage: &str) -> bool {
        if tcl_dialect::trace_env::var_os("TCL_LSP_TRACE_EVENT_OPERAND").is_some() {
            eprintln!("REPORT_EVENT_SOURCE stage={stage} refused={value}");
        }
        value
    }

    fn original_hosted_when_event_word(
        &self,
        context: crate::registry_invocation::InvocationMetadataContext<'_>,
        bindings: &crate::command_binding::SourceCommandBindings,
        candidates: &[crate::command_binding::SourceRuleDeclarationCandidate],
        source: (
            &SegmentedCommand,
            &tcl_lexer::SourceImage,
            &CommandTokens,
            &[tcl_lexer::NativeWord],
        ),
        word: &tcl_lexer::NativeWord,
        event: &str,
    ) -> Option<tcl_lexer::NativeWord> {
        let (seg, image, tokens, words) = source;
        let input =
            Self::trace_report_event_option(context.source_analysis_input(), "hosted-input")?;
        let owner =
            Self::trace_report_event_option(bindings.original_analysis_owner(), "analysis-owner")?;
        let root = Self::trace_report_event_option(bindings.source_origin(), "root-origin")?;
        let binding =
            Self::trace_report_event_option(tokens.source_binding.as_ref(), "token-binding")?;
        let site = Self::trace_report_event_option(binding.invocation_site(), "invocation-site")?;
        if Self::trace_report_event_refusal(tokens.synthetic.is_some(), "synthetic")
            || Self::trace_report_event_refusal(
                !tokens.original_words_match_argv_text(),
                "original-argv",
            )
            || Self::trace_report_event_refusal(
                words.iter().any(|word| word.group().expand),
                "expanded",
            )
            || Self::trace_report_event_refusal(site.offset != seg.span.start(), "site-offset")
            || Self::trace_report_event_refusal(
                !std::sync::Arc::ptr_eq(&site.source, root),
                "site-root-owner",
            )
            || Self::trace_report_event_refusal(
                !owner.matches(bindings, input),
                "analysis-owner-current",
            )
            || Self::trace_report_event_refusal(
                bindings.source_origin()?.source_image() != image,
                "root-image",
            )
            || Self::trace_report_event_refusal(input.lexer_config() != self.config, "input-config")
            || Self::trace_report_event_refusal(
                !context.matches_registry(self.registry),
                "registry-current",
            )
        {
            return None;
        }
        // The retained declaration owns source geometry, not a current
        // dispatch cell or installation of this event handler.
        let policy = Self::trace_report_event_option(input.vendor_source_policy().or_else(|| {
            let hosted = tcl_dialect::model::bigip_execution_context::BigIpExecutionContext::for_environment(
                &input.borrowed_context_registry().context().environment.id,
            )?;
            tcl_syntax::naming::VendorSourceNamePolicy::authored(hosted)
        }), "hosted-policy")?;
        let head = crate::signature_scan::vendor_name::VendorSourceNameInput::from_original_word(
            words.first()?,
            policy,
        );
        let written = std::str::from_utf8(Self::trace_report_event_option(
            head.literal_units(tcl_syntax::naming::VendorSourceNamePurpose::CommandHead),
            "original-head-units",
        )?)
        .ok()?;
        let command = tcl_syntax::naming::unroot_rooted_key(written).unwrap_or(written);
        if candidates.iter().any(|candidate| {
            tcl_syntax::naming::unroot_rooted_key(&candidate.target.command)
                .unwrap_or(&candidate.target.command)
                != command
        }) {
            return None;
        }
        let input = crate::signature_scan::vendor_name::VendorSourceNameInput::from_original_word(
            word, policy,
        );
        let literal = Self::trace_report_event_option(
            input.literal_units(tcl_syntax::naming::VendorSourceNamePurpose::EventName),
            "hosted-event-units",
        )?;
        Self::trace_report_event_option(
            literal
                .eq_ignore_ascii_case(event.as_bytes())
                .then(|| word.clone()),
            "hosted-event-value",
        )
    }

    /// Event geometry comes from the retained declaration and original words.
    /// A captured/derived argument cannot borrow a displayed event spelling.
    fn original_when_event_word(
        &self,
        seg: &SegmentedCommand,
        event: &str,
        image: &tcl_lexer::SourceImage,
    ) -> Option<tcl_lexer::NativeWord> {
        use tcl_registry::events::IrulesTopLevelDeclaration;
        let bindings =
            Self::trace_report_event_option(self.source_bindings.as_ref(), "source-bindings")?;
        let candidates = Self::trace_report_event_option(
            bindings.deferred_rule_declaration_candidates(seg.span.start()),
            "deferred-candidates",
        )?;
        let indices = Self::trace_report_event_option(
            candidates
                .iter()
                .map(|candidate| {
                    let IrulesTopLevelDeclaration::Event { event: name, .. } =
                        &candidate.declaration
                    else {
                        return None;
                    };
                    if name != event
                        || !candidate.target.registry_backed
                        || !candidate.target.prepended.is_empty()
                    {
                        return None;
                    }
                    let spec = Self::trace_report_event_option(
                        self.registry.get(&candidate.target.command),
                        "candidate-registry-spec",
                    )?;
                    let symbol =
                        Self::trace_report_event_option(spec.defines_symbol, "candidate-symbol")?;
                    (symbol.kind == tcl_registry::DefinedSymbolKind::Event)
                        .then_some(usize::from(symbol.name_arg) + 1)
                })
                .collect::<Option<Vec<_>>>(),
            "event-indices",
        )?;
        let &index = Self::trace_report_event_option(indices.first(), "event-index")?;
        if indices.iter().any(|candidate| *candidate != index) {
            return None;
        }
        let tokens = self.cmd_tokens(seg);
        let words = Self::trace_report_event_option(
            crate::registry_invocation::original_native_compiler_words(
                image,
                &tokens.word_exprs,
                seg.span.start(),
                self.config,
            ),
            "original-words",
        )?;
        let word = Self::trace_report_event_option(words.get(index), "original-event-word")?;
        if let Some(context) =
            Self::trace_report_event_option(self.invocation_metadata_context(), "metadata-context")
            && context
                .source_analysis_input()
                .is_some_and(crate::analyser::ResolvedAnalysisInput::has_hosted_source_name_context)
        {
            return self.original_hosted_when_event_word(
                context,
                bindings,
                candidates,
                (seg, image, &tokens, &words),
                word,
                event,
            );
        }
        let dialect =
            Self::trace_report_event_option(self.invocation_dialect, "standalone-dialect")?;
        let key = crate::signature_scan::scope::SignatureSourceNameKey::from_original_native_word(
            word,
            dialect.word_values,
            dialect.authored_name_policy()?,
        )?;
        key.bytes()
            .eq_ignore_ascii_case(event.as_bytes())
            .then(|| word.clone())
    }

    fn original_when_source_geometry(
        &self,
        seg: &SegmentedCommand,
        event: &str,
    ) -> (tcl_lexer::SourceImage, Option<tcl_lexer::NativeWord>) {
        let image = self
            .source_bindings
            .as_ref()
            .map(|bindings| bindings.invocation_at_source(seg.name(), seg.span.start()))
            .and_then(|binding| {
                binding
                    .invocation_site()
                    .map(|site| site.source.source_image().clone())
            })
            .unwrap_or_else(|| {
                tcl_lexer::SourceImage::from_bytes(self.source.as_bytes(), self.source_channel)
            });
        let word = self.original_when_event_word(seg, event, &image);
        (image, word)
    }

    /// Lower a registry-validated iRules event declaration.
    fn lower_when(
        &mut self,
        seg: &SegmentedCommand,
        namespace: &str,
        event_name: &str,
        body_idx: usize,
        priority: Option<u16>,
    ) -> Statement {
        if self.compilation_scope == tcl_runtime_api::SourceCompilationScope::EnteredSource {
            return self.lower_default(seg, namespace);
        }
        let args = seg.args();
        let body_tok = seg.arg_tokens()[body_idx];
        let retained = self
            .source_bindings
            .as_ref()
            .and_then(|bindings| bindings.deferred_rule_declaration_candidates(seg.span.start()))
            .and_then(|candidates| {
                candidates
                    .iter()
                    .find_map(|candidate| candidate.body.clone())
            });
        let body = self.in_procedure_frame(Some(IrulesExecutionContext::EventBody), |lowerer| {
            if let Some(source) = retained {
                lowerer.lower_retained_script(source)
            } else {
                lowerer.lower_body_from_tok(&args[body_idx], Some(&body_tok), namespace)
            }
        });
        let body_offset = body_tok.span.start() + u32::from(body_tok.content_offset);

        let base_priority = u32::from(priority.unwrap_or(self.irules_priority));

        let n = self.when_counts.get(event_name).copied().unwrap_or(0);
        *self.when_counts.entry(event_name.to_owned()).or_insert(0) += 1;
        let qualified = if n == 0 {
            format!("::when::{event_name}")
        } else {
            format!("::when::{event_name}#{n}")
        };

        let (image, event_word) = self.original_when_source_geometry(seg, event_name);
        let event_body = std::sync::Arc::new(crate::ir::SourceIrulesEventBody::from_lowering(
            event_name,
            seg.span,
            &body,
            image,
            self.config,
            self.registry,
            event_word,
        ));
        let id = self.module.retain_procedure(Procedure {
            name: event_name.to_owned(),
            qualified_name: qualified,
            publication: None,
            declaration_site: self
                .cmd_tokens(seg)
                .source_binding
                .as_ref()
                .and_then(|binding| binding.invocation_site())
                .cloned(),
            body_kind: crate::ir::ProcedureBodyKind::Event,
            params: vec![],
            span: seg.span,
            body,
            params_raw: String::new(),
            body_source: None,
            body_offset,
            namespace_scoped: false,
            base_priority,
        });
        self.module.irules_event_bodies.insert(id, event_body);

        Statement::Call {
            span: seg.span,
            command: "when".into(),
            canonical_command: None,
            args: args.to_vec(),
            defs: vec![],
            reads: vec![],
            reads_own_defs: false,
            safe_on_uninit: false,
            tokens: Some(self.cmd_tokens(seg)),
            foreach_groups: None,
        }
    }
}
