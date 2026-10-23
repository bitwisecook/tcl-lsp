// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Original procedure body coverage, independently of callable IR registration.

use super::{
    AllocationIncarnation, BindingKind, CommandAllocation, DeferredImplementationId,
    ExecutedScriptSource, MayBinding, SourceCommandBindings, SourceCommandTarget,
};

/// Original body of one exact retained procedure implementation. The caller
/// independently proves that implementation is selected at its use site.
/// This inventory supplies no body-entry, completion or compiler authority.
#[derive(Debug, Clone, Copy)]
pub struct SourceProcedureImplementationBody<'a> {
    /// Full implementation allocation, including its incarnation.
    pub allocation: &'a CommandAllocation,
    /// Original namespace presentation, with no native lookup authority.
    pub namespace: &'a str,
    /// Exact original namespace allocation or native incarnation.
    pub namespace_key: &'a super::SourceNamespaceKey,
    /// Original native formal binding plan.
    pub parameters: &'a [tcl_syntax::formal_params::FormalParameter],
    /// Original byte-valued formal topology, independent of presentation.
    pub(crate) original_parameters: Option<&'a super::formal_topology::OriginalFormalTopology>,
    /// Original evaluated body source and provenance.
    pub source: &'a ExecutedScriptSource,
}

#[cfg(test)]
mod implementation_tests {
    use super::*;

    #[test]
    fn historical_procedure_bodies_require_the_full_selected_implementation() {
        let source = "proc f {} {return FIRST}; f; proc f {} {return SECOND}; f";
        let owner = tcl_registry::model::ingress::static_context_for("tcl8.6");
        let profile = owner.commands().profile().expect("actual Tcl 8.6 fixture");
        let bindings = SourceCommandBindings::analyse_with_options(
            source,
            tcl_lexer::LexerConfig::from_grammar(profile.grammar),
            owner.commands(),
            super::super::SourceAnalysisOptions {
                invocation_dialect: Some(tcl_registry::InvocationDialect::of_profile(profile)),
                native_compilation: crate::environment_ingress::authoring_native_compilation(),
                ..Default::default()
            },
        );
        let bodies = source
            .match_indices("; f")
            .zip(["return FIRST", "return SECOND"])
            .map(|((offset, _), expected)| {
                let binding =
                    bindings.invocation_at_source("f", u32::try_from(offset + 2).unwrap());
                let target = binding
                    .proved_handler_target()
                    .expect("actual procedure target");
                let body = bindings
                    .procedure_implementation_body(target)
                    .expect("original body");
                assert_eq!(body.source.text.try_text().unwrap(), expected);
                assert_eq!(
                    Some(body.allocation),
                    target.implementation_allocation.as_ref()
                );
                body.allocation.clone()
            })
            .collect::<Vec<_>>();
        assert_eq!(bodies.len(), 2);
        assert_ne!(bodies[0], bodies[1]);
        let inventory = bindings
            .procedure_implementation_bodies()
            .map(|body| body.allocation.clone())
            .collect::<Vec<_>>();
        assert!(
            bodies
                .iter()
                .all(|allocation| inventory.contains(allocation))
        );
    }
}

/// Original body metadata retained for analysis coverage. The issuing query
/// independently selects original declarations or currently installed bodies.
/// This supplies no callable, body-entry or compilation authority.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceInstalledProcedureBody {
    /// Exact native definition allocation, including its source instance.
    pub allocation: CommandAllocation,
    /// Command presentation; native slot identity is retained independently.
    pub command: String,
    /// Namespace presentation selected by the issuing original-body query.
    pub namespace: String,
    /// Exact namespace context, independent of its display spelling.
    pub namespace_key: super::SourceNamespaceKey,
    /// Formals decoded by the selected native definition grammar.
    pub parameters: Vec<tcl_syntax::formal_params::FormalParameter>,
    /// Authentic original formal producer and selected binding grammar.
    pub(crate) original_parameters: Option<super::formal_topology::OriginalFormalTopology>,
    /// Exact evaluated body and its retained source origin.
    pub source: ExecutedScriptSource,
}

impl SourceCommandBindings {
    /// Original procedure implementations retained for body analysis. This
    /// includes replaced declarations and does not select a callable at any site.
    /// Consumers match a separately proved current implementation allocation.
    pub fn procedure_implementation_bodies(
        &self,
    ) -> impl Iterator<Item = SourceProcedureImplementationBody<'_>> {
        self.deferred.values().filter_map(|root| {
            if root.receiver_method || root.event.is_some() || root.statics.is_some() {
                return None;
            }
            let allocation = root.implementation_allocation.as_ref()?;
            if allocation.incarnation == AllocationIncarnation::RepeatedFresh {
                return None;
            }
            let (_, _, source) = root.executed_script.as_ref()?;
            Some(SourceProcedureImplementationBody {
                allocation,
                namespace: &root.namespace,
                namespace_key: &root.namespace_key,
                parameters: &root.parameters,
                original_parameters: root.original_parameters.as_ref(),
                source,
            })
        })
    }

    /// Original inventory for this exact implementation, including replaced
    /// declarations. Current dispatch must come from a separate positioned receipt;
    /// final command names and future declarations cannot select this body.
    #[must_use]
    pub fn procedure_implementation_body(
        &self,
        target: &SourceCommandTarget,
    ) -> Option<SourceProcedureImplementationBody<'_>> {
        if target.kind != BindingKind::Proc {
            return None;
        }
        let allocation = target.implementation_allocation.as_ref()?;
        if allocation.incarnation == AllocationIncarnation::RepeatedFresh {
            return None;
        }
        let root = self.deferred.get(&target.implementation_id())?;
        if root.receiver_method || root.event.is_some() || root.statics.is_some() {
            return None;
        }
        let allocation = root
            .implementation_allocation
            .as_ref()
            .filter(|retained| *retained == allocation)?;
        let (_, _, source) = root.executed_script.as_ref()?;
        Some(SourceProcedureImplementationBody {
            allocation,
            namespace: &root.namespace,
            namespace_key: &root.namespace_key,
            parameters: &root.parameters,
            original_parameters: root.original_parameters.as_ref(),
            source,
        })
    }

    /// Retain every original declaration body with its exact site, source and
    /// namespace key. Final callable state is independent: deletion, replacement
    /// and an opaque continuation cannot erase original declaration coverage.
    /// This inventory never proves that a body entered or remains callable.
    #[must_use]
    pub fn original_declaration_body_units(&self) -> Vec<SourceInstalledProcedureBody> {
        let mut seen = std::collections::HashSet::new();
        let mut bodies = self
            .procedure_implementation_bodies()
            .filter(|body| {
                seen.insert((
                    body.allocation.clone(),
                    body.namespace_key.clone(),
                    body.source.clone(),
                ))
            })
            .map(|body| SourceInstalledProcedureBody {
                allocation: body.allocation.clone(),
                command: body.allocation.command.clone(),
                namespace: body.namespace.to_owned(),
                namespace_key: body.namespace_key.clone(),
                parameters: body.parameters.to_vec(),
                original_parameters: body.original_parameters.cloned(),
                source: body.source.clone(),
            })
            .collect::<Vec<_>>();
        bodies.sort_by(|left, right| {
            left.allocation
                .cmp(&right.allocation)
                .then_with(|| left.namespace_key.cmp(&right.namespace_key))
        });
        bodies
    }

    /// Recover only singleton procedure implementations still installed at the
    /// document boundary. Retired/replaced, opaque, repeated and static-bound
    /// implementations decline; future-only declarations grant no evidence.
    #[must_use]
    pub fn installed_procedure_body_units(&self) -> Vec<SourceInstalledProcedureBody> {
        let state = &self.final_state;
        if state.has_opaque_domain() {
            return Vec::new();
        }
        let mut bodies = Vec::new();
        for (slot, bindings) in state.bindings.iter() {
            if bindings.len() != 1 {
                continue;
            }
            let Some(MayBinding::Target(target)) = bindings.first() else {
                continue;
            };
            if !target.terminal || target.kind != BindingKind::Proc || !target.prepended.is_empty()
            {
                continue;
            }
            let Some(allocation) = target
                .implementation_allocation
                .as_ref()
                .filter(|allocation| {
                    allocation.incarnation != AllocationIncarnation::RepeatedFresh
                })
            else {
                continue;
            };
            let Some(identity) = target.token.as_ref() else {
                continue;
            };
            let key = DeferredImplementationId {
                command: identity.origin.clone(),
                generation: target.implementation_generation,
                allocation: Some(allocation.clone()),
            };
            let Some(root) = self.deferred.get(&key).filter(|root| {
                root.event.is_none() && !root.receiver_method && root.statics.is_none()
            }) else {
                continue;
            };
            let Some(command) =
                crate::command_binding::ModuleCommandBindings::command_key_label(slot)
            else {
                continue;
            };
            let namespace_key = slot.holder().into_owned();
            let namespace = namespace_key.display().unwrap_or_default();
            let Some((_, _, source)) = &root.executed_script else {
                continue;
            };
            bodies.push(SourceInstalledProcedureBody {
                allocation: allocation.clone(),
                command,
                namespace,
                namespace_key,
                parameters: root.parameters.clone(),
                original_parameters: root.original_parameters.clone(),
                source: source.clone(),
            });
        }
        bodies.sort_by(|left, right| left.allocation.cmp(&right.allocation));
        bodies.dedup_by(|left, right| left.allocation == right.allocation);
        bodies
    }
}
#[derive(Debug, Clone, PartialEq, Eq)]
enum OriginalNativeProcedureRecipe {
    ReadFlow(Box<OriginalNativeProcedureReadFlow>),
    Source(std::sync::Arc<super::source_transition_advice::OriginalNativeSourceDeclaration>),
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct OriginalNativeProcedureReadFlow {
    declaration: super::declaration_preview::DeclaredProcedureBody,
    read_flow: super::declaration_preview::OriginalProcedureReadFlowEntry,
}

/// Original Native-name procedure syntax under its authentic source body owner.
/// A read-flow recipe and a frame-free source recipe remain separate purposes;
/// neither supplies an implementation allocation or an entered body.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OriginalNativeProcedureBody {
    recipe: OriginalNativeProcedureRecipe,
    input: crate::analyser::ResolvedAnalysisInput,
    lowered_body: Option<std::sync::Arc<crate::ir::Script>>,
    module_entry: Option<super::SourceAnalysisEntry>,
}
impl OriginalNativeProcedureBody {
    pub(crate) fn site(&self) -> &super::CommandAllocationSite {
        match &self.recipe {
            OriginalNativeProcedureRecipe::ReadFlow(flow) => flow.declaration.declaration_site(),
            OriginalNativeProcedureRecipe::Source(source) => source.site(),
        }
    }
    pub(crate) fn namespace(&self) -> &super::SourceNamespaceKey {
        match &self.recipe {
            OriginalNativeProcedureRecipe::ReadFlow(flow) => &flow.read_flow.header.namespace,
            OriginalNativeProcedureRecipe::Source(source) => source.namespace(),
        }
    }
    pub(crate) fn source_slot(&self) -> &tcl_core_types::ByteCommandSlot {
        match &self.recipe {
            OriginalNativeProcedureRecipe::ReadFlow(flow) => &flow.read_flow.header.source_slot,
            OriginalNativeProcedureRecipe::Source(source) => source.declaration_slot(),
        }
    }
    pub(crate) fn original_name_input(
        &self,
    ) -> &crate::signature_scan::scope::SignatureSourceNameInput {
        match &self.recipe {
            OriginalNativeProcedureRecipe::ReadFlow(flow) => &flow.read_flow.header.name_input,
            OriginalNativeProcedureRecipe::Source(source) => source.name_input(),
        }
    }
    pub(crate) fn name(&self) -> Option<&str> {
        std::str::from_utf8(self.original_name_input().bytes()).ok()
    }
    pub(crate) fn reporting_label(&self) -> Option<String> {
        match &self.recipe {
            OriginalNativeProcedureRecipe::ReadFlow(flow) => flow.declaration.command_label(),
            OriginalNativeProcedureRecipe::Source(source) => source.label(),
        }
    }
    pub(crate) fn parameters(&self) -> &[tcl_syntax::formal_params::FormalParameter] {
        match &self.recipe {
            OriginalNativeProcedureRecipe::ReadFlow(flow) => &flow.read_flow.header.parameters,
            OriginalNativeProcedureRecipe::Source(source) => source.parameters(),
        }
    }
    pub(crate) fn parameters_text(&self) -> Option<&str> {
        match &self.recipe {
            OriginalNativeProcedureRecipe::ReadFlow(flow) => flow.read_flow.read.parameters_text(),
            OriginalNativeProcedureRecipe::Source(source) => source.parameters_text(),
        }
    }
    pub(crate) fn source_declaration_span(&self) -> Option<tcl_lexer::Span> {
        match &self.recipe {
            OriginalNativeProcedureRecipe::Source(source) => source.declaration_span(),
            OriginalNativeProcedureRecipe::ReadFlow(_) => None,
        }
    }
    pub(crate) fn body_span(&self) -> Option<tcl_lexer::Span> {
        match &self.recipe {
            OriginalNativeProcedureRecipe::ReadFlow(flow) => {
                flow.read_flow.read.original_body_span()
            }
            OriginalNativeProcedureRecipe::Source(source) => Some(source.body_span()),
        }
    }
    pub(crate) fn body_text(&self) -> Option<&str> {
        match &self.recipe {
            OriginalNativeProcedureRecipe::ReadFlow(flow) => {
                flow.read_flow.read.original_body_text()
            }
            OriginalNativeProcedureRecipe::Source(source) => source.body_text(),
        }
    }
    pub(crate) fn source_children(
        &self,
        bindings: &SourceCommandBindings,
    ) -> Option<std::sync::Arc<super::source_transition_advice::OriginalNativeSourceBodyChildren>>
    {
        match &self.recipe {
            OriginalNativeProcedureRecipe::Source(source) => source.children(bindings, &self.input),
            OriginalNativeProcedureRecipe::ReadFlow(_) => None,
        }
    }
    pub(crate) fn owns_source_declaration(
        &self,
        declaration: &std::sync::Arc<
            super::source_transition_advice::OriginalNativeSourceDeclaration,
        >,
    ) -> bool {
        matches!(&self.recipe, OriginalNativeProcedureRecipe::Source(source) if std::sync::Arc::ptr_eq(source, declaration))
    }
    pub(crate) fn retain_lowered_body(
        mut self,
        body: &crate::ir::Script,
        entry: &super::SourceAnalysisEntry,
    ) -> Self {
        self.lowered_body = Some(std::sync::Arc::new(body.clone()));
        self.module_entry = Some(entry.clone());
        self
    }
    pub(crate) fn matches_source(
        &self,
        image: &tcl_lexer::SourceImage,
        input: &crate::analyser::ResolvedAnalysisInput,
        registry: &tcl_registry::CommandRegistry,
    ) -> bool {
        if &self.input != input
            || input.has_logical_source_name_context()
            || input.has_hosted_source_name_context()
            || tcl_registry::InvocationDialect::of_profile(input.unit_profile())
                .native_name_protocol()
                .is_none()
            || input
                .borrowed_context_registry()
                .commands()
                .snapshot()
                .semantic_key()
                != registry.snapshot().semantic_key()
            || self.site().source.source_image() != image
        {
            return false;
        }
        match &self.recipe {
            OriginalNativeProcedureRecipe::Source(source) => source.matches_source(image, input),
            OriginalNativeProcedureRecipe::ReadFlow(flow) => {
                flow.read_flow.read.original_image() == image
                    && flow.read_flow.header.declaration == *self.site()
                    && flow.read_flow.header.namespace == flow.declaration.namespace
                    && flow.read_flow.header.parameters == flow.declaration.parameters
                    && flow.read_flow.header.original_parameters
                        == flow.declaration.original_parameters
                    && flow.read_flow.header.frame == flow.declaration.frame
                    && flow.declaration.source.origin == self.site().source
                    && self.body_span().is_some_and(|span| {
                        flow.read_flow.read.owns_original_read(span)
                            && image.bytes().get(span.as_range()).is_some()
                    })
            }
        }
    }
    pub(crate) fn matches_procedure(
        &self,
        module: &crate::ir::Module,
        procedure: &crate::ir::Procedure,
    ) -> bool {
        let Some(input) = module.source_metadata_input.as_ref() else {
            return false;
        };
        self.module_entry.as_ref() == Some(&module.source_entry)
            && module.native_lexer_config() == input.lexer_config()
            && module.source_entry.metadata_context.source_analysis_input() == Some(input)
            && self.matches_source(&module.source, input, module.resolved_registry())
            && module.registry_snapshot.as_ref().is_some_and(|snapshot| {
                snapshot.semantic_key()
                    == input
                        .borrowed_context_registry()
                        .commands()
                        .snapshot()
                        .semantic_key()
            })
            && procedure.body_kind == crate::ir::ProcedureBodyKind::OriginalDeclaration
            && procedure.publication.is_none()
            && procedure.declaration_site.as_ref() == Some(self.site())
            && self.name() == Some(procedure.name.as_str())
            && procedure.span.start() == self.site().offset
            && self.body_span().is_some_and(|span| {
                procedure.body_offset == span.start() && procedure.span.end() == span.end()
            })
            && procedure.body_source.as_deref() == self.body_text()
            && self.parameters_text() == Some(procedure.params_raw.as_str())
            && procedure
                .params
                .iter()
                .map(String::as_str)
                .eq(self.parameters().iter().map(|p| p.name.as_str()))
            && procedure.body.namespace_context.as_deref() == Some(self.namespace())
            && procedure.body.executed_source.is_none()
            && procedure.body.native_compilation_failure.is_none()
            && procedure.body.native_compilation_admission.is_none()
            && self.lowered_body.as_deref() == Some(&procedure.body)
    }
}

impl SourceCommandBindings {
    /// Retain unanimous original Native declaration/read-flow recipes without
    /// selecting successful publication, an implementation or a runtime frame.
    pub(crate) fn original_native_procedure_body_cards(
        &self,
        input: &crate::analyser::ResolvedAnalysisInput,
        registry: &tcl_registry::CommandRegistry,
    ) -> Vec<std::sync::Arc<OriginalNativeProcedureBody>> {
        if input.has_logical_source_name_context()
            || input.has_hosted_source_name_context()
            || tcl_registry::InvocationDialect::of_profile(input.unit_profile())
                .native_name_protocol()
                .is_none()
            || input
                .borrowed_context_registry()
                .commands()
                .snapshot()
                .semantic_key()
                != registry.snapshot().semantic_key()
        {
            return Vec::new();
        }
        let Some(origin) = self.source_origin() else {
            return Vec::new();
        };
        if !self.matches_original_source_image(origin.source_image(), input.lexer_config()) {
            return Vec::new();
        }
        let mut cards = self
            .declaration_layouts
            .keys()
            .filter_map(|site| {
                if &site.source != origin {
                    return None;
                }
                let declaration = self.original_declared_procedure_at(site, registry)?;
                declaration.original_parameters.as_ref()?;
                let binding = self.invocation_at_source("", site.offset);
                if binding.invocation_site() != Some(site) {
                    return None;
                }
                let (_, mut tokens) = binding.original_recorded_command()?;
                tokens.source_binding = Some(binding);
                let read_flow =
                    self.original_procedure_read_flow_entry(&tokens, registry, input)?;
                let card = OriginalNativeProcedureBody {
                    recipe: OriginalNativeProcedureRecipe::ReadFlow(Box::new(
                        OriginalNativeProcedureReadFlow {
                            declaration,
                            read_flow,
                        },
                    )),
                    input: input.clone(),
                    lowered_body: None,
                    module_entry: None,
                };
                (card.matches_source(origin.source_image(), input, registry)
                    && card.name().is_some()
                    && card.reporting_label().is_some()
                    && card.parameters_text().is_some()
                    && card.body_text().is_some())
                .then(|| std::sync::Arc::new(card))
            })
            .collect::<Vec<_>>();
        if let Some(declarations) = self.original_native_source_declarations(input) {
            for declaration in declarations {
                if cards.iter().any(|card| card.site() == declaration.site()) {
                    continue;
                }
                let card = OriginalNativeProcedureBody {
                    recipe: OriginalNativeProcedureRecipe::Source(declaration),
                    input: input.clone(),
                    lowered_body: None,
                    module_entry: None,
                };
                if card.matches_source(origin.source_image(), input, registry)
                    && card.name().is_some()
                    && card.reporting_label().is_some()
                    && card.parameters_text().is_some()
                    && card.body_text().is_some()
                {
                    cards.push(std::sync::Arc::new(card));
                }
            }
        }
        cards.sort_by(|a, b| a.site().cmp(b.site()));
        cards
    }
}

#[cfg(test)]
mod original_native_body_tests {

    fn module(source: &str, environment: &str) -> crate::ir::Module {
        let context = tcl_registry::model::ingress::resolve_environment(environment)
            .default_context_registry();
        let profile = context.commands().profile().unwrap();
        let config = tcl_lexer::LexerConfig::for_file_grammar(profile.grammar);
        let input = crate::analyser::ResolvedAnalysisInput::new(
            profile,
            profile,
            std::sync::Arc::clone(&context),
            config,
        );
        // Unentered native declarations need an explicit unknown entry; no
        // entry means a fresh interpreter that defines each procedure.
        crate::lowering::lower_to_ir_with(
            crate::lowering::Lowerer::with_config(context.commands(), config)
                .with_resolved_analysis_input(input)
                .with_explicit_unknown_entry(),
            source,
        )
    }

    #[test]
    fn original_native_source_cards_retain_unentered_declarations_and_actual_ids() {
        let source =
            "proc f {x {y DEFAULT}} {set value $x; return $value}\nproc f {} {return SECOND}\n";
        let module = module(source, "tcl8.6");
        assert!(module.source_entry.native_entry.is_none());
        assert!(module.source_entry.unknown_entry);
        assert!(module.procedures.is_empty());
        assert_eq!(module.original_native_declaration_body_units.len(), 2);
        let rows = module
            .original_native_declaration_body_units
            .iter()
            .collect::<Vec<_>>();
        assert_ne!(rows[0].0, rows[1].0);
        let procedures = rows
            .iter()
            .map(|(id, card)| {
                assert!(module.procedure_ids.owns(**id));
                let procedure = module.body_units.get(id).unwrap();
                assert!(card.matches_procedure(&module, procedure));
                assert_eq!(procedure.name, "f");
                assert_eq!(procedure.qualified_name, "::f");
                assert!(procedure.publication.is_none());
                assert!(procedure.body.executed_source.is_none());
                assert!(procedure.body.native_compilation_failure.is_none());
                assert!(procedure.body.native_compilation_admission.is_none());
                assert!(
                    module
                        .original_source_procedure_bodies()
                        .any(|(owned, _)| owned == *id)
                );
                assert!(
                    !module
                        .executable_script_roots()
                        .iter()
                        .any(|(script, _)| std::ptr::eq(*script, &procedure.body))
                );
                procedure
            })
            .collect::<Vec<_>>();
        assert!(procedures.iter().any(|p| p.params == ["x", "y"]));
        assert!(
            procedures
                .iter()
                .any(|p| p.body_source.as_deref() == Some("return SECOND"))
        );
    }

    #[test]
    fn original_native_source_cards_refuse_changed_and_foreign_owner_inputs() {
        let original = module("proc f {x} {return $x}\n", "tcl8.6");
        let (id, card) = original
            .original_native_declaration_body_units
            .iter()
            .next()
            .unwrap();
        let procedure = original.body_units.get(id).unwrap();
        assert!(card.matches_procedure(&original, procedure));
        let mut changed = original.clone();
        changed.source = tcl_lexer::SourceImage::document("proc f {x} {return BAD}\n");
        assert!(!card.matches_procedure(&changed, procedure));
        changed = original.clone();
        changed.source_metadata_input = None;
        assert!(!card.matches_procedure(&changed, procedure));
        changed = original.clone();
        changed.source_entry.unknown_entry = !changed.source_entry.unknown_entry;
        assert!(!card.matches_procedure(&changed, procedure));
        changed = original.clone();
        changed.source_metadata_input =
            module("proc f {x} {return $x}\n", "tcl9.0").source_metadata_input;
        assert!(!card.matches_procedure(&changed, procedure));
        for field in 0..4 {
            let mut forged = procedure.clone();
            match field {
                0 => forged.params[0] = "foreign".into(),
                1 => forged.params_raw = "foreign".into(),
                2 => forged.body_source = Some("return BAD".into()),
                _ => forged.body_offset += 1,
            }
            assert!(!card.matches_procedure(&original, &forged));
        }
        let foreign = module("proc f {x} {return $x}\n", "tcl8.6");
        let foreign_id = *foreign
            .original_native_declaration_body_units
            .keys()
            .next()
            .unwrap();
        assert_ne!(*id, foreign_id);
        changed = original.clone();
        changed.body_units.insert(foreign_id, procedure.clone());
        changed
            .original_native_declaration_body_units
            .insert(foreign_id, std::sync::Arc::clone(card));
        assert!(
            !changed
                .original_source_procedure_bodies()
                .any(|(owned, _)| *owned == foreign_id)
        );
        let mut report_only = procedure.clone();
        report_only.qualified_name = "::unrelated-report".into();
        assert!(card.matches_procedure(&original, &report_only));
    }

    #[test]
    fn native_source_declaration_birth_requires_complete_actual_input() {
        let actual = module("proc f {x} {return $x}\n", "tcl8.6");
        let input = actual.source_metadata_input.as_ref().unwrap();
        let retained = actual.retained_source_bindings.as_ref().unwrap();
        let bindings = retained
            .positioned_for_module(&actual, input.borrowed_context_registry().commands())
            .unwrap();
        assert_eq!(
            bindings
                .original_native_source_declarations(input)
                .unwrap()
                .len(),
            1
        );
        let foreign_profile =
            tcl_registry::model::ingress::resolve_environment("tcl9.0").unit_profile();
        for changed in [
            crate::analyser::ResolvedAnalysisInput::new(
                foreign_profile,
                input.unit_profile(),
                input.context_registry(),
                input.lexer_config(),
            ),
            crate::analyser::ResolvedAnalysisInput::new(
                input.analyser_profile(),
                foreign_profile,
                input.context_registry(),
                input.lexer_config(),
            ),
        ] {
            assert!(std::sync::Arc::ptr_eq(
                &input.context_registry(),
                &changed.context_registry()
            ));
            assert_eq!(input.lexer_config(), changed.lexer_config());
            assert!(!changed.has_logical_source_name_context());
            assert!(!changed.has_hosted_source_name_context());
            assert!(
                bindings
                    .original_native_source_declarations(&changed)
                    .is_none()
            );
        }
    }

    #[test]
    fn native_source_children_keep_actual_schema_and_open_dependency_owners() {
        let source = "proc f {x} {set value $x; return $value}\nproc caller {x} {not-declared $x; return $x}\n";
        let actual = module(source, "tcl8.6");
        let input = actual.source_metadata_input.as_ref().unwrap();
        let registry = input.borrowed_context_registry().commands();
        assert_eq!(actual.original_native_declaration_body_units.len(), 2);
        let (id, card) = actual
            .original_native_declaration_body_units
            .iter()
            .find(|(_, card)| {
                card.body_text()
                    .is_some_and(|body| body.starts_with("set value"))
            })
            .unwrap();
        let procedure = actual.body_units.get(id).unwrap();
        assert!(card.matches_procedure(&actual, procedure));
        assert!(matches!(
            procedure.body.statements[0],
            crate::ir::Statement::AssignValue { .. }
        ));
        assert!(matches!(
            procedure.body.statements[1],
            crate::ir::Statement::Return { .. }
        ));
        for statement in &procedure.body.statements {
            let tokens = statement.tokens().unwrap();
            let words = tokens
                .source_binding
                .as_ref()
                .unwrap()
                .original_native_source_words(tokens, input, registry)
                .unwrap();
            assert!(matches!(words.source(), crate::registry_invocation::source_structure::OriginalRegistrySource::SourceTransitions(_)));
            let crate::registry_invocation::source_structure::OriginalRegistrySource::SourceTransitions(advice) = words.source() else {
                unreachable!("asserted original source transition owner");
            };
            assert_eq!(
                advice
                    .original_procedure_source_body()
                    .map(|body| body.content_span()),
                card.body_span(),
            );
            assert!(
                crate::registry_invocation::admitted_native_compiler_invocation(
                    registry, None, tokens
                )
                .is_none()
            );
            let mut changed = tokens.clone();
            changed.argv_texts[0] = "foreign".into();
            assert!(
                changed
                    .source_binding
                    .as_ref()
                    .unwrap()
                    .original_native_source_words(&changed, input, registry)
                    .is_none()
            );
        }
        let caller = actual
            .original_native_declaration_body_units
            .iter()
            .find(|(_, card)| {
                card.body_text()
                    .is_some_and(|body| body.starts_with("not-declared"))
            })
            .map(|(id, _)| actual.body_units.get(id).unwrap())
            .unwrap();
        let tokens = caller.body.statements[0].tokens().unwrap();
        let dependencies =
            crate::registry_invocation::original_procedure_may_dependencies_for_word_for_module(
                tokens,
                "not-declared",
                &actual,
                registry,
            )
            .unwrap();
        assert!(dependencies.unknown_alternatives);
        assert!(
            crate::registry_invocation::original_procedure_may_dependencies_for_word_for_module(
                tokens,
                "forged-head",
                &actual,
                registry
            )
            .is_none()
        );
        assert_eq!(dependencies.bodies.len(), 2);
        assert!(
            dependencies
                .bodies
                .iter()
                .all(|body| actual.procedure_ids.owns(*body))
        );
        let foreign = module(source, "tcl8.6");
        assert!(
            !tokens
                .source_binding
                .as_ref()
                .unwrap()
                .original_native_source_may_owner(tokens, "not-declared", &foreign, registry)
        );
        assert!(procedure.body.executed_source.is_none());
        assert!(procedure.body.native_compilation_admission.is_none());
        let foreign_same_input = crate::lowering::lower_to_ir_with(
            crate::lowering::Lowerer::with_config(registry, input.lexer_config())
                .with_resolved_analysis_input(input.clone())
                .with_explicit_unknown_entry(),
            source,
        );
        let foreign_bindings = foreign_same_input
            .retained_source_bindings
            .as_ref()
            .unwrap()
            .positioned_for_module(&foreign_same_input, registry)
            .unwrap();
        assert!(card.source_children(&foreign_bindings).is_none());
    }

    #[test]
    fn native_source_children_refuse_original_expansion_cardinality() {
        for source in ["proc f {} {set {*}{x 1}}", "proc f {args} {set {*}$args}"] {
            let actual = module(source, "tcl8.6");
            let input = actual.source_metadata_input.as_ref().unwrap();
            let registry = input.borrowed_context_registry().commands();
            let (&id, _) = actual
                .original_native_declaration_body_units
                .iter()
                .next()
                .unwrap();
            let procedure = actual.body_units.get(&id).unwrap();
            let tokens = procedure.body.statements[0].tokens().unwrap();
            let metadata = crate::registry_invocation::InvocationMetadataContext::for_source_input(
                registry,
                input,
                input.lexer_config(),
                Some(input.unit_profile()),
            )
            .unwrap();
            assert!(
                crate::registry_invocation::original_native_source_structured_invocation(
                    registry, metadata, tokens
                )
                .is_none()
            );
            assert!(!matches!(
                procedure.body.statements[0],
                crate::ir::Statement::AssignConst { .. } | crate::ir::Statement::AssignValue { .. }
            ));
        }
    }

    #[test]
    fn original_native_source_cards_require_complete_original_header_grammar() {
        for source in [
            "proc $name {} {return VALUE}",
            "proc f {a {b C D}} {return VALUE}",
            "proc f {a} $body",
        ] {
            let module = module(source, "tcl8.6");
            assert!(
                module.original_native_declaration_body_units.is_empty(),
                "{source}"
            );
        }
        let context =
            tcl_registry::model::ingress::resolve_environment("tcl8.6").default_context_registry();
        let config =
            tcl_lexer::LexerConfig::for_file_grammar(context.commands().profile().unwrap().grammar);
        let missing = crate::lowering::lower_to_ir_with(
            crate::lowering::Lowerer::with_config(context.commands(), config),
            "proc f {} {return VALUE}",
        );
        assert!(missing.original_native_declaration_body_units.is_empty());
        let logical = module("proc f {} {return VALUE}", "tcl");
        assert!(logical.original_native_declaration_body_units.is_empty());
    }
}
