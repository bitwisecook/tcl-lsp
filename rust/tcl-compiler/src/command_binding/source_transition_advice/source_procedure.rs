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
        let mut key = self.lookup_head_key(head.bytes())?;
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
        // docs/design/analysis/name-resolution-proofs/source-original-future-procedure-body.md
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
        if native.is_empty()
            || written.len() != native.len()
            || written
                .iter()
                .zip(native)
                .any(|(word, original)| word.original != *original)
        {
            return false;
        }
        let Some(head) = written.first().and_then(|word| word.input.as_ref()) else {
            return false;
        };
        let Some(procedure) = graph.original_procedure(head) else {
            return false;
        };
        self.retain_original_procedure_body(tape, graph, &procedure)
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
                .retain_original_invocation(&mut branch, &mut inventory, &command)
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
    fn original_future_procedure_joins_later_current_factory_with_authentic_parent_body() {
        // naming.source.original-future-procedure-body
        // docs/design/analysis/name-resolution-proofs/source-original-future-procedure-body.md
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
        // docs/design/analysis/name-resolution-proofs/source-original-future-procedure-body.md
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
        // docs/design/analysis/name-resolution-proofs/source-original-future-procedure-body.md
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
        // docs/design/analysis/name-resolution-proofs/source-original-future-procedure-body.md
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
        // docs/design/analysis/name-resolution-proofs/source-original-future-procedure-body.md
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
