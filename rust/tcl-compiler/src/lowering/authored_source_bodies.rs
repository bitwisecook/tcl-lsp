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
        let id = self.module.retain_body_unit(Procedure {
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
        });
        self.module
            .original_authored_declaration_body_units
            .insert(id, card);
        Some(self.lower_default(seg, namespace))
    }
}
