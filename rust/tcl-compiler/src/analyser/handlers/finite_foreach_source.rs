// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Conditional names from one exact literal foreach and its sole declaration.
//!
//! These receipts describe source declarations only. They supply no fetched
//! variable value, successful iteration, current command target or Native entry.

#[cfg(test)]
use super::SegmentedCommand;
use super::{Analyser, Span, Token, TokenType};
use crate::command_binding::OriginalSourceAnalysisOwner;
use crate::signature_scan::scope::{
    SignatureNamespaceScope, SignatureSourceCommand, SignatureSourceNameValue,
};
use std::sync::Arc;

// The lazy cell is per Analyser, never thread-local and never a report/cache field.
// Guards own their state so the full ordinary walk can keep borrowing &mut self.
#[derive(Default)]
pub(in crate::analyser) struct FiniteForeachReportingState {
    frames: std::sync::OnceLock<Arc<std::sync::Mutex<Vec<ActiveIteration>>>>,
}

impl std::fmt::Debug for FiniteForeachReportingState {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("FiniteForeachReportingState")
            .finish_non_exhaustive()
    }
}

struct ActiveIteration {
    plan: Arc<FiniteForeachProcedures>,
    index: usize,
    disabled: bool,
    first_header_seen: bool,
}

pub(super) enum FiniteNameLookup {
    Ordinary,
    Candidate(Arc<FiniteForeachProcedureName>),
    Refused,
}

#[derive(Clone, Copy)]
enum RestoreScope {
    Frame,
    Iteration { index: usize, disabled: bool },
}

pub(super) struct ReportingGuard {
    frames: Arc<std::sync::Mutex<Vec<ActiveIteration>>>,
    plan: Arc<FiniteForeachProcedures>,
    restore: RestoreScope,
}

impl Drop for ReportingGuard {
    fn drop(&mut self) {
        let Ok(mut frames) = self.frames.lock() else {
            return;
        };
        let Some(active) = frames.last_mut() else {
            return;
        };
        if !Arc::ptr_eq(&active.plan, &self.plan) {
            let retired = std::mem::take(&mut *frames);
            drop(frames);
            drop(retired);
            return;
        }
        match self.restore {
            RestoreScope::Frame => {
                let retired = frames.pop();
                drop(frames);
                drop(retired);
            }
            RestoreScope::Iteration { index, disabled } => {
                active.index = index;
                active.disabled = disabled;
            }
        }
    }
}

impl FiniteForeachReportingState {
    fn enter(&self, plan: FiniteForeachProcedures) -> Option<ReportingGuard> {
        let frames = Arc::clone(
            self.frames
                .get_or_init(|| Arc::new(std::sync::Mutex::new(Vec::new()))),
        );
        let plan = Arc::new(plan);
        frames.lock().ok()?.push(ActiveIteration {
            plan: Arc::clone(&plan),
            index: 0,
            disabled: false,
            first_header_seen: false,
        });
        Some(ReportingGuard {
            frames,
            plan,
            restore: RestoreScope::Frame,
        })
    }

    pub(super) fn lookup(
        &self,
        head: Option<Token>,
        name: Token,
        scope: &[usize],
        body_depth: u32,
    ) -> FiniteNameLookup {
        let Some(head) = head else {
            return FiniteNameLookup::Ordinary;
        };
        let Some(frames) = self.frames.get() else {
            return FiniteNameLookup::Ordinary;
        };
        let Ok(mut frames) = frames.try_lock() else {
            return FiniteNameLookup::Refused;
        };
        let Some(active) = frames.last_mut() else {
            return FiniteNameLookup::Ordinary;
        };
        let Some(first) = active.plan.names.first() else {
            return FiniteNameLookup::Refused;
        };
        let expected_depth = if active.index == 0 {
            active.plan.entry_depth.checked_add(1)
        } else {
            Some(active.plan.entry_depth)
        };
        if active.disabled
            || expected_depth != Some(body_depth)
            || first.command != head
            || first.name_token != name
            || first.scope != scope
        {
            return FiniteNameLookup::Ordinary;
        }
        let Some(candidate) = active.plan.names.get(active.index) else {
            return FiniteNameLookup::Refused;
        };
        if active.index == 0 {
            active.first_header_seen = true;
        }
        FiniteNameLookup::Candidate(Arc::clone(candidate))
    }

    pub(super) fn advance(
        &self,
        variable: &str,
        body: Token,
        scope: &[usize],
        index: usize,
        element: &str,
    ) -> Option<ReportingGuard> {
        let frames = self.frames.get()?;
        let mut locked = frames.try_lock().ok()?;
        let active = locked.last_mut()?;
        let first = active.plan.names.first()?;
        if active.plan.variable != variable
            || active.plan.body_span != body.span
            || first.scope != scope
        {
            return None;
        }
        let restore = RestoreScope::Iteration {
            index: active.index,
            disabled: active.disabled,
        };
        active.disabled = !active.first_header_seen
            || active
                .plan
                .names
                .get(index)
                .is_none_or(|name| name.value.bytes() != element.as_bytes());
        active.index = index;
        Some(ReportingGuard {
            frames: Arc::clone(frames),
            plan: Arc::clone(&active.plan),
            restore,
        })
    }
}

pub(super) struct FiniteForeachProcedureName {
    command: Token,
    name_token: Token,
    variable: String,
    declaration_span: Span,
    value: SignatureSourceNameValue,
    image: tcl_lexer::SourceImage,
    input: crate::analyser::ResolvedAnalysisInput,
    owner: Arc<OriginalSourceAnalysisOwner>,
    namespace: SignatureNamespaceScope,
    scope: Vec<usize>,
}

impl FiniteForeachProcedureName {
    pub(super) fn reborrow(
        &self,
        analyser: &Analyser,
        name_token: Token,
        scope: &[usize],
    ) -> Option<(SignatureSourceCommand, String)> {
        let realm = analyser.result.retained_command_realm()?;
        let current_scope = super::super::scope::scope_at(&analyser.result.global_scope, scope)?;
        if current_scope.original_interpreter_source_domain.is_some() {
            return None;
        }
        let declaration = current_scope.variables.get(&self.variable)?;
        if !FiniteForeachProcedures::supports_input(analyser, &self.input)
            || analyser.result.original_source_region.is_some()
            || realm.source_bindings_ref().has_conditional_source_region()
            || declaration.definition_span != self.declaration_span
            || declaration.link_target.is_some()
            || !declaration.array_indices.is_empty()
            || self.name_token != name_token
            || self.scope != scope
            || analyser.result.resolved_input.as_ref() != Some(&self.input)
            || !Arc::ptr_eq(
                &self.owner,
                realm.source_bindings_ref().original_analysis_owner()?,
            )
            || !analyser
                .result
                .matches_original_source_image(&self.image, self.input.lexer_config())
            || analyser.declaration_namespace_scope(scope).as_ref() != Some(&self.namespace)
            || analyser.declaration_name_policy() != Some(self.value.policy())
            || self.image.bytes() != analyser.source.as_bytes()
        {
            return None;
        }
        let text = std::str::from_utf8(self.value.bytes()).ok()?.to_owned();
        let name = SignatureSourceCommand::procedure_in_context(
            self.value.policy(),
            &self.namespace,
            &text,
        )?;
        Some((name, text))
    }
}

// Observe only existing results; the original flag/filter/1024 quota remain shared.
macro_rules! finite_some {
    ($stage:literal, $value:expr) => {
        crate::registry_invocation::source_structure::trace_jim_source_option(
            concat!("analyser.finite.", $stage),
            $value,
        )?
    };
}
macro_rules! finite_none {
    ($stage:literal) => {
        crate::registry_invocation::source_structure::trace_jim_source_option(
            concat!("analyser.finite.", $stage),
            None,
        )
    };
}

struct FiniteForeachProcedures {
    names: Vec<Arc<FiniteForeachProcedureName>>,
    variable: String,
    body_span: Span,
    entry_depth: u32,
}

impl FiniteForeachProcedures {
    fn supports_input(analyser: &Analyser, input: &crate::analyser::ResolvedAnalysisInput) -> bool {
        !input.has_logical_source_name_context()
            && !input.has_hosted_source_name_context()
            && analyser
                .source_analysis_entry
                .as_ref()
                .is_none_or(|entry| entry.source_point_entry.is_none())
    }

    fn selected_call(
        analyser: &Analyser,
        head: Token,
        args: &[String],
        tokens: &[Token],
        policy: tcl_syntax::naming::NamePolicyProtocol,
    ) -> Option<crate::analyser::diagnostic_registry::OriginalDiagnosticInvocation> {
        use crate::registry_invocation::InvocationWordOrigin;
        use tcl_registry::hooks::AnalyserHookId;
        let call = analyser.original_diagnostic_call_at(head, tokens)?;
        let (arguments, actual_tokens, _) = call.source_arguments()?;
        if !call.matches_analysis(&analyser.result, &analyser.source)
            || !call.supplemental_literals().is_empty()
            || call.words().origins().len() != 4
            || !matches!(
                call.words().origins().first(),
                Some(InvocationWordOrigin::Written(0) | InvocationWordOrigin::ResolvedHead)
            )
            || call.words().origins()[1..]
                != [
                    InvocationWordOrigin::Written(1),
                    InvocationWordOrigin::Written(2),
                    InvocationWordOrigin::Written(3),
                ]
            || call.head().tokens() != [head]
            || SignatureSourceNameValue::from_original_static_word(
                call.head(),
                analyser.word_rules(),
                policy,
            )?
            .bytes()
                != call.command().as_bytes()
            || arguments != args
            || actual_tokens != tokens
        {
            return None;
        }
        let roles = call.with_schema(|schema| {
            (schema.semantics.analyser_hook == Some(AnalyserHookId::Foreach))
                .then(|| schema.arg_roles())
                .flatten()
        })??;
        if roles
            .iter()
            .filter(|(_, role)| *role == tcl_registry::ArgRole::LoopVarList)
            .map(|(index, _)| *index)
            .collect::<Vec<_>>()
            != [0]
            || roles
                .iter()
                .filter(|(_, role)| *role == tcl_registry::ArgRole::Body)
                .map(|(index, _)| *index)
                .collect::<Vec<_>>()
                != [2]
        {
            return None;
        }
        Some(call)
    }

    fn capture_loop_header(
        analyser: &Analyser,
        head: Token,
        args: &[String],
        tokens: &[Token],
        scope: &[usize],
        policy: tcl_syntax::naming::NamePolicyProtocol,
        input: &crate::analyser::ResolvedAnalysisInput,
    ) -> Option<(
        crate::analyser::diagnostic_registry::OriginalDiagnosticInvocation,
        String,
        Span,
    )> {
        if !Self::supports_input(analyser, input)
            || analyser.result.original_source_region.is_some()
            || policy.recipe() != tcl_syntax::naming::NativeNameProtocol::Jim084
            || args.len() != 3
            || tokens.len() != 3
            || tokens[1].kind != TokenType::Str
            || tokens[2].kind != TokenType::Str
        {
            return finite_none!("eligibility");
        }
        let call = finite_some!(
            "selected-call",
            Self::selected_call(analyser, head, args, tokens, policy)
        );
        let (variable, _) = finite_some!(
            "literal-binding",
            Analyser::literal_foreach_binding(&args[0], &args[1], true, analyser.word_rules())
        );
        if variable.is_empty()
            || !variable
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b == b'_')
        {
            return finite_none!("variable-shape");
        }
        let current_scope = finite_some!(
            "scope",
            super::super::scope::scope_at(&analyser.result.global_scope, scope)
        );
        if current_scope.original_interpreter_source_domain.is_some() {
            return finite_none!("source-domain");
        }
        let declaration = finite_some!("variable", current_scope.variables.get(&variable));
        if declaration.link_target.is_some() || !declaration.array_indices.is_empty() {
            return finite_none!("variable-bindings");
        }
        Some((call, variable, declaration.definition_span))
    }

    fn capture(
        analyser: &Analyser,
        head: Token,
        args: &[String],
        tokens: &[Token],
        scope: &[usize],
    ) -> Option<Self> {
        let input = finite_some!("input", analyser.result.resolved_input.as_ref());
        let policy = finite_some!("policy", analyser.declaration_name_policy());
        let (call, variable, declaration_span) =
            Self::capture_loop_header(analyser, head, args, tokens, scope, policy, input)?;
        let parent = finite_some!("list-word", call.word(1));
        let list = finite_some!(
            "list-value",
            SignatureSourceNameValue::from_original_static_word(
                parent,
                analyser.word_rules(),
                policy,
            )
        );
        let values = finite_some!("list-elements", list.list_elements());
        if values.is_empty() {
            return finite_none!("empty-elements");
        }
        let commands = finite_some!(
            "body-statements",
            analyser.direct_statements_in_span(tokens[2].span)
        );
        let [declaration] = commands.as_slice() else {
            return finite_none!("body-count");
        };
        let positions = finite_some!(
            "procedure-words",
            analyser.original_procedure_source_words_at(
                declaration.argv[0],
                declaration.args(),
                declaration.arg_tokens(),
            )
        );
        let name_token = *finite_some!("name-token", declaration.arg_tokens().get(positions.name));
        if name_token.kind != TokenType::Var
            || crate::value_shapes::whole_word_scalar_var_name(
                finite_some!("name-text", declaration.args().get(positions.name)),
                analyser.file_lexer_config(),
            ) != Some(variable.as_str())
            || !declaration
                .arg_single_token()
                .get(positions.name)
                .copied()
                .unwrap_or(false)
        {
            return finite_none!("name-shape");
        }
        let realm = finite_some!("realm", analyser.result.retained_command_realm());
        if realm.source_bindings_ref().has_conditional_source_region() {
            return finite_none!("region");
        }
        let owner = Arc::clone(finite_some!(
            "owner",
            realm.source_bindings_ref().original_analysis_owner()
        ));
        let namespace = finite_some!("namespace", analyser.declaration_namespace_scope(scope));
        for value in &values {
            let text = finite_some!("element-utf8", std::str::from_utf8(value.bytes()).ok());
            finite_some!(
                "element-name",
                SignatureSourceCommand::procedure_in_context(policy, &namespace, text)
            );
        }
        let image = tcl_lexer::SourceImage::document(&analyser.source);
        if realm.original_source_image() != Some(&image) {
            return None;
        }
        let names = values
            .into_iter()
            .map(|value| {
                Arc::new(FiniteForeachProcedureName {
                    command: declaration.argv[0],
                    name_token,
                    variable: variable.clone(),
                    declaration_span,
                    value,
                    image: image.clone(),
                    input: input.clone(),
                    owner: Arc::clone(&owner),
                    namespace: namespace.clone(),
                    scope: scope.to_vec(),
                })
            })
            .collect();
        Some(Self {
            names,
            variable,
            body_span: tokens[2].span,
            entry_depth: analyser.body_depth,
        })
    }
}

impl Analyser {
    pub(in crate::analyser) fn handle_foreach_command_at(
        &mut self,
        head: Token,
        command: &str,
        args: &[String],
        tokens: &[Token],
        scope: &[usize],
    ) -> bool {
        let plan = FiniteForeachProcedures::capture(self, head, args, tokens, scope);
        let _reporting_scope = plan.and_then(|plan| self.finite_foreach_names.enter(plan));
        self.handle_foreach_command(command, args, tokens, scope)
    }
}

#[cfg(test)]
mod controls {
    use super::*;

    fn input() -> crate::analyser::ResolvedAnalysisInput {
        tcl_spectcl::core_surfaces::ensure();
        let environment = tcl_registry::model::ingress::resolve_environment("jimtcl");
        let profile = environment.analyser_profile();
        let context = Arc::new(
            environment
                .default_context_registry()
                .with_command_store(Arc::new(tcl_registry::CommandRegistry::build_default())),
        );
        crate::analyser::ResolvedAnalysisInput::new(
            profile,
            profile,
            context,
            tcl_lexer::LexerConfig::for_profile(Some(profile)),
        )
    }

    fn fixture(
        source: &str,
        input: &crate::analyser::ResolvedAnalysisInput,
    ) -> (Analyser, SegmentedCommand) {
        let mut analyser = Analyser::new().with_resolved_input(input.clone());
        let _ = analyser.analyse_and_retain_result_for_test(source, "jimtcl");
        let command = crate::segmenter::segment_commands_with_offset_and_config(
            source,
            0,
            analyser.file_lexer_config(),
        )
        .remove(0);
        (analyser, command)
    }

    fn capture(analyser: &Analyser, command: &SegmentedCommand) -> Option<FiniteForeachProcedures> {
        FiniteForeachProcedures::capture(
            analyser,
            command.argv[0],
            command.args(),
            command.arg_tokens(),
            &[],
        )
    }

    #[test]
    fn literal_jim_iterations_keep_readonly_list_lineage_and_source_names() {
        let source = "foreach name {a b} {proc $name {} {{n 0}} {return $n}}";
        let (analyser, command) = fixture(source, &input());
        assert!(!analyser.result.allows_lexical_declaration_advice());
        let plan = capture(&analyser, &command).expect("actual sole declaration iteration plan");
        assert_eq!(plan.names.len(), 2);
        let header_read = plan.names[0].name_token.span;
        assert!(
            analyser
                .result
                .global_scope
                .variables
                .get("name")
                .unwrap()
                .references
                .contains(&header_read)
        );
        assert!(matches!(
            analyser.finite_foreach_names.lookup(
                Some(plan.names[0].command),
                plan.names[0].name_token,
                &[],
                analyser.body_depth,
            ),
            FiniteNameLookup::Ordinary
        ));
        for (receipt, expected) in plan.names.iter().zip(["a", "b"]) {
            assert_eq!(receipt.value.bytes(), expected.as_bytes());
            assert!(receipt.value.original_static_list_container().is_some());
            let (name, text) = receipt
                .reborrow(&analyser, receipt.name_token, &[])
                .unwrap();
            assert_eq!(text, expected);
            assert_eq!(name.simple_name().as_deref(), Some(expected));
            assert!(
                analyser
                    .result
                    .all_procs
                    .values()
                    .any(|procedure| procedure.name == expected)
            );
        }
        assert!(
            analyser
                .source_analysis_entry
                .as_ref()
                .is_none_or(|entry| entry.native_entry.is_none())
        );
    }

    #[test]
    fn iteration_names_refuse_equal_foreign_owner_changed_source_and_namespace() {
        let source = "foreach name {a b} {proc $name {} {return VALUE}}";
        let input = input();
        let (mut analyser, command) = fixture(source, &input);
        let mut plan = capture(&analyser, &command).unwrap();
        let receipt = plan.names.remove(0);
        assert!(
            receipt
                .reborrow(&analyser, receipt.name_token, &[])
                .is_some()
        );
        analyser
            .result
            .global_scope
            .variables
            .get_mut("name")
            .unwrap()
            .link_target = Some("::foreign".to_owned());
        assert!(
            receipt
                .reborrow(&analyser, receipt.name_token, &[])
                .is_none()
        );
        assert!(capture(&analyser, &command).is_none());
        analyser
            .result
            .global_scope
            .variables
            .get_mut("name")
            .unwrap()
            .link_target = None;
        assert!(
            receipt
                .reborrow(&analyser, receipt.name_token, &[])
                .is_some()
        );
        let (foreign, _) = fixture(source, &input);
        assert!(
            receipt
                .reborrow(&foreign, receipt.name_token, &[])
                .is_none()
        );
        assert!(
            receipt
                .reborrow(&analyser, receipt.name_token, &[0])
                .is_none()
        );
        let original_config = analyser.result.body_lexer_config;
        let mut changed = original_config.unwrap();
        changed.strict_quoting = !changed.strict_quoting;
        analyser.result.body_lexer_config = Some(changed);
        assert!(
            receipt
                .reborrow(&analyser, receipt.name_token, &[])
                .is_none()
        );
        analyser.result.body_lexer_config = original_config;
        assert!(
            receipt
                .reborrow(&analyser, receipt.name_token, &[])
                .is_some()
        );
        analyser.source.push_str("\n# changed complete source");
        assert!(
            receipt
                .reborrow(&analyser, receipt.name_token, &[])
                .is_none()
        );
    }

    #[test]
    fn iteration_plan_refuses_mutation_multiple_bindings_expansion_and_name_prefixes() {
        let input = input();
        for source in [
            "foreach name {a b} {set name changed; proc $name {} {return VALUE}}",
            "foreach {name other} {a b} {proc $name {} {return VALUE}}",
            "foreach name {*}$values {proc $name {} {return VALUE}}",
            "foreach name {a b} {proc prefix$name {} {return VALUE}}",
            "foreach name {a b} {proc {*}$name {} {return VALUE}}",
            "foreach name {a b} {proc $name {} {return VALUE}; proc fixed {} {}}",
        ] {
            let (analyser, command) = fixture(source, &input);
            assert!(
                capture(&analyser, &command).is_none(),
                "source refused: {source}"
            );
        }
    }

    #[test]
    fn reporting_scope_restores_after_unwind_and_preserves_body_depth_refusal() {
        let source = "foreach name {a b} {proc $name {} {return VALUE}}";
        let (mut analyser, command) = fixture(source, &input());
        let plan = capture(&analyser, &command).unwrap();
        let head = plan.names[0].command;
        let name = plan.names[0].name_token;
        let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let _scope = analyser.finite_foreach_names.enter(plan).unwrap();
            assert!(matches!(
                analyser.finite_foreach_names.lookup(
                    Some(head),
                    name,
                    &[],
                    analyser.body_depth + 1
                ),
                FiniteNameLookup::Candidate(_)
            ));
            panic!("test-only source reporting scope unwind");
        }));
        assert!(outcome.is_err());
        assert!(matches!(
            analyser
                .finite_foreach_names
                .lookup(Some(head), name, &[], analyser.body_depth + 1),
            FiniteNameLookup::Ordinary
        ));

        let plan = capture(&analyser, &command).unwrap();
        let _scope = analyser.finite_foreach_names.enter(plan).unwrap();
        let previous_depth = analyser.body_depth;
        analyser.body_depth = crate::depth_guard::MAX_SOURCE_NEST_DEPTH.0;
        let held_depth = analyser.body_depth;
        analyser.analyse_body(&command.args()[2], command.arg_tokens()[2], &[]);
        assert_eq!(analyser.body_depth, held_depth);
        assert!(
            analyser
                .result
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic.code == tcl_core_types::DiagCode::E207)
        );
        assert!(
            !analyser
                .finite_foreach_names
                .frames
                .get()
                .unwrap()
                .lock()
                .unwrap()
                .last()
                .unwrap()
                .first_header_seen
        );
        // Restore the real replay depth so refusal cannot be hidden by a mismatch.
        analyser.body_depth = previous_depth;
        // A refused ordinary body walk cannot issue the later iteration name.
        let _iteration = analyser
            .finite_foreach_names
            .advance("name", command.arg_tokens()[2], &[], 1, "b")
            .unwrap();
        assert!(matches!(
            analyser
                .finite_foreach_names
                .lookup(Some(head), name, &[], analyser.body_depth),
            FiniteNameLookup::Ordinary
        ));
    }
}
