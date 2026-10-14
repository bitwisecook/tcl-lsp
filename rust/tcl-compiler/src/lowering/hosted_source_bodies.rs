// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Conditional hosted bodies keep their genuine declaration scope and issuer.

use super::{IrulesExecutionContext, Lowerer, Procedure};

impl Lowerer<'_> {
    pub(super) fn retain_hosted_procedure_body_units(&mut self) {
        let Some(input) = self.module.source_metadata_input.as_ref() else {
            return;
        };
        let cards = self
            .module_source_bindings
            .as_ref()
            .map_or_else(Vec::new, |bindings| {
                bindings.original_hosted_procedure_body_cards(input, self.registry)
            });
        if cards.is_empty() {
            return;
        }
        let retained = self.module_source_bindings.take();
        let enclosing = self.exchange_boxed_source_bindings(retained);
        let previous = std::mem::replace(&mut self.suppress_proc_register, true);
        for card in cards {
            let retained = self
                .module
                .original_hosted_declaration_body_units
                .values()
                .any(|previous| previous.as_ref() == card.as_ref());
            if !retained {
                self.retain_hosted_procedure_body(card);
            }
        }
        self.suppress_proc_register = previous;
        self.module_source_bindings = self.exchange_boxed_source_bindings(enclosing);
    }

    fn retain_hosted_procedure_body(
        &mut self,
        card: std::sync::Arc<crate::command_binding::OriginalHostedProcedureBody>,
    ) {
        let Some(input) = self.module.source_metadata_input.as_ref() else {
            return;
        };
        let image = card.site().source.source_image();
        if !card.matches_source(image, input, self.registry)
            || image.bytes() != self.source.as_bytes()
        {
            return;
        }
        let (Some(namespace), Some(name), Some(parameters), Some(parameters_text), Ok(text)) = (
            card.namespace().advisory_key(),
            card.name().map(str::to_owned),
            card.parameter_names(),
            card.parameters_text().map(str::to_owned),
            card.source().try_text(),
        ) else {
            return;
        };
        let text = text.to_owned();
        let previous = std::mem::replace(&mut self.readonly_hosted_body, true);
        let mut body = self
            .in_procedure_frame(Some(IrulesExecutionContext::ProcedureBody), |lowerer| {
                lowerer.lower_body(&text, card.source().base(), &namespace)
            });
        self.readonly_hosted_body = previous;
        body.namespace_context = Some(Box::new(card.namespace().clone()));
        let card = std::sync::Arc::new(
            card.as_ref()
                .clone()
                .retain_lowered_body(&body, &self.module.source_entry),
        );
        let label = format!("::original-hosted-body#{}", self.body_unit_count);
        self.body_unit_count += 1;
        let id = self.module.retain_body_unit(Procedure {
            name,
            qualified_name: label,
            publication: None,
            declaration_site: Some(card.site().clone()),
            body_kind: crate::ir::ProcedureBodyKind::OriginalDeclaration,
            params: parameters,
            span: card.span(),
            body,
            params_raw: parameters_text,
            body_source: Some(text),
            body_offset: card.source().base(),
            namespace_scoped: false,
            base_priority: 500,
        });
        self.module
            .original_hosted_declaration_body_units
            .insert(id, card);
    }
}
