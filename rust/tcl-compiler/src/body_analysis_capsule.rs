// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Retained compilation owners for per-body analysis and source edits.

use std::hash::{Hash, Hasher};
use std::sync::Arc;

use crate::compilation_unit::{CompilationUnit, FunctionUnit};
use crate::ir::{Procedure, ProcedureId};
use crate::optimiser::Optimisation;
use tcl_registry::CommandRegistry;

/// One actual body and its complete immutable compilation inputs.
#[derive(Debug, Clone)]
pub struct BodyAnalysisCapsule {
    unit: Arc<CompilationUnit>,
    body: ProcedureId,
    neutral: Option<Arc<NativeClosedBodyProjection>>,
}

#[derive(Debug, Clone, PartialEq)]
struct NativeClosedBodyProjection {
    declaration: NeutralDeclaration,
    body: crate::ir::Script,
    function: FunctionUnit,
    summary: Option<crate::interprocedural::ProcSummary>,
    instance_classes: Option<crate::taint::InstanceClassState>,
    tainted_global_writes: Option<std::collections::HashSet<String>>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct NeutralDeclaration {
    publication: crate::signature_scan::scope::SignatureSourceCommand,
    namespace: crate::command_binding::SourceNamespaceKey,
    written_name: String,
    display: String,
    parameters: Vec<String>,
    parameters_text: String,
    body_text: String,
    kind: crate::ir::ProcedureBodyKind,
}

impl BodyAnalysisCapsule {
    /// Retain a source-corresponding body without deriving an identity from its name.
    #[must_use]
    pub fn retain(
        unit: Arc<CompilationUnit>,
        body: ProcedureId,
        registry: &CommandRegistry,
    ) -> Option<Self> {
        let module = &unit.ir_module;
        let procedure = module.procedures.get(&body)?;
        let function = unit.procedures.get(&body)?;
        let retained = module.retained_source_bindings.as_deref()?;
        retained.positioned_for_module(module, registry)?;
        if function.body_id != Some(body)
            || unit.cfg_module.procedures.get(&body) != Some(&function.cfg)
            || module.source.try_text().ok()? != unit.source
            || procedure.declaration_site.is_none()
            || !retained.matches_original_procedure(procedure)
        {
            return None;
        }
        let neutral = native_closed_projection(&unit, body, registry).map(Arc::new);
        Some(Self {
            unit,
            body,
            neutral,
        })
    }

    /// The complete retained compilation unit, including original world and input owners.
    #[must_use]
    pub fn unit(&self) -> &CompilationUnit {
        &self.unit
    }

    /// The Module-issued body coordinate retained by this capsule.
    #[must_use]
    pub const fn body_id(&self) -> ProcedureId {
        self.body
    }

    /// Whether original Native closure and injective source relocation certify reuse.
    /// The certificate supplies analysis data equivalence, never Native execution.
    #[must_use]
    pub const fn has_closed_native_correspondence(&self) -> bool {
        self.neutral.is_some()
    }

    /// The exact retained source declaration.
    #[must_use]
    pub fn procedure(&self) -> &Procedure {
        self.unit
            .ir_module
            .procedures
            .get(&self.body)
            .expect("retained capsule declaration")
    }

    /// The actual analysis belonging to the retained declaration.
    #[must_use]
    pub fn function(&self) -> &FunctionUnit {
        self.unit
            .procedures
            .get(&self.body)
            .expect("retained capsule function")
    }

    /// Select body-local source edits from the genuine whole-unit analysis.
    /// A group crossing the body's source boundary is retained by neither memo.
    #[must_use]
    pub fn original_body_optimisations(
        &self,
        registry: &CommandRegistry,
        dialect: Option<&'static tcl_dialect::DialectProfile>,
    ) -> Vec<Optimisation> {
        let span = self.procedure().span;
        let raw = crate::optimiser::optimise_unit_raw(self.unit(), registry, dialect);
        let outside_groups: std::collections::HashSet<_> = raw
            .iter()
            .filter(|edit| edit.span.start() < span.start() || span.end() < edit.span.end())
            .filter_map(|edit| edit.group)
            .collect();
        raw.into_iter()
            .filter(|edit| {
                span.start() <= edit.span.start()
                    && edit.span.end() <= span.end()
                    && edit
                        .group
                        .is_none_or(|group| !outside_groups.contains(&group))
            })
            .map(|mut edit| {
                edit.span = tcl_lexer::Span::new(
                    edit.span.start() - span.start(),
                    edit.span.end() - span.start(),
                );
                edit
            })
            .collect()
    }
}

impl PartialEq for BodyAnalysisCapsule {
    fn eq(&self, other: &Self) -> bool {
        match (&self.neutral, &other.neutral) {
            (Some(left), Some(right)) => {
                left.declaration == right.declaration
                    && left.body == right.body
                    && ((self.body == other.body && Arc::ptr_eq(&self.unit, &other.unit))
                        || left == right)
            }
            (None, None) => self.body == other.body && Arc::ptr_eq(&self.unit, &other.unit),
            _ => false,
        }
    }
}

impl Eq for BodyAnalysisCapsule {}

impl Hash for BodyAnalysisCapsule {
    fn hash<H: Hasher>(&self, state: &mut H) {
        if let Some(projection) = &self.neutral {
            projection.declaration.hash(state);
            projection.body.hash(state);
        } else {
            self.body.hash(state);
            Arc::as_ptr(&self.unit).hash(state);
        }
    }
}

fn native_closed_projection(
    unit: &CompilationUnit,
    body_id: ProcedureId,
    registry: &CommandRegistry,
) -> Option<NativeClosedBodyProjection> {
    let module = &unit.ir_module;
    let procedure = module.procedures.get(&body_id)?;
    let publication = procedure.publication.as_ref()?;
    if module
        .procedures
        .values()
        .filter(|candidate| candidate.publication.as_ref() == Some(publication))
        .count()
        != 1
    {
        return None;
    }
    let declaration = NeutralDeclaration {
        publication: publication.clone(),
        namespace: procedure.body.namespace_context.as_deref()?.clone(),
        written_name: procedure.name.clone(),
        display: procedure.qualified_name.clone(),
        parameters: procedure.params.clone(),
        parameters_text: procedure.params_raw.clone(),
        body_text: procedure.body_source.clone()?,
        kind: procedure.body_kind,
    };
    let mut original = procedure.body.clone();
    crate::lattice_rebase::rebase_script(&mut original, -i64::from(procedure.span.start()));
    let bindings = crate::command_binding::ModuleCommandBindings::analyse(module, registry);
    let function = unit.procedures.get(&body_id)?;
    let scope = crate::command_binding::BodyProofScope {
        source: &unit.source,
        body_source: &declaration.body_text,
        original_body_offset: procedure.span.start(),
        executable_body_offset: procedure.body_offset,
        metadata: crate::registry_invocation::InvocationMetadataInput::SuppliedSource(
            function.source_metadata_input(),
        ),
        config: function.source_lexer_config(),
    };
    let scope = if module.source_entry.metadata_context.is_standalone() {
        crate::command_binding::BodyProofScope {
            metadata: crate::registry_invocation::InvocationMetadataInput::Standalone,
            ..scope
        }
    } else {
        scope
    };
    let template = bindings.prepare_native_body_template(&original, scope, registry)?;
    let mut normalized = function.clone();
    crate::lattice_rebase::rebase_function_unit(
        &mut normalized,
        -i64::from(procedure.span.start()),
    );
    let mut normalized = normalized
        .relocated_variable_proofs(&template.variable_relocation)?
        .restored_source_proofs(&crate::command_binding::BodySourceProofs::from_body(
            &template.body,
        ))?;
    // The checked original declaration and injective body relocation own this
    // comparison. The private projection is never used as an executable unit.
    normalized.body_id = None;
    let mut summary = unit
        .interproc
        .as_ref()
        .and_then(|analysis| analysis.procedures.get(&body_id))
        .cloned();
    if let Some(summary) = &mut summary {
        if !summary.calls.is_empty()
            || !summary.direct_calls.is_empty()
            || !summary.defined_callees.is_empty()
        {
            return None;
        }
        if let Some(defined_at) = summary.defined_at {
            summary.defined_at =
                Some(defined_at.checked_sub(procedure.declaration_site.as_ref()?.offset)?);
        }
    }
    Some(NativeClosedBodyProjection {
        declaration,
        body: template.body,
        function: normalized,
        summary,
        instance_classes: unit
            .interproc
            .as_ref()
            .and_then(|analysis| analysis.global_instance_classes.get(&body_id))
            .cloned(),
        tainted_global_writes: unit
            .interproc
            .as_ref()
            .and_then(|analysis| analysis.tainted_global_writes.get(&body_id))
            .cloned(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn unit(
        source: &str,
    ) -> (
        Arc<tcl_registry::model::ContextRegistry>,
        Arc<CompilationUnit>,
    ) {
        let (context, unit) = crate::interprocedural::logical_completion_unit(source, "tcl8.6");
        (context, Arc::new(unit))
    }

    #[test]
    fn source_body_capsule_keeps_the_actual_module_and_function() {
        let (context, unit) = unit("proc p {argument} {return $argument}");
        let body = *unit.ir_module.procedures.keys().next().unwrap();
        let capsule = BodyAnalysisCapsule::retain(Arc::clone(&unit), body, context.commands())
            .expect("original declaration, actual module and actual body");
        assert!(std::ptr::eq(capsule.unit(), unit.as_ref()));
        assert!(std::ptr::eq(capsule.function(), &unit.procedures[&body]));
        assert_eq!(capsule.body_id(), body);
        assert_eq!(capsule, capsule.clone());
    }

    #[test]
    fn equal_source_and_display_do_not_replace_the_module_owner() {
        let source = "proc p {} {return VALUE}";
        let (context, left) = unit(source);
        let (_, right) = unit(source);
        let left_id = *left.ir_module.procedures.keys().next().unwrap();
        let right_id = *right.ir_module.procedures.keys().next().unwrap();
        assert_eq!(left_id.ordinal(), right_id.ordinal());
        assert_ne!(left_id, right_id);
        assert!(
            BodyAnalysisCapsule::retain(Arc::clone(&left), right_id, context.commands()).is_none()
        );
        assert_ne!(
            BodyAnalysisCapsule::retain(left, left_id, context.commands()).unwrap(),
            BodyAnalysisCapsule::retain(right, right_id, context.commands()).unwrap(),
        );
    }

    #[test]
    fn changed_source_registry_or_body_owner_withdraws_the_capsule() {
        let source = "proc p {} {return VALUE}";
        let (context, actual) = unit(source);
        let body = *actual.ir_module.procedures.keys().next().unwrap();
        let mut changed_source = (*actual).clone();
        changed_source.source = "proc p {} {return OTHER}".to_owned();
        assert!(
            BodyAnalysisCapsule::retain(Arc::new(changed_source), body, context.commands())
                .is_none()
        );
        let mut changed_function = (*actual).clone();
        changed_function.procedures.get_mut(&body).unwrap().body_id = None;
        assert!(
            BodyAnalysisCapsule::retain(Arc::new(changed_function), body, context.commands())
                .is_none()
        );
        let foreign = CommandRegistry::build_default();
        assert!(BodyAnalysisCapsule::retain(actual, body, &foreign).is_none());
    }
}
