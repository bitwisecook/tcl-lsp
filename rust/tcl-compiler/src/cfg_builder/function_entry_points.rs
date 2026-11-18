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

//! The `build_cfg_function*` family: the CFG of one script body in a given scope.

use super::{CfgBuilder, CfgCommandClasses, CfgContext, PreparedCfgContext};
use crate::cfg::Function;
use crate::command_binding::ModuleCommandBindings;
use crate::ir::{Module, Script};
use std::collections::HashMap;
use std::sync::Arc;
use tcl_registry::CommandRegistry;

/// Build a CFG for a single script body against one resolved command surface.
#[must_use]
pub fn build_cfg_function(
    name: &str,
    script: &Script,
    inline_loops: bool,
    registry: &CommandRegistry,
    plain_command_dispatch: bool,
) -> Function {
    build_cfg_function_with_config(
        name,
        script,
        inline_loops,
        registry,
        plain_command_dispatch,
        tcl_lexer::LexerConfig::for_profile(registry.profile()),
    )
}

/// [`build_cfg_function`] with the exact lexer configuration used to lower
/// `script`.
#[must_use]
pub fn build_cfg_function_with_config(
    name: &str,
    script: &Script,
    inline_loops: bool,
    registry: &CommandRegistry,
    plain_command_dispatch: bool,
    config: tcl_lexer::LexerConfig,
) -> Function {
    let mut builder = CfgBuilder::new(inline_loops, registry)
        .with_lexer_config(config)
        .with_command_surface(plain_command_dispatch)
        .with_invocation_namespace(script.execution_namespace(
            crate::ir::ExecutionNamespace::exact(command_namespace(name)),
        ));
    builder.build_function(name, script)
}

/// Build a closed-language phase whose command scope is explicitly selected
/// by its caller. The generated function label grants no namespace identity;
/// source correspondence and target-language admission remain caller duties.
#[must_use]
pub fn build_cfg_function_in_runtime_selected_scope(
    name: &str,
    script: &Script,
    inline_loops: bool,
    registry: &CommandRegistry,
    plain_command_dispatch: bool,
    config: tcl_lexer::LexerConfig,
) -> Function {
    CfgBuilder::new(inline_loops, registry)
        .with_lexer_config(config)
        .with_command_surface(plain_command_dispatch)
        .with_invocation_namespace(crate::ir::ExecutionNamespace::RuntimeSelected)
        .build_function(name, script)
}

/// Build the exact certified closed BPF phase without inventing Tcl callee
/// effects. Original source/census and BPF target admission remain separate.
#[must_use]
pub fn build_cfg_function_in_bpf_selected_scope(
    name: &str,
    script: &Script,
    module: &Module,
    registry: &CommandRegistry,
    config: tcl_lexer::LexerConfig,
    effects: &crate::ir::SourceBpfClosedEffects,
) -> Option<Function> {
    effects
        .matches(module, registry, script, config)
        .then_some(())?;
    let mut builder = CfgBuilder::new(false, registry)
        .with_lexer_config(config)
        .with_invocation_namespace(
            script.execution_namespace(crate::ir::ExecutionNamespace::RuntimeSelected),
        );
    builder.bpf_closed_effects = Some(Arc::new(effects.clone()));
    Some(builder.build_function(name, script))
}

/// Build a secondary analysis body in its retained execution namespace.
/// Possible body regions and exceptional continuations remain analysis facts;
/// the synthetic unit label does not select a command lookup namespace.
pub(crate) fn build_analysis_body(
    name: &str,
    script: &Script,
    namespace: crate::ir::ExecutionNamespace,
    registry: &CommandRegistry,
    config: tcl_lexer::LexerConfig,
    metadata: Arc<tcl_registry::model::ContextRegistry>,
) -> Function {
    CfgBuilder::new_with_upvars_and_classes(
        true,
        HashMap::new(),
        HashMap::new(),
        HashMap::new(),
        ModuleCommandBindings::default(),
        registry,
        CfgCommandClasses::from_metadata(registry, Some(metadata)),
    )
    .with_faithful_exceptions()
    .with_lexer_config(config)
    .with_invocation_namespace(script.execution_namespace(namespace))
    .build_function(name, script)
}

/// [`build_analysis_body`] under a complete source input, so positioned
/// conditional source stores are retained beside the body's statements.
pub(crate) fn build_analysis_body_for_input(
    name: &str,
    script: &Script,
    namespace: crate::ir::ExecutionNamespace,
    registry: &CommandRegistry,
    config: tcl_lexer::LexerConfig,
    input: &crate::analyser::ResolvedAnalysisInput,
) -> Function {
    CfgBuilder::new_with_upvars_and_classes(
        true,
        HashMap::new(),
        HashMap::new(),
        HashMap::new(),
        ModuleCommandBindings::default(),
        registry,
        CfgCommandClasses::from_source_input(registry, Some(input)),
    )
    .with_faithful_exceptions()
    .with_lexer_config(config)
    .with_invocation_namespace(script.execution_namespace(namespace))
    .build_function(name, script)
}

/// Build a CFG for one body with the registry-owned module context prepared by
/// [`prepare_cfg_context`].
#[must_use]
pub fn build_cfg_function_with_upvars(
    name: &str,
    script: &Script,
    inline_loops: bool,
    registry: &CommandRegistry,
    plain_command_dispatch: bool,
    context: CfgContext,
) -> Function {
    build_cfg_function_with_upvars_and_config(
        name,
        script,
        inline_loops,
        registry,
        plain_command_dispatch,
        context,
        tcl_lexer::LexerConfig::for_profile(registry.profile()),
    )
}

/// Build a CFG for one body with both the registry-owned module context and the
/// exact dialect-aware lexer configuration used during lowering.
#[must_use]
pub fn build_cfg_function_with_upvars_and_config(
    name: &str,
    script: &Script,
    inline_loops: bool,
    registry: &CommandRegistry,
    plain_command_dispatch: bool,
    context: CfgContext,
    config: tcl_lexer::LexerConfig,
) -> Function {
    build_cfg_function_with_upvars_inner(
        name,
        script,
        crate::ir::ExecutionNamespace::exact(command_namespace(name)),
        inline_loops,
        registry,
        plain_command_dispatch,
        context,
        false,
        None,
        config,
    )
}

/// Build an ordinary body CFG using the module's already-prepared facts.
/// Missing or foreign original metadata refuses command assistance.
#[must_use]
pub fn build_cfg_function_with_prepared_context(
    name: &str,
    script: &Script,
    inline_loops: bool,
    registry: &CommandRegistry,
    plain_command_dispatch: bool,
    context: &PreparedCfgContext,
    config: tcl_lexer::LexerConfig,
) -> Function {
    build_cfg_function_with_upvars_inner(
        name,
        script,
        crate::ir::ExecutionNamespace::exact(command_namespace(name)),
        inline_loops,
        registry,
        plain_command_dispatch,
        context.context.clone(),
        false,
        Some(context.command_classes.clone()),
        config,
    )
}

/// Test seam for building one method with an explicitly supplied context.
#[cfg(test)]
#[allow(clippy::too_many_arguments)]
pub(super) fn build_cfg_method_function_with_upvars(
    name: &str,
    script: &Script,
    execution_namespace: crate::ir::ExecutionNamespace,
    inline_loops: bool,
    registry: &CommandRegistry,
    plain_command_dispatch: bool,
    context: CfgContext,
    widen_oo_dispatch: bool,
) -> Function {
    build_cfg_function_with_upvars_inner(
        name,
        script,
        execution_namespace,
        inline_loops,
        registry,
        plain_command_dispatch,
        context,
        widen_oo_dispatch,
        None,
        tcl_lexer::LexerConfig::for_profile(registry.profile()),
    )
}

/// Build a method CFG using the module's already-prepared facts.
#[allow(clippy::too_many_arguments)]
pub(crate) fn build_cfg_method_function_with_prepared_context(
    name: &str,
    script: &Script,
    execution_namespace: crate::ir::ExecutionNamespace,
    inline_loops: bool,
    registry: &CommandRegistry,
    plain_command_dispatch: bool,
    context: &PreparedCfgContext,
    widen_oo_dispatch: bool,
    config: tcl_lexer::LexerConfig,
) -> Function {
    build_cfg_function_with_upvars_inner(
        name,
        script,
        execution_namespace,
        inline_loops,
        registry,
        plain_command_dispatch,
        context.context.clone(),
        widen_oo_dispatch,
        Some(context.command_classes.clone()),
        config,
    )
}

#[allow(clippy::too_many_arguments)]
fn build_cfg_function_with_upvars_inner(
    name: &str,
    script: &Script,
    execution_namespace: crate::ir::ExecutionNamespace,
    inline_loops: bool,
    registry: &CommandRegistry,
    plain_command_dispatch: bool,
    context: CfgContext,
    widen_oo_dispatch: bool,
    command_classes: Option<CfgCommandClasses>,
    config: tcl_lexer::LexerConfig,
) -> Function {
    let (upvar_procs, proc_params, global_write_procs, command_bindings, procedure_summary_owners) =
        context;
    let command_classes =
        command_classes.unwrap_or_else(|| CfgCommandClasses::from_registry(registry));
    let execution_namespace = script.execution_namespace(execution_namespace);
    let timeline = command_bindings.source_binding_timeline_from_boundary(
        script,
        registry,
        &execution_namespace,
    );
    let mut builder = CfgBuilder::new_with_upvars_and_classes(
        inline_loops,
        upvar_procs,
        proc_params,
        global_write_procs,
        command_bindings,
        registry,
        command_classes,
    )
    .with_procedure_summary_owners(procedure_summary_owners)
    .with_faithful_exceptions()
    .with_lexer_config(config)
    .with_command_surface(plain_command_dispatch)
    .with_invocation_namespace(execution_namespace)
    .with_source_binding_timeline(timeline);
    if widen_oo_dispatch {
        builder = builder.with_oo_dispatch_widening();
    }
    builder.build_function(name, script)
}

pub(super) fn command_namespace(qname: &str) -> String {
    let (holder, _) = tcl_syntax::naming::key_holder_and_tail(qname);
    if holder.is_empty() {
        "::".to_owned()
    } else {
        holder.to_owned()
    }
}
