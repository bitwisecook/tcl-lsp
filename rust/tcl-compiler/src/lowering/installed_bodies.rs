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
        if installed_bodies.is_empty() && declaration_bodies.is_empty() {
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
                    procedure.params_raw.clone(),
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
        self.suppress_proc_register = previous;
        self.module_source_bindings = self.exchange_boxed_source_bindings(enclosing);
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
            format!(
                "{}::installed-body#{}",
                installed.namespace.trim_end_matches("::"),
                self.body_unit_count
            )
        };
        self.body_unit_count += 1;
        let start = installed.source.base();
        let end =
            start.saturating_add(u32::try_from(installed.source.text.len()).unwrap_or(u32::MAX));
        self.module.body_units.insert(
            label.clone(),
            Procedure {
                name: installed.command,
                qualified_name: label.clone(),
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
            },
        );
        if original_declaration {
            self.module
                .original_declaration_body_units
                .insert(label, installed.allocation);
        } else {
            self.module
                .installed_procedure_body_units
                .insert(label, installed.allocation);
        }
    }
}
