// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program.  If not, see <https://www.gnu.org/licenses/>.
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Bodies walked as callers beyond the procedures: `uplevel` bodies, `TclOO` methods and other executable scripts.

use super::ExtraCallSiteScanContext;
use super::unit_linkage::walk_module_scripts;
use crate::ir::Module as IrModule;
use tcl_registry::CommandRegistry;

/// Whether [`build_extra_call_site_scan_contexts`] has anything to build for
/// this module — and therefore whether its
/// [`crate::cfg_builder::prepare_cfg_context`] must be computed.
///
/// A single predicate so the two builders that gate on it
/// ([`crate::compilation_unit::CompilationUnit`]'s build and
/// [`scan_source_call_sites`]) cannot drift from what the context builder
/// actually consumes.
pub(crate) fn needs_extra_call_site_scan_contexts(ir_module: &IrModule) -> bool {
    if !ir_module.methods.is_empty() || !ir_module.body_units.is_empty() {
        return true;
    }
    // Only a module with neither pays for the statement walk, and it costs
    // no allocation — a bare `uplevel {…}` is rare enough that building the
    // caller list here just to test it for emptiness would be wasteful.
    let mut found = false;
    walk_module_scripts(ir_module, &mut |_, stmt| {
        found |= matches!(stmt, crate::ir::Statement::UpFrame { .. });
    });
    found
}

/// The synthetic caller name prefix an `uplevel` body's bare CFG carries:
/// unique per occurrence, so its own variable-scope facts never clobber
/// another scope's.
const UPFRAME_SCOPE_PREFIX: &str = "@upframe@";

/// One `uplevel ?level? { … }` body the call-site scan visits as a caller.
pub(super) struct UpFrameCaller<'a> {
    /// Actual retained frame namespace; unknown callers remain unavailable.
    pub(super) execution_namespace: crate::ir::ExecutionNamespace,
    /// Synthetic, occurrence-unique variable-scope identity for its CFG.
    pub(super) scope: String,
    /// The lowered body.
    pub(super) body: &'a crate::ir::Script,
}

/// Collect each literal upframe body with its independently retained parent namespace.
pub(super) fn upframe_scan_bodies(ir_module: &IrModule) -> Vec<UpFrameCaller<'_>> {
    let mut out = Vec::new();
    collect_upframes(
        &ir_module.top_level,
        &ir_module
            .top_level
            .execution_namespace(crate::ir::ExecutionNamespace::RuntimeSelected),
        ir_module.resolved_registry(),
        &mut out,
    );
    for procedure in ir_module
        .procedures
        .values()
        .chain(ir_module.body_lowerings().map(|(_, procedure)| procedure))
    {
        collect_upframes(
            &procedure.body,
            &procedure
                .body
                .execution_namespace(crate::ir::ExecutionNamespace::RuntimeSelected),
            ir_module.resolved_registry(),
            &mut out,
        );
    }
    for method in ir_module.methods.values() {
        collect_upframes(
            &method.body,
            &method.execution_namespace,
            ir_module.resolved_registry(),
            &mut out,
        );
    }
    out
}

pub(super) fn collect_upframes<'a>(
    script: &'a crate::ir::Script,
    parent_namespace: &crate::ir::ExecutionNamespace,
    registry: &CommandRegistry,
    out: &mut Vec<UpFrameCaller<'a>>,
) {
    for statement in &script.statements {
        if let crate::ir::Statement::UpFrame {
            frame_shift,
            absolute,
            body,
            span,
            tokens,
        } = statement
        {
            let execution_namespace = if *frame_shift != 0 {
                crate::ir::ExecutionNamespace::RuntimeSelected
            } else if *absolute {
                tokens
                    .as_ref()
                    .and_then(|tokens| {
                        tokens
                            .source_binding
                            .as_ref()?
                            .original_native_root_namespace_context(tokens, registry)
                    })
                    .map_or(
                        crate::ir::ExecutionNamespace::RuntimeSelected,
                        crate::ir::ExecutionNamespace::SourceContext,
                    )
            } else {
                parent_namespace.clone()
            };
            out.push(UpFrameCaller {
                execution_namespace: execution_namespace.clone(),
                scope: format!("{UPFRAME_SCOPE_PREFIX}{}", span.start()),
                body,
            });
            collect_upframes(body, &execution_namespace, registry, out);
        } else {
            for child in statement.child_scripts() {
                collect_upframes(child, parent_namespace, registry, out);
            }
        }
    }
}

/// Build bare CFGs (no further per-function analysis) for every `TclOO` method,
/// synthetic body unit (`apply` lambda, `namespace eval` body), and static
/// `uplevel` body, so [`collect_call_site_constants`] can walk them as
/// *callers* too.
///
/// Neither is itself ever seeded with `param_constants`
/// (`build_method_units` / `build_body_units` always pass `None` for their
/// own analysis), but a call *from* one of their bodies *to* an ordinary
/// user proc is a real call site whose argument can vary between call
/// sites, exactly like a bare top-level or proc-body call — and invisible to
/// a collector that walks only `cfg_module.top_level` and
/// `cfg_module.procedures`. That is a real, varying call site silently
/// missing from the "every caller agrees" evidence, reached through a
/// method/lambda body instead
/// of namespace-blind recursion or a `catch`/`uplevel` body.
///
/// Returns an empty `Vec` (no cost beyond
/// [`needs_extra_call_site_scan_contexts`]) when the module has none of the
/// three — the overwhelmingly common case — or when `cfg_context` is `None`
/// (all three require it; the builders compute it exactly when
/// [`needs_extra_call_site_scan_contexts`] says so).
pub(crate) fn build_extra_call_site_scan_contexts(
    ir_module: &IrModule,
    cfg_context: Option<&crate::cfg_builder::PreparedCfgContext>,
    registry: &CommandRegistry,
    config: tcl_lexer::LexerConfig,
) -> Vec<ExtraCallSiteScanContext> {
    let upframes = upframe_scan_bodies(ir_module);
    if ir_module.methods.is_empty() && ir_module.body_units.is_empty() && upframes.is_empty() {
        return Vec::new();
    }
    let Some(cfg_context) = cfg_context else {
        return Vec::new();
    };
    let build = |qname: &str, body: &crate::ir::Script| {
        crate::cfg_builder::build_cfg_function_with_prepared_context(
            qname,
            body,
            true,
            registry,
            ir_module.plain_command_dispatch,
            cfg_context,
            config,
        )
    };
    let dispatch_barrier = crate::optimiser::method_barrier::compute(ir_module, registry);
    ir_module
        .methods
        .iter()
        .map(|(id, method)| {
            let mqname = method.reporting_label();
            let execution_namespace = method.execution_namespace.clone();
            let requires_runtime_command_namespace = matches!(
                &execution_namespace,
                crate::ir::ExecutionNamespace::RuntimeSelected
            )
                && crate::ir_helpers::requires_runtime_command_namespace(&method.body, registry);
            let cfg = crate::cfg_builder::build_cfg_method_function_with_prepared_context(
                &mqname,
                &method.body,
                execution_namespace.clone(),
                true,
                registry,
                ir_module.plain_command_dispatch,
                cfg_context,
                !dispatch_barrier.allows_locals(*id),
                config,
            );
            ExtraCallSiteScanContext {
                analysis_body: Some(*id),
                cfg,
                execution_namespace,
                requires_runtime_command_namespace,
            }
        })
        .chain(ir_module.body_lowerings().map(|(id, unit)| {
            let execution_namespace = unit
                .body
                .execution_namespace(crate::ir::ExecutionNamespace::RuntimeSelected);
            ExtraCallSiteScanContext {
                analysis_body: Some(*id),
                cfg: build(&unit.qualified_name, &unit.body),
                execution_namespace,
                requires_runtime_command_namespace: false,
            }
        }))
        .chain(upframes.into_iter().map(|up| {
            let requires_runtime_command_namespace = matches!(
                &up.execution_namespace,
                crate::ir::ExecutionNamespace::RuntimeSelected
            )
                && crate::ir_helpers::requires_runtime_command_namespace(up.body, registry);
            ExtraCallSiteScanContext {
                analysis_body: None,
                cfg: build(&up.scope, up.body),
                execution_namespace: up.execution_namespace,
                requires_runtime_command_namespace,
            }
        }))
        .collect()
}
