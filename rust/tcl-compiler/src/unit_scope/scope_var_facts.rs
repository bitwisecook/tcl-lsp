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

//! Literal-value facts for the local variables of each caller scope, and the literal values a word may take there.

use super::{
    CallSiteScanCtx, CallerFrame, CallerScopeId, ExtraCallSiteScanContext, ModuleVarFacts,
    ScopeVars, call_site_scan_functions,
};
use crate::cfg::{CfgModule, Function as CfgFunction};
use crate::ir::{Module as IrModule, Statement};
use crate::naming::is_dynamic_word;
use crate::value_shapes::whole_word_scalar_var_name;
use std::collections::BTreeSet;
use tcl_registry::{ArgRole, Traits};

/// The set of literal strings a word may evaluate to.
pub(super) enum WordValues {
    /// Fully enumerated — the word can only ever be one of these.
    Literals(BTreeSet<String>),
    /// Not enumerable by this scan.
    Unknown,
}

impl ScopeVars {
    /// Record an assignment to the variable `name` names — `Some(value)`
    /// when the assigned value is a compile-time literal, `None` when it
    /// is not.  Poisons the whole scope when the *name* is computed.
    fn note_assignment(&mut self, name: &str, value: Option<&str>) {
        if is_dynamic_word(name) {
            self.dynamic_name_write = true;
            return;
        }
        let facts = self.vars.entry(name.to_owned()).or_default();
        match value {
            Some(literal) => {
                facts.values.insert(literal.to_owned());
            }
            None => facts.unknown = true,
        }
    }

    fn note_unknown(&mut self, name: &str) {
        self.vars.entry(name.to_owned()).or_default().unknown = true;
    }

    /// Record a write to the variable a *word* names — poisoning the whole
    /// scope when the name itself is computed.
    fn note_write_word(&mut self, word: &str) {
        if is_dynamic_word(word) {
            self.dynamic_name_write = true;
        } else {
            self.note_unknown(word);
        }
    }
}

/// Collect literal-value facts for one scope's local variables.
///
/// Constant assignments (`set cmd helper`, or the plain-bareword
/// `AssignValue` shape lowering leaves alone) contribute a literal; every
/// other write contributes "unknown" for that name.  Which *words* of a
/// generic call name a variable is command-surface data
/// ([`ArgRole::VarWrite`], plus [`Traits::CREATES_SCOPE_ALIAS`] for the
/// vararg alias forms `global x y z` / `variable a b` / `upvar 1 a b 1 c d`,
/// whose per-argument name list the role query deliberately does not expand) — no command name is
/// matched here.
///
/// Structured bodies (`if`, `while`, `catch`, a *literal* `eval`/`uplevel
/// #0` body) are already flattened into the caller's own CFG blocks by
/// lowering, so their writes are ordinary block statements here.  The two
/// that are not — a frame-shifting [`Statement::UpFrame`] body and a body
/// reached only through a substitution on a [`Traits::EVALUATES_CODE`]
/// command — set [`ScopeVars::cross_frame_write`] instead.
#[cfg(test)]
pub(super) fn collect_scope_var_facts(
    cfg: &CfgFunction,
    surface: &tcl_registry::model::DocumentCommandSurface<'_>,
    context: Option<&tcl_registry::model::ContextRegistry>,
) -> ScopeVars {
    let mut out = ScopeVars::default();
    if context.is_none_or(|context| {
        !crate::registry_invocation::InvocationMetadataContext::from(context)
            .matches_registry(surface.commands())
    }) {
        out.dynamic_name_write = true;
        out.cross_frame_write = true;
        return out;
    }
    for block in cfg.blocks.values() {
        for stmt in &block.statements {
            match stmt {
                Statement::AssignConst { name, value, .. } => {
                    out.note_assignment(name, Some(value));
                }
                // Lowering only produces `AssignConst` for a *recognised*
                // constant shape; a plain bareword value (`set cmd helper`)
                // arrives as `AssignValue` and is just as literal — provided
                // it neither substitutes nor needs backslash resolution.
                Statement::AssignValue {
                    name,
                    value,
                    value_needs_backsubst,
                    ..
                } => {
                    let literal =
                        (!*value_needs_backsubst && !is_dynamic_word(value)).then_some(value);
                    out.note_assignment(name, literal.map(String::as_str));
                }
                Statement::AssignExpr { name, .. } | Statement::Incr { name, .. } => {
                    out.note_write_word(name);
                }
                Statement::Call { defs, .. } => {
                    for d in defs {
                        out.note_write_word(d);
                    }
                    note_surface_var_writes(&mut out, surface, stmt, context);
                }
                Statement::Barrier { .. } => {
                    note_surface_var_writes(&mut out, surface, stmt, context);
                }
                // `uplevel N {…}`: the body's writes land in another
                // frame, which this per-scope walk does not own.
                Statement::UpFrame { .. } => out.cross_frame_write = true,
                _ => {}
            }
        }
    }
    out
}

/// Record the variable writes the command surface declares for one call.
#[cfg(test)]
fn note_surface_var_writes(
    out: &mut ScopeVars,
    surface: &tcl_registry::model::DocumentCommandSurface<'_>,
    stmt: &Statement,
    context: Option<&tcl_registry::model::ContextRegistry>,
) {
    let registry = surface.commands();
    let Some(invocation) = stmt.tokens().and_then(|tokens| {
        crate::registry_invocation::normal_transfer_invocation_with_metadata_context(
            registry,
            context.map(Into::into),
            tokens,
        )
    }) else {
        if stmt.tokens().is_some_and(|tokens| {
            tokens.has_unproved_source_binding()
                || crate::registry_invocation::registry_invocation_assistance_with_metadata_context(
                    registry,
                    context.map(Into::into),
                    tokens,
                )
                .is_none_or(|assistance| {
                    assistance.candidates.is_empty()
                        || assistance.unknown_residual
                        || assistance.may_be_absent
                })
        }) {
            out.dynamic_name_write = true;
            out.cross_frame_write = true;
        }
        return;
    };
    for (index, role) in invocation.variable_roles() {
        if role == ArgRole::VarWrite {
            if let Some(name) = invocation.argument_literal(index) {
                out.note_unknown(&name);
            } else {
                out.dynamic_name_write = true;
            }
        }
    }
    for alias in invocation.variable_alias_transitions() {
        if let Some(local) = alias.local.literal() {
            out.note_unknown(local);
        } else {
            out.dynamic_name_write = true;
        }
    }
}

/// The literal values `word` may take when evaluated in `caller`'s scope.
///
/// A word with no substitution is its own only value.  A whole-word scalar
/// reference (`$cmd` / `${cmd}`) resolves against the scope's literal-value
/// facts, unioned — when the name is one of the body's own parameters —
/// with the literals its callers pass at that position
/// ([`parameter_values`]).  Anything else is [`WordValues::Unknown`].
pub(super) fn word_values(
    ctx: &CallSiteScanCtx<'_, impl std::hash::BuildHasher>,
    caller: &CallerFrame<'_>,
    word: &str,
) -> WordValues {
    if !is_dynamic_word(word) {
        return WordValues::Literals(std::iter::once(word.to_owned()).collect());
    }
    let Some(var) = whole_word_scalar_var_name(word, ctx.module.lexer_config) else {
        return WordValues::Unknown;
    };
    // A namespace-qualified reference names a variable any body in the
    // module (or another file) may write; this scan models local frames
    // only.
    if crate::naming::is_qualified(var.as_bytes()) {
        return WordValues::Unknown;
    }
    // A script running in a frame this scan cannot read may have assigned
    // any local of any scope.
    if ctx.var_facts.cross_frame_write {
        return WordValues::Unknown;
    }
    let Some(scope_vars) = ctx.var_facts.scopes.get(&caller.scope) else {
        return WordValues::Unknown;
    };
    if scope_vars.dynamic_name_write {
        return WordValues::Unknown;
    }
    let mut values = BTreeSet::new();
    if let Some(facts) = scope_vars.vars.get(var) {
        if facts.unknown {
            return WordValues::Unknown;
        }
        values.extend(facts.values.iter().cloned());
    }
    match parameter_values(ctx, caller, var) {
        Some(WordValues::Unknown) => return WordValues::Unknown,
        Some(WordValues::Literals(from_callers)) => values.extend(from_callers),
        None => {}
    }
    WordValues::Literals(values)
}

/// The literals `var`'s callers pass when `var` is one of `caller`'s
/// declared parameters, or `None` when it is not a parameter at all.
///
/// A parameter with no recorded caller contributes nothing: within a
/// compilation unit already gated on having no `package provide`, an
/// uncalled procedure's body never runs. A parameter of a body whose
/// callers this scan does not attribute at all (a method, an `apply`
/// lambda) can hold anything.
fn parameter_values(
    ctx: &CallSiteScanCtx<'_, impl std::hash::BuildHasher>,
    caller: &CallerFrame<'_>,
    var: &str,
) -> Option<WordValues> {
    let index = caller.params.iter().position(|p| p == var)?;
    // `args` slurps every remaining argument into a *list*, so its value is
    // not any one caller's word.
    if var == "args" || !caller.callers_tracked {
        return Some(WordValues::Unknown);
    }
    let CallerScopeId::Procedure(id) = caller.scope else {
        return Some(WordValues::Unknown);
    };
    let previous = ctx.previous.get_for_body(ctx.module, id);
    match previous
        .as_ref()
        .and_then(|evidence| evidence.slots.get(&index))
    {
        Some(slot) if slot.unknown => Some(WordValues::Unknown),
        Some(slot) => Some(WordValues::Literals(slot.values.clone())),
        None => Some(WordValues::Literals(BTreeSet::new())),
    }
}

/// Collect per-scope variable literal facts for every body the scan walks as
/// a caller — the input a dispatch word's value set is read from.
pub(super) fn collect_module_scope_var_facts(
    cfg_module: &CfgModule,
    extra_callers: &[ExtraCallSiteScanContext],
    surface: &tcl_registry::model::DocumentCommandSurface<'_>,
    module: &IrModule,
    identities: &crate::realm::CommandBindingRealm,
) -> ModuleVarFacts {
    let mut out = ModuleVarFacts::default();
    for (scope, func) in call_site_scan_functions(cfg_module, extra_callers) {
        let facts = collect_original_scope_var_facts(func, surface, module, identities);
        out.cross_frame_write |= facts.cross_frame_write;
        out.scopes.insert(scope, facts);
    }
    out
}

/// Local literal sets and possible output names under the actual Module/CFG
/// owners. A missing source operation poisons dispatch-value enumeration; a
/// conditional authored write never certifies an executed variable-cell store.
fn collect_original_scope_var_facts(
    cfg: &CfgFunction,
    surface: &tcl_registry::model::DocumentCommandSurface<'_>,
    module: &IrModule,
    identities: &crate::realm::CommandBindingRealm,
) -> ScopeVars {
    let mut out = ScopeVars::default();
    let registry = surface.commands();
    let Some(metadata) = crate::registry_invocation::InvocationMetadataContext::for_module(
        registry, module,
    )
    .filter(|_| {
        module
            .retained_source_bindings
            .as_deref()
            .is_some_and(|owner| owner.matches_module(module, registry))
    }) else {
        out.dynamic_name_write = true;
        out.cross_frame_write = true;
        return out;
    };
    let Some(input) = metadata.source_analysis_input() else {
        out.dynamic_name_write = true;
        out.cross_frame_write = true;
        return out;
    };
    for (&block_id, block) in &cfg.blocks {
        for (index, stmt) in block.statements.iter().enumerate() {
            if stmt
                .tokens()
                .is_some_and(|tokens| tokens.synthetic.is_some())
            {
                continue;
            }
            let original = (|| {
                let tokens = cfg.source_tokens_at(block_id, index)?;
                let binding = tokens.source_binding.as_ref()?;
                binding
                    .original_invocation_metadata_for_owner(tokens, &cfg.metadata_context, registry)
                    .ok()??;
                binding.original_invocation_metadata_for_module(tokens, module, registry)?;
                Some(tokens)
            })();
            let Some(tokens) = original else {
                out.dynamic_name_write = true;
                out.cross_frame_write = true;
                continue;
            };
            if note_original_scope_var_statement(stmt, &mut out) {
                continue;
            }
            record_original_scope_call_facts(tokens, identities, input, &mut out);
        }
    }
    out
}

fn record_original_scope_call_facts(
    tokens: &crate::ir::CommandTokens,
    identities: &crate::realm::CommandBindingRealm,
    input: &crate::analyser::ResolvedAnalysisInput,
    out: &mut ScopeVars,
) {
    let Some(words) =
        crate::registry_invocation::source_structure::original_registry_words_for_supplied_tokens(
            identities, input, tokens,
        )
    else {
        out.dynamic_name_write = true;
        out.cross_frame_write = true;
        return;
    };
    let context = input.context_registry();
    let selected = words
        .with_source_schema(&context, |schema| {
            let (_, complete) = schema.authored_source_argument_roles();
            complete.then(|| {
                (
                    schema.authored_source_variable_name_arguments(),
                    schema.semantics.traits,
                )
            })
        })
        .flatten();
    let Some((roles, traits)) = selected else {
        out.dynamic_name_write = true;
        out.cross_frame_write = true;
        return;
    };
    for (ordinal, role) in roles {
        if role != ArgRole::VarWrite {
            continue;
        }
        if let Some(name) = words
            .arguments()
            .get(ordinal)
            .and_then(|word| word.literal_bytes())
            .and_then(|bytes| std::str::from_utf8(bytes).ok())
        {
            out.note_unknown(name);
        } else {
            out.dynamic_name_write = true;
        }
    }
    // Alias destinations and a shifted/unavailable body are independently possible writes.
    if traits.contains(Traits::CREATES_SCOPE_ALIAS) {
        out.dynamic_name_write = true;
    }
    if traits.contains(Traits::EVALUATES_IN_SHIFTED_FRAME)
        || (traits.contains(Traits::EVALUATES_CODE)
            && words
                .source_script_bodies_for_mutation_coverage(&context)
                .is_none())
    {
        out.cross_frame_write = true;
    }
}

fn note_original_scope_var_statement(stmt: &Statement, out: &mut ScopeVars) -> bool {
    match stmt {
        Statement::AssignConst { name, value, .. } => out.note_assignment(name, Some(value)),
        Statement::AssignValue {
            name,
            value,
            value_needs_backsubst,
            ..
        } => {
            let value =
                (!*value_needs_backsubst && !is_dynamic_word(value)).then_some(value.as_str());
            out.note_assignment(name, value);
        }
        Statement::AssignExpr { name, .. } | Statement::Incr { name, .. } => {
            out.note_write_word(name);
        }
        Statement::UpFrame { .. } => out.cross_frame_write = true,
        Statement::Call { .. } | Statement::Barrier { .. } => return false,
        _ => return true,
    }
    true
}
