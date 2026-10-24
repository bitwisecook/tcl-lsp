// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Conditional original procedure bodies share the source invocation walker.

use super::{
    AdviceCell, AdviceGraph, AdviceInvocation, AdviceInvocationContext, AdviceNamingPolicy,
    AdvicePublicationPurpose, OriginalSourceCommandTransition,
    OriginalSourceCommandTransitionAdvice, OriginalSourceTransitionAdviceTape,
    SourceAdviceNameInput, SourceAdviceWord,
};
use crate::registry_invocation::OriginalSourceScriptBody;
use std::sync::{Arc, atomic::Ordering};
use tcl_core_types::ByteCommandSlot;
use tcl_registry::{ArgRole, CommandBindingDefinitionKind, CommandBindingTransition, Traits};

const MAX_DEPTH: usize = 64;
const MAX_COMMANDS: usize = 16_384;

/// A conditional procedure source call at its actual original source horizon.
/// This is not a resolved runtime invocation, entered body or current slot.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OriginalSourceProcedureCall {
    original: Arc<[tcl_lexer::NativeWord]>,
    procedure: Arc<OriginalSourceProcedureBody>,
    prefix: Vec<SourceAdviceWord>,
    caller: Option<Arc<OriginalSourceScriptBody>>,
    uncertain_operations: Vec<Arc<[tcl_lexer::NativeWord]>>,
    future_body: bool,
    input: Option<crate::analyser::ResolvedAnalysisInput>,
    analysis_owner: Option<Arc<crate::command_binding::OriginalSourceAnalysisOwner>>,
    inherited_entry: Option<crate::command_binding::OriginalSourcePointEntry>,
}
impl OriginalSourceProcedureCall {
    /// Separate diagnostic-report purpose: replace only the actual analysis
    /// owner pair after the sealed complete current output attests both sides.
    /// Runtime Eq and source-call applicability are unchanged.
    pub(crate) fn authenticated_report_eq(
        &self,
        other: &Self,
        output: &crate::analyser::authenticated_report::ReportOutputCorrespondence<'_>,
    ) -> Option<bool> {
        let (left, right) = output.analyses();
        let Self {
            original: _,
            procedure: _,
            prefix: _,
            caller: _,
            uncertain_operations: _,
            future_body: _,
            input: _,
            analysis_owner: _,
            inherited_entry: _,
        } = self;
        if !self.matches_analysis(left)
            || !other.matches_analysis(right)
            || !output.analysis_owners(
                self.analysis_owner.as_ref()?,
                other.analysis_owner.as_ref()?,
            )
            || self.input != other.input
            || self.inherited_entry != other.inherited_entry
        {
            return None;
        }
        // These retained typed structures have no replacement projection here.
        // A distinct nested issuer cannot be relabelled as semantic Difference.
        if self.procedure != other.procedure || self.caller != other.caller {
            return None;
        }
        Some(
            self.original == other.original
                && self.prefix == other.prefix
                && self.uncertain_operations == other.uncertain_operations
                && self.future_body == other.future_body,
        )
    }
    /// Actual complete written vector; reporting invocation labels do not issue it.
    #[must_use]
    pub fn original_words(&self) -> &[tcl_lexer::NativeWord] {
        &self.original
    }
    /// Original alias-prefix argv slots retained by the same source cell walk.
    /// These have no written operand at this call and grant no runtime binding.
    #[must_use]
    pub fn captured_argument_count(&self) -> usize {
        self.prefix.len()
    }
    /// Original declaring header joined against the retained analysis allocation.
    #[must_use]
    pub fn declaration<'a>(
        &self,
        analysis: &'a crate::analyser::AnalysisResult,
    ) -> Option<
        &'a crate::signature_scan::original_name::SourceDeclarationMetadata<
            crate::analyser::ProcDef,
        >,
    > {
        self.matches_analysis(analysis).then_some(())?;
        self.procedure.source_procedure(analysis)
    }
    /// Header in an actual retained source ancestor of this child's view.
    /// The host separately validates current physical parent publications.
    #[must_use]
    pub fn declaration_in_parent<'a>(
        &self,
        child: &crate::analyser::AnalysisResult,
        parent: &'a crate::analyser::AnalysisResult,
    ) -> Option<
        &'a crate::signature_scan::original_name::SourceDeclarationMetadata<
            crate::analyser::ProcDef,
        >,
    > {
        self.matches_analysis(child).then_some(())?;
        let input = child.resolved_input.as_ref()?;
        let realm = child.retained_command_realm()?;
        let bindings = realm.source_bindings_ref();
        let point = bindings.original_inherited_source_entry()?;
        (self.inherited_entry.as_ref() == Some(point)).then_some(())?;
        let origin = bindings.root_origin.as_ref()?;
        if !child.matches_original_source_image(origin.source_image(), input.lexer_config())
            || !matches!(origin.kind(), crate::command_binding::SourceOriginKind::InheritedPoint { parent, .. } if parent == &point.source_origin())
            || !point.matches_ancestor_analysis(parent)
        {
            return None;
        }
        self.procedure.source_procedure(parent)
    }

    /// Header in an actual dependency of this exact conditional region.
    /// Physical publication eligibility remains a separate host obligation.
    #[must_use]
    pub fn declaration_in_region_dependency<'a>(
        &self,
        caller: &crate::analyser::AnalysisResult,
        declaring: &'a crate::analyser::AnalysisResult,
    ) -> Option<
        &'a crate::signature_scan::original_name::SourceDeclarationMetadata<
            crate::analyser::ProcDef,
        >,
    > {
        self.matches_analysis(caller).then_some(())?;
        let region = caller.original_source_region()?;
        let point = region.bindings().original_inherited_source_entry()?;
        (self.inherited_entry.as_ref() == Some(point)).then_some(())?;
        point
            .conditional_region()?
            .owns_procedure_declaration(&self.procedure.declaration)
            .then_some(())?;
        region
            .matches_declaration_dependency(declaring)
            .then_some(())?;
        self.procedure.source_procedure(declaring)
    }

    fn matches_analysis(&self, analysis: &crate::analyser::AnalysisResult) -> bool {
        let (Some(input), Some(owner), Some(realm)) = (
            &self.input,
            &self.analysis_owner,
            analysis.retained_command_realm(),
        ) else {
            return false;
        };
        analysis.resolved_input.as_ref() == Some(input)
            && realm
                .source_bindings_ref()
                .original_analysis_owner()
                .is_some_and(|current| Arc::ptr_eq(current, owner))
            && owner.matches(realm.source_bindings_ref(), input)
    }
    pub(crate) fn with_input(
        mut self,
        input: &crate::analyser::ResolvedAnalysisInput,
        owner: Arc<crate::command_binding::OriginalSourceAnalysisOwner>,
        inherited_entry: Option<&crate::command_binding::OriginalSourcePointEntry>,
    ) -> Self {
        self.input = Some(input.clone());
        self.analysis_owner = Some(owner);
        self.inherited_entry = inherited_entry.cloned();
        self
    }
    /// Actual declaration premises remain conditional source obligations.
    #[must_use]
    pub fn declaration_advice(&self) -> &OriginalSourceCommandTransitionAdvice {
        &self.procedure.declaration
    }
    /// Held alias/move lineage at this original call horizon.
    #[must_use]
    pub fn lineage(&self) -> &[Arc<super::OriginalSourceCommandTransition>] {
        &self.procedure.lineage
    }
    /// Operand effects remain an independent pre-lookup applicability debt.
    #[must_use]
    pub fn has_lookup_operand_effects(&self) -> bool {
        !super::body_effects::original_operand_effects(&self.original).is_empty()
    }
    /// Authentic enclosing procedure declaration, independently of entry.
    #[must_use]
    pub fn caller_declaration<'a>(
        &self,
        analysis: &'a crate::analyser::AnalysisResult,
    ) -> Option<
        &'a crate::signature_scan::original_name::SourceDeclarationMetadata<
            crate::analyser::ProcDef,
        >,
    > {
        self.matches_analysis(analysis).then_some(())?;
        let body = self.caller.as_ref()?;
        let crate::registry_invocation::source_structure::OriginalRegistrySource::SourceTransitions(
            advice,
        ) = body.source_words().source()
        else {
            return None;
        };
        let mut records = analysis.original_procedure_declarations().filter(|record| {
            record.declaration_site() == advice.site()
                && advice
                    .original_words()
                    .contains(record.name_input().original_word())
                && record.metadata().source_name.as_ref() == Some(record.name())
        });
        let record = records.next()?;
        records.next().is_none().then_some(record)
    }
    /// Authentic enclosing procedure source body, without entry or frame proof.
    #[must_use]
    pub fn caller_body(&self) -> Option<&OriginalSourceScriptBody> {
        self.caller.as_deref()
    }
    /// Earlier unknown mutation vectors remain debt, never an absence proof.
    #[must_use]
    pub fn uncertain_operations(&self) -> &[Arc<[tcl_lexer::NativeWord]>] {
        &self.uncertain_operations
    }
    /// The body was surveyed conditionally rather than observed entered.
    #[must_use]
    pub const fn is_future_body(&self) -> bool {
        self.future_body
    }
    pub(crate) fn matches_source_context(
        &self,
        image: &tcl_lexer::SourceImage,
        config: tcl_lexer::LexerConfig,
        context: &tcl_registry::model::ContextRegistry,
    ) -> bool {
        ((self.procedure.declaration.matches_source(image, config)
            && self
                .analysis_owner
                .as_ref()
                .is_some_and(|owner| owner.source == self.procedure.declaration.site().source))
            || self.inherited_entry.as_ref().is_some_and(|point| {
                self.input.as_ref() == Some(point.input())
                    && point.input().lexer_config() == config
                    && (point
                        .owns_ancestor_origin(self.procedure.declaration.site().source.as_ref())
                        || point.conditional_region().is_some_and(|region| {
                            region.owns_procedure_declaration(&self.procedure.declaration)
                        }))
            }))
            && self.procedure.declaration.matches_context(context)
            && self
                .original
                .iter()
                .all(|word| word.image() == image && word.config() == config)
            && self
                .prefix
                .iter()
                .all(|word| word.original.image() == image && word.original.config() == config)
            && self.caller.as_ref().is_none_or(|body| {
                body.matches_source(image, config) && body.matches_context(context)
            })
    }
}

/// Genuine original declaration body, independently of installed or entered state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct OriginalSourceProcedureBody {
    pub(super) declaration: Arc<OriginalSourceCommandTransitionAdvice>,
    pub(super) body: Arc<OriginalSourceScriptBody>,
    pub(super) slot: ByteCommandSlot,
    pub(super) declaration_slot: ByteCommandSlot,
    pub(super) declaration_namespace: tcl_core_types::ByteNamespacePath,
    pub(super) name: SourceAdviceNameInput,
    pub(super) lineage: Vec<Arc<OriginalSourceCommandTransition>>,
}
impl OriginalSourceProcedureBody {
    pub(super) fn source_procedure<'a>(
        &self,
        analysis: &'a crate::analyser::AnalysisResult,
    ) -> Option<
        &'a crate::signature_scan::original_name::SourceDeclarationMetadata<
            crate::analyser::ProcDef,
        >,
    > {
        let input = analysis.resolved_input.as_ref()?;
        let original = self.name.original_word()?;
        self.declaration
            .matches_source(original.image(), input.lexer_config())
            .then_some(())?;
        self.declaration
            .matches_context(&input.context_registry())
            .then_some(())?;
        analysis
            .matches_original_source_image(original.image(), input.lexer_config())
            .then_some(())?;
        let mut records = analysis.original_procedure_declarations().filter(|record| {
            record.declaration_site() == self.declaration.site()
                && record.name_input().original_word() == original
                && record.metadata().source_name.as_ref() == Some(record.name())
        });
        let record = records.next()?;
        records.next().is_none().then_some(record)
    }

    pub(super) fn moved(
        &self,
        slot: ByteCommandSlot,
        transition: &Arc<OriginalSourceCommandTransition>,
    ) -> Self {
        let mut moved = self.clone();
        moved.slot = slot;
        moved.lineage.push(Arc::clone(transition));
        moved
    }
}
impl AdviceGraph {
    fn original_procedure(
        &self,
        head: &SourceAdviceNameInput,
    ) -> Option<Arc<OriginalSourceProcedureBody>> {
        let mut key = self.selected_head_key(head.bytes())?;
        let mut seen = Vec::new();
        let mut lineage = Vec::new();
        loop {
            if seen.contains(&key) {
                return None;
            }
            seen.push(key.clone());
            match self.cells.get(&key)? {
                AdviceCell::SourceProcedure(procedure) => {
                    let mut procedure = procedure.as_ref().clone();
                    procedure.lineage.extend(lineage);
                    return Some(Arc::new(procedure));
                }
                AdviceCell::Alias {
                    target,
                    lookup,
                    lineage: steps,
                    ..
                } => {
                    lineage.extend(steps.iter().cloned());
                    key = self.alias_target_key(target, *lookup)?;
                }
                _ => return None,
            }
        }
    }
}
impl AdviceInvocationContext<'_> {
    pub(super) fn source_procedure_declaration(
        &self,
        graph: &AdviceGraph,
        invocation: AdviceInvocation<'_>,
        schema: &tcl_registry::ResolvedInvocation<'_, '_>,
    ) -> Option<Arc<OriginalSourceProcedureBody>> {
        // naming.source.original-future-procedure-body

        if !matches!(self.policy, AdviceNamingPolicy::Native(_))
            || !schema.semantics.traits.contains(Traits::DEFERS_BODY)
            || schema.facts().arity_accepts_frozen_arguments() != Some(true)
            || schema.semantics.body_interpreter.resolve_with(|ordinal| {
                std::str::from_utf8(invocation.arguments.get(ordinal)?.value.as_deref()?).ok()
            }) != tcl_registry::world_effect::InterpreterScope::Current
        {
            return None;
        }
        let transitions = schema.state_transitions();
        let mut definitions =
            transitions
                .command_bindings()
                .filter_map(|transition| match transition {
                    CommandBindingTransition::Define {
                        name,
                        kind: CommandBindingDefinitionKind::Procedure,
                    } => Some(name),
                    _ => None,
                });
        let name = definitions.next()?;
        if definitions.next().is_some() {
            return None;
        }
        let name = invocation
            .arguments
            .get(name.argument_index()?)?
            .input
            .as_ref()?;
        let slot = graph.publication_key(name.bytes(), AdvicePublicationPurpose::Define)?;
        graph.namespaces.contains(&slot.namespace).then_some(())?;
        let mut declaration = self.schema_advice(invocation, graph, schema)?;
        let roles = declaration.roles()?;
        let mut parameter_roles = roles.iter().filter(|(_, role)| *role == ArgRole::ParamList);
        let parameters = invocation
            .arguments
            .get(parameter_roles.next()?.0)?
            .original
            .clone();
        if parameter_roles.next().is_some() {
            return None;
        }
        if matches!(
            crate::signature_scan::formal_count::SourceFormalCount::from_original_word(
                &parameters,
                self.dialect
            ),
            crate::signature_scan::formal_count::SourceFormalCount::Unknown
        ) {
            return None;
        }
        declaration.procedure_body.clone_from(&graph.procedure_body);
        let words =
            crate::registry_invocation::source_structure::source_transition_words_from_advice(
                self.origin.source_image().try_text().ok()?,
                self.config,
                self.context,
                declaration.clone(),
            )?;
        let mut bodies = words.source_script_bodies(self.context).into_iter();
        let body = bodies.next()?;
        if bodies.next().is_some()
            || !body.matches_source(self.origin.source_image(), self.config)
            || !body.matches_context(self.context)
        {
            return None;
        }
        let plan = tcl_lexer::native_script_words_in(
            self.origin.source_image().clone(),
            body.content_span(),
            self.config,
        )
        .ok()?;
        if plan.fatal_tail.is_some() {
            return None;
        }
        Some(Arc::new(OriginalSourceProcedureBody {
            declaration: Arc::new(declaration),
            body: Arc::new(body),
            declaration_slot: slot.clone(),
            slot,
            declaration_namespace: graph.native_source_context()?.namespace.clone(),
            name: name.clone(),
            lineage: invocation.lineage.to_vec(),
        }))
    }

    pub(super) fn retain_native_procedure_may_command(
        &self,
        tape: &mut OriginalSourceTransitionAdviceTape,
        graph: &AdviceGraph,
        native: &[tcl_lexer::NativeWord],
        written: &[SourceAdviceWord],
    ) {
        if !matches!(self.policy, AdviceNamingPolicy::Native(_))
            || native.iter().any(|word| word.group().expand)
            || written.len() != native.len()
            || written
                .iter()
                .zip(native)
                .any(|(word, native)| word.original != *native)
        {
            return;
        }
        let (Some(body), Some(head)) = (
            graph.procedure_body.as_ref(),
            written.first().and_then(|word| word.input.as_ref()),
        ) else {
            return;
        };
        if head.native_input().is_none()
            || (graph.blocks_registry_source(head) && graph.original_procedure(head).is_none())
        {
            return;
        }
        let Some(offset) = native.first().map(|word| word.span().start()) else {
            return;
        };
        let receipt = super::native_declared_body::OriginalNativeSourceMayCommand {
            original: Arc::from(native),
            head: head.clone(),
            body: Arc::clone(body),
        };
        super::source_class::merge_receipt(
            &mut tape.native_procedure_may_commands,
            offset,
            Some(receipt),
        );
    }

    pub(super) fn retain_original_source_receiver(
        &self,
        tape: &mut OriginalSourceTransitionAdviceTape,
        graph: &mut AdviceGraph,
        native: &[tcl_lexer::NativeWord],
        written: &[SourceAdviceWord],
    ) -> bool {
        if self.retain_class_instance_invocation(tape, graph, native, written) {
            graph.widen(native);
            return true;
        }
        if self.retain_registered_invocation(tape, graph, native, written) {
            self.finish_registered_invocation(graph, native, written);
            return true;
        }
        if self.retain_source_constructor_call(tape, graph, native, written) {
            let instance = tape
                .constructor_call(native[0].span().start())
                .and_then(|call| self.class_instance_from_call(call, graph));
            graph.widen(native);
            Self::install_source_class_instance(graph, instance);
            return true;
        }
        if self.retain_source_class_proc_call(tape, graph, native, written) {
            graph.widen(native);
            return true;
        }
        if self.retain_original_procedure_call(tape, graph, native, written) {
            graph.widen(native);
            return true;
        }
        false
    }

    fn retain_original_procedure_call(
        &self,
        tape: &mut OriginalSourceTransitionAdviceTape,
        graph: &AdviceGraph,
        native: &[tcl_lexer::NativeWord],
        written: &[SourceAdviceWord],
    ) -> bool {
        let Some(procedure) =
            self.retain_original_procedure_call_receipt(tape, graph, native, written)
        else {
            return false;
        };
        self.retain_original_procedure_body(tape, graph, &procedure)
    }

    pub(super) fn retain_original_procedure_call_receipt(
        &self,
        tape: &mut OriginalSourceTransitionAdviceTape,
        graph: &AdviceGraph,
        native: &[tcl_lexer::NativeWord],
        written: &[SourceAdviceWord],
    ) -> Option<Arc<OriginalSourceProcedureBody>> {
        if native.is_empty()
            || written.len() != native.len()
            || written
                .iter()
                .zip(native)
                .any(|(word, original)| word.original != *original)
        {
            return None;
        }
        let head = written.first().and_then(|word| word.input.as_ref())?;
        let mut selected = graph.resolve_source_cell(head)?;
        let AdviceCell::SourceProcedure(procedure) = selected.cell else {
            return None;
        };
        let mut procedure = procedure.as_ref().clone();
        procedure.lineage.extend(selected.lineage);
        let procedure = Arc::new(procedure);
        for (ordinal, word) in selected.prefix.iter_mut().enumerate() {
            word.origin = crate::registry_invocation::InvocationWordOrigin::BindingPrefix(ordinal);
        }
        let call = OriginalSourceProcedureCall {
            original: Arc::from(native),
            procedure: Arc::clone(&procedure),
            prefix: selected.prefix,
            caller: graph.procedure_body.clone(),
            uncertain_operations: graph.uncertain_operations.clone(),
            future_body: graph.future_procedure_source,
            input: None,
            analysis_owner: None,
            inherited_entry: None,
        };
        super::source_class::merge_receipt(
            &mut tape.native_procedure_calls,
            native[0].span().start(),
            Some(call),
        );
        Some(procedure)
    }

    /// Genuine selected bodies supply conditional future source assistance.
    /// Earlier actual source scans are not overwritten by a later possibility.
    pub(super) fn retain_future_original_procedure_bodies(
        &self,
        tape: &mut OriginalSourceTransitionAdviceTape,
        graph: &AdviceGraph,
    ) {
        if !matches!(self.policy, AdviceNamingPolicy::Native(_)) {
            return;
        }
        let mut future = graph.clone();
        future.future_procedure_source = true;
        let mut procedures = graph
            .cells
            .values()
            .filter_map(|cell| match cell {
                AdviceCell::SourceProcedure(procedure) => Some(Arc::clone(procedure)),
                _ => None,
            })
            .collect::<Vec<_>>();
        procedures.sort_by_key(|procedure| procedure.declaration.site().offset);
        for procedure in procedures {
            if !tape
                .scanned_procedure_declarations
                .contains(&procedure.declaration.site().offset)
            {
                self.retain_original_procedure_body(tape, &future, &procedure);
            }
        }
    }

    fn retain_original_procedure_body(
        &self,
        tape: &mut OriginalSourceTransitionAdviceTape,
        graph: &AdviceGraph,
        procedure: &OriginalSourceProcedureBody,
    ) -> bool {
        let site = procedure.declaration.site();
        if &site.source != self.origin {
            return true;
        }
        tape.scanned_procedure_declarations.insert(site.offset);
        if graph.procedure_path.len() >= MAX_DEPTH
            || graph.procedure_path.contains(site)
            || !procedure
                .declaration
                .matches_source(self.origin.source_image(), self.config)
            || !procedure.declaration.matches_context(self.context)
            || !procedure
                .body
                .matches_source(self.origin.source_image(), self.config)
            || !procedure.body.matches_context(self.context)
        {
            return true;
        }
        let Ok(plan) = tcl_lexer::native_script_words_in(
            self.origin.source_image().clone(),
            procedure.body.content_span(),
            self.config,
        ) else {
            return true;
        };
        if plan.fatal_tail.is_some() {
            return true;
        }
        let mut branch = graph.clone();
        branch.procedure_body = Some(Arc::clone(&procedure.body));
        branch.procedure_namespace = matches!(
            self.policy
                .native()
                .map(tcl_syntax::naming::NamePolicyProtocol::recipe),
            Some(tcl_syntax::naming::NativeNameProtocol::C(_))
        )
        .then(|| procedure.slot.namespace.clone());
        branch.procedure_path.push(site.clone());
        // A parent/global source handle is not a local parameter or variable.
        branch.handles.clear();
        branch.class_handles.clear();
        let mut inventory = OriginalSourceTransitionAdviceTape::default();
        for command in plan.commands {
            if branch.procedure_steps.fetch_add(1, Ordering::Relaxed) >= MAX_COMMANDS {
                break;
            }
            if self
                .retain_original_invocation(
                    &mut branch,
                    &mut inventory,
                    &command,
                    super::logical_navigation::LogicalCallbackScope::Unavailable,
                )
                .is_none()
            {
                return true;
            }
        }
        tape.extend_inventory(inventory);
        // The conditional branch never publishes body effects to its caller.
        true
    }
}
pub(super) fn install_original_procedure(
    graph: &mut AdviceGraph,
    procedure: Option<Arc<OriginalSourceProcedureBody>>,
) {
    if let Some(procedure) = procedure {
        graph.procedure_declarations.push(Arc::clone(&procedure));
        graph.cells.insert(
            procedure.slot.clone(),
            AdviceCell::SourceProcedure(procedure),
        );
    }
}

#[cfg(test)]
mod tests {
    use crate::analyser::Analyser;
    use crate::registry_invocation::source_structure::source_registered_instance_words_at;

    #[test]
    fn source_procedure_calls_retain_actual_horizon_and_unentered_caller_body() {
        let source = "proc target {} {}; proc owner {} {target}; owner; rename target held";
        let analysis = crate::analyser::Analyser::new().analyse(source, "tcl8.6");
        let offset = u32::try_from(source.find("{target}").unwrap() + 1).unwrap();
        let call = crate::registry_invocation::source_structure::source_original_procedure_call_at(
            source, &analysis, offset,
        )
        .expect("actual source call before later rename");
        assert_eq!(
            call.declaration(&analysis).unwrap().name_input().bytes(),
            b"target"
        );
        assert_eq!(
            call.caller_declaration(&analysis)
                .unwrap()
                .name_input()
                .bytes(),
            b"owner"
        );
        let body = call
            .caller_body()
            .expect("authentic owner body without entry");
        assert_eq!(&source[body.content_span().as_range()], "target");
        assert!(!call.declaration_advice().obligations().is_empty());
        assert!(analysis.original_completed_command_world().is_none());
        assert!(
            crate::registry_invocation::source_structure::source_original_procedure_call_at(
                &source.replace("held", "gone"),
                &analysis,
                offset,
            )
            .is_none()
        );
        let independent = crate::analyser::Analyser::new().analyse(source, "tcl8.6");
        assert!(call.declaration(&independent).is_none());
        assert!(call.caller_declaration(&independent).is_none());
        let foreign = crate::analyser::Analyser::new().analyse(source, "tcl9.0");
        assert!(call.declaration(&foreign).is_none());
    }

    #[test]
    fn original_future_procedure_joins_later_current_factory_with_authentic_parent_body() {
        // naming.source.original-future-procedure-body

        for source in [
            "proc touch {} {.t bogus}; ttk::treeview .t; touch",
            "ttk::treeview .t; proc touch {} {.t bogus}; touch",
        ] {
            let analysis = Analyser::new().analyse(source, "tcl8.6");
            let offset = u32::try_from(source.find(".t bogus").unwrap()).unwrap();
            let receipt =
                source_registered_instance_words_at(source, &analysis, offset).expect(source);
            let body = receipt.source_body().expect("authentic deferred parent");
            assert_eq!(body.original_container().span().start(), offset - 1);
            assert_eq!(receipt.instance().factory().command(), "ttk::treeview");
            assert!(receipt.obligations().contains(&super::super::SourceCommandTransitionObligation::OriginalProcedureBodyApplicability));
            assert!(
                source_registered_instance_words_at(
                    &source.replace("bogus", "other"),
                    &analysis,
                    offset
                )
                .is_none()
            );
        }
    }

    #[test]
    fn original_uncalled_procedure_uses_authentic_final_source_graph_without_call_or_frame() {
        // naming.source.original-future-procedure-body

        for source in [
            "proc p {} {C new one}; oo::class create C {constructor {x y} { }}",
            "oo::class create C {constructor {x y} { }}; proc p {} {C new one}",
        ] {
            let analysis = Analyser::new().analyse(source, "tcl8.6");
            let offset = u32::try_from(source.find("C new one").unwrap()).unwrap();
            let call = crate::registry_invocation::source_structure::source_constructor_call_at(
                source, &analysis, offset,
            )
            .expect(source);
            assert!(call.original_source_body().is_some());
            assert!(call.obligations().contains(&super::super::SourceCommandTransitionObligation::FutureOriginalProcedureSourceApplicability));
            assert_eq!(
                call.class_declaration()
                    .source_class(&analysis)
                    .unwrap()
                    .name_input()
                    .bytes(),
                b"C"
            );
            assert_eq!(call.original_words()[0].span().start(), offset);
            assert_eq!(
                call.constructor_shape(&analysis)
                    .unwrap()
                    .constructor_args_from(),
                1
            );
        }
        for source in [
            "proc p {} {C new one}; oo::class create C {}; rename p {}",
            "proc p {} {C new one}; oo::class create C {}; proc p {} {}",
            "proc p {} {C new one}; oo::class create C {}; rename C {}",
        ] {
            let analysis = Analyser::new().analyse(source, "tcl8.6");
            let offset = u32::try_from(source.find("C new one").unwrap()).unwrap();
            assert!(
                crate::registry_invocation::source_structure::source_constructor_call_at(
                    source, &analysis, offset
                )
                .is_none(),
                "{source}"
            );
        }
    }

    #[test]
    fn original_future_procedure_source_roles_retain_independently_selected_c_and_jim_grammar() {
        // naming.source.original-future-procedure-body

        let source = "proc p {} {if {1} {puts value}}; p";
        for dialect in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1", "jimtcl"] {
            let analysis = Analyser::new().analyse(source, dialect);
            let input = analysis.resolved_input.as_ref().unwrap();
            let context = input.context_registry();
            let tape = analysis
                .retained_command_realm()
                .unwrap()
                .source_bindings_ref()
                .original_source_transition_advice_tape(&context)
                .unwrap();
            let offset = u32::try_from(source.find("if {1}").unwrap()).unwrap();
            let super::super::OriginalSourceTransitionAdviceLookup::Advice(advice) =
                tape.lookup(offset)
            else {
                panic!("{dialect}");
            };
            assert_eq!(advice.command(), "if");
            assert!(advice.original_procedure_source_body().is_some());
            assert!(advice.matches_context(&context));
            assert_eq!(advice.original_words()[0].config(), input.lexer_config());
        }
    }

    #[test]
    fn original_future_procedure_refuses_known_replacement_deletion_and_malformed_formals() {
        // naming.source.original-future-procedure-body

        for source in [
            "proc touch {} {.t bogus}; ttk::treeview .t; proc touch {} {}; touch",
            "proc touch {} {.t bogus}; ttk::treeview .t; rename touch {}; touch",
            "proc touch {{a b c}} {.t bogus}; ttk::treeview .t; touch",
        ] {
            let analysis = Analyser::new().analyse(source, "tcl8.6");
            let offset = u32::try_from(source.find(".t bogus").unwrap()).unwrap();
            assert!(
                source_registered_instance_words_at(source, &analysis, offset).is_none(),
                "{source}"
            );
        }
    }

    #[test]
    fn original_future_procedure_local_handle_does_not_borrow_parent_value_or_escape_branch() {
        // naming.source.original-future-procedure-body

        let source = "set lb [listbox .l]; proc p {} {$lb bogus}; p";
        let analysis = Analyser::new().analyse(source, "tcl8.6");
        let offset = u32::try_from(source.find("$lb bogus").unwrap()).unwrap();
        assert!(source_registered_instance_words_at(source, &analysis, offset).is_none());
        let source = "proc p {} {set local [listbox .new]; $local bogus}; p; $local bogus";
        let analysis = Analyser::new().analyse(source, "tcl8.6");
        let inside = u32::try_from(source.find("$local bogus").unwrap()).unwrap();
        let outside = u32::try_from(source.rfind("$local bogus").unwrap()).unwrap();
        assert!(
            source_registered_instance_words_at(source, &analysis, inside)
                .unwrap()
                .source_body()
                .is_some()
        );
        assert!(source_registered_instance_words_at(source, &analysis, outside).is_none());
    }
}

#[cfg(test)]
mod original_captured_call_controls {
    use crate::analyser::Analyser;
    use crate::registry_invocation::source_structure::source_original_procedure_call_at;

    #[test]
    fn same_original_native_source_call_retains_actual_alias_prefix_count() {
        for dialect in ["tcl8.4", "tcl8.6", "tcl9.0", "jimtcl"] {
            let source = "proc target {a b c} {}; interp alias {} one {} target X; interp alias {} two {} one Y; two Z";
            let analysis = Analyser::new().analyse(source, dialect);
            let offset = u32::try_from(source.rfind("two Z").unwrap()).unwrap();
            let call = source_original_procedure_call_at(source, &analysis, offset).unwrap();
            assert_eq!(call.captured_argument_count(), 2);
            assert_eq!(call.original_words().len(), 2);
            assert_eq!(call.original_words()[0].span().start(), offset);
            assert_eq!(
                call.declaration(&analysis).unwrap().name_input().bytes(),
                b"target"
            );
            assert!(!call.lineage().is_empty());
        }
        let source = "proc target {a b} {}; interp alias {} one {} target X; interp alias {} two {} one; rename one {}; two Z";
        let analysis = Analyser::new().analyse(source, "tcl8.6");
        let offset = u32::try_from(source.rfind("two Z").unwrap()).unwrap();
        assert!(source_original_procedure_call_at(source, &analysis, offset).is_none());
    }
}
