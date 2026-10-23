// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Analysis coverage for exact installed and original declaration bodies.

use super::{IrulesExecutionContext, Lowerer, Procedure};

impl Lowerer<'_> {
    pub(super) fn retain_installed_procedure_body_units(&mut self) {
        let (installed_bodies, declaration_bodies) =
            self.module_source_bindings.as_ref().map_or_else(
                || (Vec::new(), Vec::new()),
                |bindings| {
                    (
                        bindings.installed_procedure_body_units(),
                        bindings.original_declaration_body_units(),
                    )
                },
            );
        let logical_cards = self
            .module_source_bindings
            .as_ref()
            .and_then(|bindings| bindings.original_logical_procedure_body_cards())
            .unwrap_or_default();
        let native_cards = self
            .module_source_bindings
            .as_ref()
            .zip(self.module.source_metadata_input.as_ref())
            .map_or_else(Vec::new, |(bindings, input)| {
                bindings.original_native_procedure_body_cards(input, self.registry)
            });
        if installed_bodies.is_empty()
            && declaration_bodies.is_empty()
            && logical_cards.is_empty()
            && native_cards.is_empty()
        {
            return;
        }
        let named: Vec<_> = self
            .module
            .procedures
            .values()
            .filter_map(|procedure| {
                Some((
                    procedure.body.executed_source.as_deref()?.clone(),
                    procedure.body.namespace_context.as_deref()?.clone(),
                    procedure.span.start(),
                    canonical_parameter_list(&procedure.params_raw),
                ))
            })
            .collect();
        let mut declared = std::collections::HashSet::new();
        let mut retained_per_site = std::collections::BTreeMap::new();
        let retained = self.module_source_bindings.take();
        let enclosing = self.exchange_boxed_source_bindings(retained);
        let previous = std::mem::replace(&mut self.suppress_proc_register, true);
        for (installed, original_declaration) in installed_bodies
            .into_iter()
            .map(|body| (body, false))
            .chain(declaration_bodies.into_iter().map(|body| (body, true)))
        {
            let context = (
                installed.allocation.clone(),
                installed.source.clone(),
                installed.namespace_key.clone(),
            );
            let parameters =
                tcl_syntax::list::join_list(installed.parameters.iter().map(|parameter| {
                    tcl_syntax::list::join_list(
                        std::iter::once(parameter.name.as_str())
                            .chain(parameter.default.as_deref()),
                    )
                }));
            let already_named = installed.allocation.incarnation
                == crate::command_binding::AllocationIncarnation::First
                && installed.allocation.site.source == installed.source.origin
                && named.iter().any(|(source, namespace, offset, params)| {
                    source == &installed.source
                        && namespace == &installed.namespace_key
                        && *offset == installed.allocation.site.offset
                        && params == &parameters
                });
            if already_named || declared.contains(&context) {
                continue;
            }
            let count = retained_per_site
                .entry(installed.allocation.site.clone())
                .or_insert(0usize);
            if *count >= crate::specialise_factories::DEFAULT_FACTORY_CAP {
                continue;
            }
            *count += 1;
            declared.insert(context);
            self.retain_installed_body(installed, original_declaration);
        }
        self.retain_original_logical_bodies(logical_cards);
        self.retain_original_native_bodies(native_cards);
        self.suppress_proc_register = previous;
        self.module_source_bindings = self.exchange_boxed_source_bindings(enclosing);
    }

    fn retain_original_logical_bodies(
        &mut self,
        cards: Vec<std::sync::Arc<crate::command_binding::OriginalLogicalProcedureBody>>,
    ) {
        for card in cards {
            let already_retained = self
                .module
                .procedures
                .values()
                .chain(self.module.body_units.values())
                .any(|procedure| {
                    procedure.declaration_site.as_ref() == Some(card.site())
                        && procedure.body_offset == card.scope().scope_body().content_span().start()
                        && procedure.params_raw == card.parameters_text()
                });
            if !already_retained {
                self.retain_original_logical_body(&card);
            }
        }
    }

    fn retain_original_native_bodies(
        &mut self,
        cards: Vec<std::sync::Arc<crate::command_binding::OriginalNativeProcedureBody>>,
    ) {
        for card in cards {
            let already_retained =
                self.module
                    .original_source_procedure_bodies()
                    .any(|(_, procedure)| {
                        procedure.declaration_site.as_ref() == Some(card.site())
                            && card
                                .body_span()
                                .is_some_and(|span| procedure.body_offset == span.start())
                            && card.parameters_text() == Some(procedure.params_raw.as_str())
                    });
            if !already_retained {
                self.retain_original_native_body(&card);
            }
        }
    }

    fn retain_original_native_body(
        &mut self,
        card: &std::sync::Arc<crate::command_binding::OriginalNativeProcedureBody>,
    ) {
        let Some(input) = self.module.source_metadata_input.as_ref() else {
            return;
        };
        if !card.matches_source(&self.module.source, input, self.registry) {
            return;
        }
        let (
            Some(span),
            Some(text),
            Some(name),
            Some(label),
            Some(parameters_text),
            Some(namespace),
        ) = (
            card.body_span(),
            card.body_text().map(str::to_owned),
            card.name().map(str::to_owned),
            card.reporting_label(),
            card.parameters_text().map(str::to_owned),
            card.namespace().display(),
        )
        else {
            return;
        };
        let children = self
            .source_bindings
            .as_ref()
            .and_then(|bindings| card.source_children(bindings));
        let previous_children = std::mem::replace(&mut self.native_source_children, children);
        let previous = std::mem::replace(
            &mut self.readonly_native_declaration_body,
            crate::lowering::SourceBodyMode::ReadOnly,
        );
        let mut body =
            self.in_procedure_frame(Some(IrulesExecutionContext::ProcedureBody), |lowerer| {
                lowerer.lower_original_source_body(
                    &text,
                    span.start(),
                    &namespace,
                    super::source_body_syntax::SourceBodySyntaxOwner::Native(
                        std::sync::Arc::clone(card),
                    ),
                )
            });
        self.readonly_native_declaration_body = previous;
        self.native_source_children = previous_children;
        body.namespace_context = Some(Box::new(card.namespace().clone()));
        let card = std::sync::Arc::new(
            card.as_ref()
                .clone()
                .retain_lowered_body(&body, &self.module.source_entry),
        );
        let id = self.module.retain_body_unit(Procedure {
            name,
            qualified_name: label,
            publication: None,
            declaration_site: Some(card.site().clone()),
            body_kind: crate::ir::ProcedureBodyKind::OriginalDeclaration,
            params: card.parameters().iter().map(|p| p.name.clone()).collect(),
            span: tcl_lexer::Span::new(card.site().offset, span.end()),
            body,
            params_raw: parameters_text,
            body_source: Some(text),
            body_offset: span.start(),
            namespace_scoped: false,
            base_priority: 500,
        });
        self.module
            .original_native_declaration_body_units
            .insert(id, card);
    }

    fn retain_original_logical_body(
        &mut self,
        card: &std::sync::Arc<crate::command_binding::OriginalLogicalProcedureBody>,
    ) {
        let Some(input) = self.module.source_metadata_input.as_ref() else {
            return;
        };
        let image = card.site().source.source_image();
        if !card.matches_source(image, input) || image.bytes() != self.source.as_bytes() {
            return;
        }
        let Some(namespace) =
            tcl_syntax::naming::checked_namespace_path_key(card.scope().namespace_recipe())
        else {
            return;
        };
        let Some(name) = card.name().map(str::to_owned) else {
            return;
        };
        let span = card.scope().scope_body().content_span();
        let Some(text) = image
            .try_text()
            .ok()
            .and_then(|s| s.get(span.as_range()))
            .map(str::to_owned)
        else {
            return;
        };
        let previous = std::mem::replace(
            &mut self.readonly_logical_body,
            crate::lowering::SourceBodyMode::ReadOnly,
        );
        let mut body =
            self.in_procedure_frame(Some(IrulesExecutionContext::ProcedureBody), |lowerer| {
                lowerer.lower_original_source_body(
                    &text,
                    span.start(),
                    &namespace,
                    super::source_body_syntax::SourceBodySyntaxOwner::Logical(
                        std::sync::Arc::clone(card),
                    ),
                )
            });
        self.readonly_logical_body = previous;
        body.namespace_context = Some(Box::new(
            crate::command_binding::SourceNamespaceKey::authored(namespace),
        ));
        let card = std::sync::Arc::new(
            card.as_ref()
                .clone()
                .retain_module_entry(&self.module.source_entry),
        );
        let label = format!("::original-logical-body#{}", self.body_unit_count);
        self.body_unit_count += 1;
        let id = self.module.retain_body_unit(Procedure {
            name,
            qualified_name: label,
            publication: None,
            declaration_site: Some(card.site().clone()),
            body_kind: crate::ir::ProcedureBodyKind::OriginalDeclaration,
            params: card.parameters().iter().map(|p| p.name.clone()).collect(),
            span,
            body,
            params_raw: card.parameters_text().to_owned(),
            body_source: Some(text),
            body_offset: span.start(),
            namespace_scoped: false,
            base_priority: 500,
        });
        self.module
            .original_logical_declaration_body_units
            .insert(id, card);
    }

    fn retain_installed_body(
        &mut self,
        installed: crate::command_binding::SourceInstalledProcedureBody,
        original_declaration: bool,
    ) {
        let body =
            self.in_procedure_frame(Some(IrulesExecutionContext::ProcedureBody), |lowerer| {
                lowerer.lower_executed_script_in_context(
                    installed.source.clone(),
                    &installed.namespace_key,
                )
            });
        let params_raw =
            tcl_syntax::list::join_list(installed.parameters.iter().map(|parameter| {
                tcl_syntax::list::join_list(
                    std::iter::once(parameter.name.as_str()).chain(parameter.default.as_deref()),
                )
            }));
        let label = if original_declaration {
            format!("::original-declaration-body#{}", self.body_unit_count)
        } else {
            tcl_syntax::naming::qualify(
                &installed.namespace,
                &format!("installed-body#{}", self.body_unit_count),
            )
        };
        self.body_unit_count += 1;
        let start = installed.source.base();
        let end =
            start.saturating_add(u32::try_from(installed.source.text.len()).unwrap_or(u32::MAX));
        let id = self.module.retain_body_unit(Procedure {
            name: installed.command,
            qualified_name: label.clone(),
            publication: None,
            declaration_site: Some(installed.allocation.site.clone()),
            body_kind: if original_declaration {
                crate::ir::ProcedureBodyKind::OriginalDeclaration
            } else {
                crate::ir::ProcedureBodyKind::Installed
            },
            params: installed
                .parameters
                .into_iter()
                .map(|parameter| parameter.name)
                .collect(),
            span: tcl_lexer::Span::new(start, end),
            body,
            params_raw,
            body_source: installed.source.try_text().ok().map(str::to_owned),
            body_offset: start,
            namespace_scoped: false,
            base_priority: 500,
        });
        if original_declaration {
            self.module
                .original_declaration_body_units
                .insert(id, installed.allocation);
        } else {
            self.module
                .installed_procedure_body_units
                .insert(id, installed.allocation);
        }
    }
}

/// The written formal list in the canonical form an installed body carries:
/// each formal re-joined from its list elements, so `{"as" v}` and its
/// installed `as v` name the same formals. Unparsable text keeps its spelling.
fn canonical_parameter_list(raw: &str) -> String {
    let Ok(formals) = tcl_syntax::list::split_list(raw) else {
        return raw.to_owned();
    };
    let mut canonical = Vec::with_capacity(formals.len());
    for formal in &formals {
        let Ok(fields) = tcl_syntax::list::split_list(formal) else {
            return raw.to_owned();
        };
        canonical.push(tcl_syntax::list::join_list(&fields));
    }
    tcl_syntax::list::join_list(canonical.iter().map(String::as_str))
}
