// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Standalone declaration analysis retains original source cards independently.

use super::{
    CommandTokens, IrulesExecutionContext, Lowerer, Procedure, SegmentedCommand, Statement,
};

impl Lowerer<'_> {
    pub(super) fn try_lower_authored_procedure_declaration(
        &mut self,
        seg: &SegmentedCommand,
        namespace: &str,
        tokens: &CommandTokens,
    ) -> Option<Statement> {
        if !self.allows_standalone_registry_metadata()
            || self.compilation_scope == tcl_runtime_api::SourceCompilationScope::EnteredSource
            || self.suppress_proc_register
        {
            return None;
        }
        let card =
            crate::registry_invocation::original_authored_procedure_body(tokens, self.registry)?;
        let span = card.body_span();
        let text = card
            .site()
            .source
            .source_image()
            .try_text()
            .ok()?
            .get(span.as_range())?
            .to_owned();
        let body_namespace = card.namespace().advisory_key()?;
        let scope = card.create_source_scope(&self.module);
        let previous_scope = std::mem::replace(&mut self.authored_source_scope, scope.clone());
        let previous = std::mem::replace(
            &mut self.readonly_authored_body,
            crate::lowering::SourceBodyMode::ReadOnly,
        );
        let mut body =
            self.in_procedure_frame(Some(IrulesExecutionContext::ProcedureBody), |lowerer| {
                lowerer.lower_original_source_body(
                    &text,
                    span.start(),
                    &body_namespace,
                    super::source_body_syntax::SourceBodySyntaxOwner::Authored(
                        std::sync::Arc::new(card.clone()),
                    ),
                )
            });
        self.readonly_authored_body = previous;
        self.authored_source_scope = previous_scope;
        body.namespace_context = Some(Box::new(card.namespace().clone()));
        let card =
            std::sync::Arc::new(card.retain_lowered_body(&body, &self.module, seg.span, scope));
        let label = format!("::original-authored-body#{}", self.body_unit_count);
        self.body_unit_count += 1;
        let mut procedure = Procedure {
            name: card.name().to_owned(),
            qualified_name: label,
            publication: None,
            declaration_site: Some(card.site().clone()),
            body_kind: crate::ir::ProcedureBodyKind::OriginalDeclaration,
            params: card.parameter_names().map(str::to_owned).collect(),
            params_raw: card.parameters_text().to_owned(),
            span: seg.span,
            body,
            body_source: Some(text),
            body_offset: span.start(),
            namespace_scoped: false,
            base_priority: 500,
        };
        // The issued body ID remains its identity. Preserve the authentic
        // source label for readonly analysis/reporting without publishing a
        // callable, selecting a Native frame or deriving ownership from text.
        if let Some(label) = card.reporting_label(&self.module, &procedure) {
            label.clone_into(&mut procedure.qualified_name);
        }
        let id = self.module.retain_body_unit(procedure);
        self.module
            .original_authored_declaration_body_units
            .insert(id, card);
        Some(self.lower_default(seg, namespace))
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn original_authored_reporting_labels_preserve_distinct_unpublished_body_ids() {
        let registry = tcl_registry::CommandRegistry::build_default();
        let module = crate::lowering::lower_to_ir(
            "proc ::p {} {return FIRST}; proc ::p {} {return SECOND}",
            &registry,
        );
        assert!(module.procedures.is_empty());
        assert!(module.original_native_declaration_body_units.is_empty());
        let bodies = module
            .original_source_procedure_bodies()
            .collect::<Vec<_>>();
        assert_eq!(bodies.len(), 2);
        assert_ne!(bodies[0].0, bodies[1].0);
        for (id, procedure) in bodies {
            assert!(module.procedure_ids.owns(*id));
            assert_eq!(procedure.qualified_name, "::p");
            assert_eq!(
                module.original_procedure_report_label(*id, &registry),
                Some("::p")
            );
            assert!(procedure.publication.is_none());
            assert_eq!(
                procedure.body_kind,
                crate::ir::ProcedureBodyKind::OriginalDeclaration
            );
            assert!(procedure.body.executed_source.is_none());
            assert!(procedure.body.native_compilation_admission.is_none());
        }
    }
}
