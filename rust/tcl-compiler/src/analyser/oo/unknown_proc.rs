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

//! The dispatch shape of a user-defined `unknown` procedure, which gates W123.

use super::super::state::Analyser;
use super::super::types::UnknownProcInfo;
use crate::ir::Statement;
use crate::signature_scan::types::ParamDef;
use tcl_registry::Traits;
use tcl_registry::side_effects::SideEffectTarget;

/// Original-handler chaining follows the retained implementation identity.
/// A saved name chosen by the author is not evidence without a proved rename.
fn chains_original_unknown(facts: &tcl_registry::InvocationFacts) -> bool {
    facts.traits.contains(Traits::UNRESOLVED_COMMAND_HANDLER)
}

/// Effective registry effects, including alias prefixes and subcommand forms.
fn spawns_process(facts: &tcl_registry::InvocationFacts) -> bool {
    facts.traits.contains(Traits::UNSAFE)
        && facts
            .effects
            .legacy()
            .side_effects
            .iter()
            .any(|effect| effect.target == SideEffectTarget::Process)
}

fn loads_external_unit(facts: &tcl_registry::InvocationFacts) -> bool {
    facts.traits.contains(Traits::LOADS_EXTERNAL_UNIT)
}

impl Analyser {
    /// Detect dispatch shape of a user-defined ``unknown`` proc.
    ///
    /// Lowers the proc body to IR, then walks the resulting
    /// top-level [`Statement`]s looking for:
    ///
    /// - Selected original case syntax referencing the first parameter —
    ///   exact labels become possible dispatch targets; glob/regexp modes
    ///   mark pattern dispatch. Canonical body-selection residuals remain
    ///   explicit. Selected case-folding child descriptors retain their own
    ///   original operand before setting ``case_insensitive``.
    /// - `Statement::Call` / `Statement::Barrier` that chains the original
    ///   handler ([`chains_original_unknown`]) — sets ``chains_original``.
    /// - a call that shells out ([`spawns_process`]) — sets ``has_exec``.
    /// - a call that loads more script ([`loads_external_unit`]) — sets
    ///   ``has_auto_load``.
    ///
    /// Empty bodies set ``empty_stub = true`` and skip the IR
    /// walk.  Lowering failures fall back to the conservative
    /// "fully dynamic" shape (every flag set, no targets) so
    /// downstream W123 emission stays suppressed.
    pub fn extract_unknown_proc_info(
        &mut self,
        body: &str,
        params: &[ParamDef],
    ) -> UnknownProcInfo {
        self.extract_unknown_proc_info_at(body, params, None)
    }

    /// Secondary body analysis uses the actual document source execution
    /// points when called by the procedure handler.
    pub(crate) fn extract_unknown_proc_info_at(
        &mut self,
        body: &str,
        params: &[ParamDef],
        base: Option<u32>,
    ) -> UnknownProcInfo {
        if body.trim().is_empty() {
            return UnknownProcInfo {
                empty_stub: true,
                ..Default::default()
            };
        }

        let first_param = params
            .first()
            .map_or_else(|| "cmd".to_string(), |p| p.name.clone());

        // Lower to IR.  On panic, be conservative — assume fully
        // dynamic — by returning an ``UnknownProcInfo`` with
        // every dynamic flag set so the W123 emitter suppresses
        // unresolved-command warnings file-wide (the safe
        // direction when we couldn't analyse the handler body).
        // naming.compiler.original-unknown-handler-source-context
        // docs/design/analysis/name-resolution-proofs/compiler-original-unknown-handler-source-context.md
        let context = self.analysis_context();
        let retained = self
            .resolved_input
            .as_ref()
            .or(self.result.resolved_input.as_ref());
        let config = retained.map_or_else(
            || self.lexer_config(),
            super::super::ResolvedAnalysisInput::lexer_config,
        );
        let profile = retained.map_or(
            self.profile,
            super::super::ResolvedAnalysisInput::unit_profile,
        );
        let metadata = retained.and_then(|input| {
            crate::registry_invocation::InvocationMetadataContext::for_source_input(
                context.commands(),
                input,
                config,
                Some(profile),
            )
        });
        let Ok(script) = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let mut lowerer = crate::lowering::Lowerer::with_config(context.commands(), config)
                .with_dialect(Some(profile))
                .with_context_registry(std::sync::Arc::clone(&context));
            if let Some(input) = retained {
                lowerer = lowerer.with_resolved_analysis_input(input.clone());
            }
            if let Some(entry) = &self.source_analysis_entry {
                lowerer.set_source_analysis_options(entry.options());
            } else if let Some(input) =
                retained.filter(|input| input.has_logical_source_name_context())
            {
                // The standalone source helper requires positive retained
                // Logical input; no reconstructed profile grants that axis.
                if let Some(options) =
                    crate::command_binding::SourceAnalysisOptions::for_logical_source(input)
                {
                    lowerer.set_source_analysis_options(options);
                }
            }
            if let Some(base) = base {
                lowerer.lower_into_script_with_bindings(
                    body,
                    base,
                    "::",
                    self.head_identities.source_bindings(),
                )
            } else {
                lowerer.lower(body).top_level.clone()
            }
        })) else {
            return UnknownProcInfo {
                chains_original: true,
                case_insensitive: true,
                has_pattern_dispatch: true,
                has_exec: true,
                has_auto_load: true,
                ..Default::default()
            };
        };

        let mut info = UnknownProcInfo::default();
        for stmt in &script.statements {
            walk_unknown_stmt(
                stmt,
                &script,
                &context,
                metadata,
                &first_param,
                &mut info,
                0,
            );
        }

        info
    }
}

/// Depth cap for [`walk_unknown_stmt`]'s recursion over nested `if`/`for`/
/// `while`/`foreach`/`catch`/`try`/`switch`/`Block`/`UpFrame` bodies —
/// Transitively bounded today via `MAX_LOWER_NEST_DEPTH`
/// (every `Script` this walk sees was built by `crate::lowering`, which
/// already caps its own construction at 256), capped here independently
/// for defence-in-depth and consistency with every other full-tree walker
/// in this crate.
const MAX_UNKNOWN_STMT_WALK_DEPTH: tcl_core_types::RecursionLimit =
    tcl_core_types::RecursionLimit(256);

/// Inspect a single IR statement for unknown-proc dispatch
/// markers.
///
/// Recurses through control-flow bodies (`if` clauses, `for` /
/// `while` / `foreach` bodies, `try` / `catch` bodies) so a
/// ``switch`` arm or ``exec`` call buried inside a guard or
/// loop is still detected. `depth` is `stmt`'s own nesting level — see
/// [`MAX_UNKNOWN_STMT_WALK_DEPTH`].
pub(super) fn walk_unknown_stmt(
    stmt: &Statement,
    script: &crate::ir::Script,
    context: &tcl_registry::model::ContextRegistry,
    metadata: Option<crate::registry_invocation::InvocationMetadataContext<'_>>,
    first_param: &str,
    info: &mut UnknownProcInfo,
    depth: u32,
) {
    if MAX_UNKNOWN_STMT_WALK_DEPTH.exceeded(depth) {
        return;
    }
    update_unknown_statement_facts(stmt, script, context, metadata, first_param, info);
    walk_unknown_stmt_children(stmt, context, metadata, first_param, info, depth);
}

fn update_unknown_statement_facts(
    stmt: &Statement,
    script: &crate::ir::Script,
    context: &tcl_registry::model::ContextRegistry,
    metadata: Option<crate::registry_invocation::InvocationMetadataContext<'_>>,
    first_param: &str,
    info: &mut UnknownProcInfo,
) {
    // Select the purpose only after this original carrier agrees with the
    // complete supplied source owner. Availability-only metadata cannot turn
    // an original Logical carrier into the Native route.
    let source_metadata = metadata.and_then(|metadata| {
        let tokens = script.retained_source_tokens_for_statement(stmt)?;
        tokens
            .source_binding
            .as_ref()?
            .original_invocation_metadata_for_input(
                tokens,
                metadata.source_analysis_input()?,
                context.commands(),
            )
    });
    let case = source_metadata.and_then(|metadata| {
        if script
            .retained_source_tokens_for_statement(stmt)?
            .source_binding
            .as_ref()?
            .original_invocation_source_purpose(
                script.retained_source_tokens_for_statement(stmt)?,
                metadata,
                context.commands(),
            )?
            == crate::command_binding::OriginalInvocationSourcePurpose::Logical
        {
            crate::registry_invocation::source_structure::original_logical_source_case_for_tokens(
                script.retained_source_tokens_for_statement(stmt)?,
                context.commands(),
                metadata,
            )
        } else {
            crate::registry_invocation::source_structure::original_native_source_case_for_statement(
                script,
                stmt,
                context.commands(),
                metadata,
            )
        }
    });
    if let Some(case) = case
        && case.references_parameter(first_param)
    {
        info.case_insensitive |= case.folds_case();
        if case.mode() == tcl_registry::spec::CaseMatchMode::Exact {
            info.dispatch_targets
                .extend(case.patterns().iter().cloned());
        } else {
            info.has_pattern_dispatch = true;
        }
        info.case_body_residuals.push(case.body_operands().clone());
    }
    if matches!(
        stmt,
        Statement::Call { .. } | Statement::Barrier { .. } | Statement::NativeCall { .. }
    ) {
        record_unknown_invocation_facts(stmt, script, context, source_metadata, info);
    }
}

fn record_unknown_invocation_facts(
    stmt: &Statement,
    script: &crate::ir::Script,
    context: &tcl_registry::model::ContextRegistry,
    source_metadata: Option<crate::registry_invocation::InvocationMetadataContext<'_>>,
    info: &mut UnknownProcInfo,
) {
    // naming.compiler.original-unknown-handler-source-context
    // docs/design/analysis/name-resolution-proofs/compiler-original-unknown-handler-source-context.md
    // Logical advice selects the exact operation; Native still needs its
    // independent proved target.
    let invocation = source_metadata.and_then(|metadata| {
        if script
            .retained_source_tokens_for_statement(stmt)?
            .source_binding
            .as_ref()?
            .original_invocation_source_purpose(
                script.retained_source_tokens_for_statement(stmt)?,
                metadata,
                context.commands(),
            )?
            == crate::command_binding::OriginalInvocationSourcePurpose::Logical
        {
            crate::registry_invocation::original_logical_operation_invocation_with_metadata_context(
                context.commands(),
                metadata,
                stmt.tokens()?,
            )
        } else {
            crate::registry_invocation::resolved_statement_invocation_with_metadata_context(
                context.commands(),
                Some(metadata),
                stmt,
            )
        }
    });
    if let Some(invocation) = invocation {
        info.chains_original |= chains_original_unknown(&invocation.facts);
        info.has_exec |= spawns_process(&invocation.facts);
        info.has_auto_load |= loads_external_unit(&invocation.facts);
    }
}

fn walk_unknown_stmt_children(
    stmt: &Statement,
    context: &tcl_registry::model::ContextRegistry,
    metadata: Option<crate::registry_invocation::InvocationMetadataContext<'_>>,
    first_param: &str,
    info: &mut UnknownProcInfo,
    depth: u32,
) {
    match stmt {
        Statement::Switch { arms, .. } => {
            // Recurse into arm bodies (a switch arm may contain
            // an exec or auto_load that should still register).
            for arm in arms {
                if let Some(body) = &arm.body {
                    walk_unknown_script(body, context, metadata, first_param, info, depth + 1);
                }
            }
        }
        Statement::If {
            clauses, else_body, ..
        } => {
            for clause in clauses {
                walk_unknown_script(
                    &clause.body,
                    context,
                    metadata,
                    first_param,
                    info,
                    depth + 1,
                );
            }
            if let Some(body) = else_body {
                walk_unknown_script(body, context, metadata, first_param, info, depth + 1);
            }
        }
        Statement::For { body, .. }
        | Statement::While { body, .. }
        | Statement::Foreach { body, .. }
        | Statement::Catch { body, .. }
        | Statement::Block { body, .. }
        | Statement::UpFrame { body, .. } => {
            walk_unknown_script(body, context, metadata, first_param, info, depth + 1);
        }
        Statement::Try {
            body,
            handlers,
            finally_body,
            ..
        } => {
            walk_unknown_script(body, context, metadata, first_param, info, depth + 1);
            for handler in handlers {
                walk_unknown_script(
                    &handler.body,
                    context,
                    metadata,
                    first_param,
                    info,
                    depth + 1,
                );
            }
            if let Some(body) = finally_body {
                walk_unknown_script(body, context, metadata, first_param, info, depth + 1);
            }
        }
        Statement::AssignConst { .. }
        | Statement::AssignExpr { .. }
        | Statement::AssignValue { .. }
        | Statement::Incr { .. }
        | Statement::ExprEval { .. }
        | Statement::Call { .. }
        | Statement::Barrier { .. }
        | Statement::NativeCall { .. }
        | Statement::Return { .. } => {}
    }
}

fn walk_unknown_script(
    script: &crate::ir::Script,
    context: &tcl_registry::model::ContextRegistry,
    metadata: Option<crate::registry_invocation::InvocationMetadataContext<'_>>,
    first_param: &str,
    info: &mut UnknownProcInfo,
    depth: u32,
) {
    for stmt in &script.statements {
        walk_unknown_stmt(stmt, script, context, metadata, first_param, info, depth);
    }
}
