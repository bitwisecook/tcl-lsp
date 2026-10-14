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

//! Per-call escape handlers.
//!
//! Each function inspects a single `Statement::Call` shape and
//! mutates the supplied `EscapeState`.

use crate::ir::CommandTokens;
#[cfg(test)]
use crate::var_escape::helpers::is_name_first_command;
use crate::var_escape::helpers::{is_dynamic_token, is_dynamic_upvar_level};
use crate::var_escape::state::EscapeState;
use crate::var_escape::types::{Barrier, BarrierKind, EscapeReason, EscapeReasonKind};
use crate::var_scoping::{
    global_declaration_indices, upvar_local_declaration_indices, variable_declaration_indices,
};
use tcl_registry::{
    ArgRole, CallerFrameSelection, InvocationFacts, OwnedSubcommandResolution, StateTransition,
    VariableAliasTarget,
};

/// True if any word in *tokens* is `{*}`-expanded.
#[must_use]
pub fn has_expand_word(tokens: Option<&CommandTokens>) -> bool {
    tokens
        .and_then(|t| t.expand_word.as_ref())
        .is_some_and(|ew| ew.iter().any(|&e| e))
}

/// Apply every variable-cell alias declared by the registry invocation.
/// Returns whether at least one alias transition was present.
pub fn handle_variable_aliases(
    facts: &InvocationFacts,
    state: &mut EscapeState,
    registry: &tcl_registry::CommandRegistry,
) -> bool {
    let aliases = facts
        .state_transitions
        .declared()
        .into_iter()
        .flat_map(tcl_registry::StateTransitions::facts)
        .filter_map(|fact| match &fact.transition {
            StateTransition::VariableCellAlias(alias) => Some(alias),
            _ => None,
        });
    apply_variable_aliases(aliases, state, |level| {
        tcl_registry::FrameLevel::parse_in(level, registry)
    })
}

/// Apply the purpose-limited normal alias projection without requiring a compiler hook.
pub fn handle_normal_variable_aliases(
    normal: &crate::registry_invocation::NormalTransferInvocation,
    state: &mut EscapeState,
) -> bool {
    apply_variable_aliases(normal.variable_alias_transitions(), state, |level| {
        normal.variable_alias_frame_level(&CallerFrameSelection::Explicit(
            tcl_registry::TransitionSubject::Literal(level.to_owned()),
        ))
    })
}

fn apply_variable_aliases<'a>(
    aliases: impl Iterator<Item = &'a tcl_registry::VariableCellAliasTransition>,
    state: &mut EscapeState,
    parse_level: impl Fn(&str) -> Option<tcl_registry::FrameLevel>,
) -> bool {
    let mut handled = false;
    for alias in aliases {
        handled = true;
        let Some(local) = alias.local.literal() else {
            state.escape_all_known();
            continue;
        };
        state.escape_with_reason(
            local,
            EscapeReason::with_detail(
                EscapeReasonKind::UpvarSource,
                format!("variable alias binds {local}"),
            ),
        );
        let VariableAliasTarget::CallerSelectedFrame { frame, variable } = &alias.target else {
            continue;
        };
        let targets_caller = match frame {
            CallerFrameSelection::DefaultCaller => true,
            CallerFrameSelection::Explicit(level) => {
                let Some(level) = level.literal() else {
                    state.record_barrier(Barrier::with_detail(
                        BarrierKind::Upvar,
                        "variable alias has dynamic frame selection",
                    ));
                    state.record_unbounded_upvar();
                    continue;
                };
                parse_level(level)
                    .is_none_or(|level| !level.is_current_frame() && !level.is_global_frame())
            }
        };
        if targets_caller {
            if let Some(variable) = variable.literal() {
                state.record_upvar_source(variable);
            } else {
                state.record_unbounded_upvar();
            }
        }
    }
    handled
}

/// Apply registry-declared variable-name and current-frame introspection.
pub fn handle_introspection(
    invocation: &crate::registry_invocation::ResolvedStatementInvocation,
    state: &mut EscapeState,
) {
    let roles = invocation
        .facts
        .arg_roles
        .iter()
        .map(|&(index, role)| (invocation.facts.argument_offset + usize::from(index), role))
        .collect::<Vec<_>>();
    transfer_introspection(
        &invocation.facts,
        &roles,
        |argument| invocation.original_variable_name_operand(argument),
        state,
    );
}

pub(crate) fn handle_source_introspection(
    invocation: &crate::registry_invocation::SourceNameRoleInventory,
    state: &mut EscapeState,
) {
    transfer_introspection(
        invocation.facts(),
        invocation.roles(),
        |argument| invocation.original_variable_name_operand(argument),
        state,
    );
    if invocation.has_unknown_residual() {
        state.mark_pessimistic();
    }
}

fn transfer_introspection(
    facts: &InvocationFacts,
    roles: &[(usize, ArgRole)],
    argument_literal: impl Fn(usize) -> Option<crate::registry_invocation::OriginalVariableNameOperand>,
    state: &mut EscapeState,
) {
    if (facts
        .traits
        .contains(tcl_registry::prelude::Traits::INTROSPECTS_BY_NAME)
        && facts.arity_accepts_frozen_arguments() != Some(true))
        || facts
            .traits
            .contains(tcl_registry::prelude::Traits::CURRENT_FRAME_INTROSPECTION)
        || matches!(
            &facts.subcommand,
            OwnedSubcommandResolution::Indeterminate { .. }
                | OwnedSubcommandResolution::Unknown { .. }
                | OwnedSubcommandResolution::Ambiguous { .. }
        )
    {
        let detail = facts.subcommand.canonical_name().map_or_else(
            || {
                format!(
                    "{} performs indeterminate frame introspection",
                    facts.canonical_command
                )
            },
            |subcommand| format!("{} {subcommand}", facts.canonical_command),
        );
        state.record_barrier(Barrier::with_detail(BarrierKind::Info, detail));
        return;
    }
    if !facts
        .traits
        .contains(tcl_registry::prelude::Traits::INTROSPECTS_BY_NAME)
    {
        return;
    }
    for (argument, role) in roles {
        if !matches!(role, ArgRole::VarRead | ArgRole::VarWrite) {
            continue;
        }
        let target = argument_literal(*argument);
        let Some(target) = target else {
            state.record_barrier(Barrier::with_detail(
                BarrierKind::Info,
                format!(
                    "{} has an unavailable variable operand",
                    facts.canonical_command
                ),
            ));
            state.escape_all_known();
            continue;
        };
        let name = target.as_str();
        if facts.operation
            == tcl_registry::SemanticOperationId::Intrinsic(tcl_registry::IntrinsicId::InfoExists)
        {
            state.escape_original_name_with_reason(
                &target,
                EscapeReason::with_detail(
                    EscapeReasonKind::InfoExists,
                    format!(
                        "{} {} {name}",
                        facts.canonical_command,
                        facts.subcommand.canonical_name().unwrap_or("<subcommand>")
                    ),
                ),
            );
        } else if *role == ArgRole::VarWrite {
            state.escape_original_name_with_reason(
                &target,
                EscapeReason::with_detail(
                    EscapeReasonKind::InfoVarWrite,
                    format!("{} writes {name}", facts.canonical_command),
                ),
            );
        } else {
            state.escape_original_name_with_reason(
                &target,
                EscapeReason::with_detail(
                    EscapeReasonKind::InfoVarRead,
                    format!("{} reads {name}", facts.canonical_command),
                ),
            );
        }
    }
}

/// Detect the `upvar ?level? src dst ...` shape and apply escape
/// rules: escape every local-side var, record caller-frame
/// source names when the level targets a caller, mark
/// pessimistic on a dynamic level.
pub fn handle_upvar(args: &[String], state: &mut EscapeState) {
    if args.is_empty() {
        return;
    }
    let head = &args[0];
    let is_level_literal = args.len() % 2 == 1;
    if is_level_literal && is_dynamic_upvar_level(head) {
        // Dynamic level — pessimistic. Also flag the unbounded-
        // upvar source so the interprocedural pass forces every
        // caller to spill its locals: a dynamic level can resolve
        // to any caller frame and the source-name pairs that
        // follow may name any of the caller's vars. Without this,
        // a callsite like ``upvar $level $vname var; set var 99``
        // leaves the caller's WASM-local mirror stale because the
        // alias write back into the runtime frame's slot never
        // reaches the mirror.
        state.record_barrier(Barrier::with_detail(
            BarrierKind::Upvar,
            format!("upvar {head}"),
        ));
        state.record_unbounded_upvar();
        return;
    }
    for idx in upvar_local_declaration_indices("upvar", args) {
        if let Some(name) = args.get(idx) {
            state.escape_with_reason(
                name,
                EscapeReason::with_detail(
                    EscapeReasonKind::UpvarSource,
                    format!("upvar {head} {name}"),
                ),
            );
        }
    }

    let (level_literal, pairs_start) = if is_level_literal {
        (head.as_str(), 1usize)
    } else {
        ("1", 0usize)
    };
    let targets_caller = level_literal != "#0" && level_literal != "0";
    if targets_caller && args.len() >= pairs_start + 2 {
        let mut i = pairs_start;
        while i + 1 < args.len() {
            let src = &args[i];
            if is_dynamic_token(src) {
                state.record_unbounded_upvar();
            } else {
                state.record_upvar_source(src);
            }
            i += 2;
        }
    }
}

/// Escape every var named in `global a b c`.
pub fn handle_global(args: &[String], state: &mut EscapeState) {
    let owned: Vec<String> = args.to_vec();
    for idx in global_declaration_indices(&owned) {
        if let Some(name) = args.get(idx) {
            state.escape(name);
        }
    }
}

/// Escape every var declared by `variable name ?value? ...`.
pub fn handle_variable(args: &[String], state: &mut EscapeState) {
    let owned: Vec<String> = args.to_vec();
    for idx in variable_declaration_indices(&owned) {
        if let Some(name) = args.get(idx) {
            state.escape(name);
        }
    }
}

/// Handle the `namespace` compound command for escape purposes.
/// Currently only `namespace upvar ns src dst ...` matters.
pub fn handle_namespace_call(args: &[String], state: &mut EscapeState) {
    let Some(sub) = args.first() else {
        return;
    };
    if sub != "upvar" {
        return;
    }
    let owned: Vec<String> = args.to_vec();
    for idx in upvar_local_declaration_indices("namespace", &owned) {
        if let Some(name) = args.get(idx) {
            state.escape(name);
        }
    }
}

#[cfg(test)]
fn handle_info(args: &[String], state: &mut EscapeState) {
    if !args.is_empty() {
        let source = format!("info {}", args.join(" "));
        if let Some(invocation) =
            crate::var_escape::info_subcommands::original_info_invocation(&source)
        {
            handle_introspection(&invocation, state);
            return;
        }
    }
    state.record_barrier(Barrier::with_detail(
        BarrierKind::Info,
        "info invocation unavailable",
    ));
}

/// Escape unresolved first-name operands using the actual source-role owner.
pub(crate) fn handle_source_name_first(
    command: &str,
    source: Option<&crate::registry_invocation::SourceNameRoleInventory>,
    state: &mut EscapeState,
) {
    if let Some(source) = source {
        if source
            .original_variable_name_operand(source.facts().argument_offset)
            .is_some()
        {
            if source.has_unknown_residual() {
                state.mark_pessimistic();
            }
            return;
        }
    }
    state.record_barrier(Barrier::with_detail(
        BarrierKind::DynName,
        format!("{command} name value unavailable"),
    ));
    state.escape_all_known();
}

/// Handle `set` / `incr` / `append` / `lappend` / `unset` whose
/// first arg is a variable name. If the name is a `$n` reference
/// and `n` was assigned exactly one literal identifier earlier,
/// treat the call as targeting that literal (escape just that
/// name). Otherwise fall back to spilling every known proc-local.
#[cfg(test)]
pub fn handle_dynamic_name_first(cmd: &str, args: &[String], state: &mut EscapeState) {
    debug_assert!(is_name_first_command(cmd));
    let Some(name) = args.first() else {
        return;
    };
    if !crate::var_escape::helpers::is_dynamic_name(name) {
        return;
    }
    if let Some(literal) = state.resolve_literal(name) {
        state.escape_with_reason(
            &literal,
            EscapeReason::with_detail(
                EscapeReasonKind::DynNameResolved,
                format!("{cmd} {name} (resolved to {literal})"),
            ),
        );
    } else {
        // Fallback: spill every known proc-local *and* record the
        // dynamic-name barrier so consumers can render the
        // specific trigger.
        state.record_barrier(Barrier::with_detail(
            BarrierKind::DynName,
            format!("{cmd} {name}"),
        ));
        state.escape_all_known();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::var_escape::types::EscapeTag;

    fn args_of(words: &[&str]) -> Vec<String> {
        words.iter().map(|w| (*w).to_string()).collect()
    }

    #[test]
    fn upvar_escapes_local_dst() {
        let mut s = EscapeState::default();
        handle_upvar(&args_of(&["src", "dst"]), &mut s);
        assert!(s.is_frame_helper("dst"));
    }

    #[test]
    fn upvar_with_level_escapes_dst() {
        let mut s = EscapeState::default();
        handle_upvar(&args_of(&["1", "src", "dst"]), &mut s);
        assert!(s.is_frame_helper("dst"));
    }

    #[test]
    fn upvar_records_caller_source_for_default_level() {
        let mut s = EscapeState::default();
        handle_upvar(&args_of(&["caller_var", "local_alias"]), &mut s);
        assert!(s.upvar_source_names.contains("caller_var"));
    }

    #[test]
    fn upvar_dynamic_level_marks_pessimistic() {
        let mut s = EscapeState::default();
        handle_upvar(&args_of(&["$lvl", "src", "dst"]), &mut s);
        assert!(s.dynamic_barrier());
        // Dynamic-level path also flags the unbounded-upvar source
        // for the interprocedural pass.
        assert!(s.unbounded_upvar_source());
    }

    #[test]
    fn upvar_command_substitution_level_marks_pessimistic() {
        let mut s = EscapeState::default();
        handle_upvar(&args_of(&["[expr {$n}]", "src", "dst"]), &mut s);
        assert!(s.dynamic_barrier());
        assert!(s.unbounded_upvar_source());
    }

    #[test]
    fn upvar_quoted_substituted_level_marks_pessimistic() {
        let mut s = EscapeState::default();
        handle_upvar(&args_of(&["\"#${n}\"", "src", "dst"]), &mut s);
        assert!(s.dynamic_barrier());
        assert!(s.unbounded_upvar_source());
    }

    #[test]
    fn upvar_namespaced_dollar_level_marks_pessimistic() {
        let mut s = EscapeState::default();
        handle_upvar(&args_of(&["$::ns::level", "src", "dst"]), &mut s);
        assert!(s.dynamic_barrier());
        assert!(s.unbounded_upvar_source());
    }

    #[test]
    fn upvar_hash_substituted_level_marks_pessimistic() {
        let mut s = EscapeState::default();
        handle_upvar(&args_of(&["#$n", "src", "dst"]), &mut s);
        assert!(s.dynamic_barrier());
        assert!(s.unbounded_upvar_source());
    }

    #[test]
    fn upvar_global_level_zero_does_not_record_caller_source() {
        let mut s = EscapeState::default();
        handle_upvar(&args_of(&["#0", "global_var", "alias"]), &mut s);
        assert!(s.upvar_source_names.is_empty());
        assert!(s.is_frame_helper("alias"));
    }

    #[test]
    fn global_escapes_named_vars() {
        let mut s = EscapeState::default();
        handle_global(&args_of(&["a", "b", "c"]), &mut s);
        assert!(s.is_frame_helper("a"));
        assert!(s.is_frame_helper("b"));
        assert!(s.is_frame_helper("c"));
    }

    #[test]
    fn variable_escapes_declared_names() {
        let mut s = EscapeState::default();
        // ``variable a 1 b 2 c`` — every-other-arg starting at 0.
        handle_variable(&args_of(&["a", "1", "b", "2", "c"]), &mut s);
        assert!(s.is_frame_helper("a"));
        assert!(s.is_frame_helper("b"));
        assert!(s.is_frame_helper("c"));
    }

    #[test]
    fn namespace_upvar_escapes_dst() {
        let mut s = EscapeState::default();
        handle_namespace_call(&args_of(&["upvar", "ns", "src", "dst"]), &mut s);
        assert!(s.is_frame_helper("dst"));
    }

    #[test]
    fn namespace_other_subcommand_is_noop() {
        let mut s = EscapeState::default();
        handle_namespace_call(&args_of(&["eval", "ns", "body"]), &mut s);
        assert!(s.tags.is_empty());
        assert!(!s.dynamic_barrier());
    }

    #[test]
    fn info_no_subcommand_is_pessimistic() {
        let mut s = EscapeState::default();
        handle_info(&[], &mut s);
        assert!(s.dynamic_barrier());
    }

    #[test]
    fn info_dynamic_subcommand_is_pessimistic() {
        let mut s = EscapeState::default();
        handle_info(&args_of(&["$dyn"]), &mut s);
        assert!(s.dynamic_barrier());
    }

    #[test]
    fn info_frame_inspecting_is_pessimistic() {
        for sub in ["level", "frame", "vars", "locals"] {
            let mut s = EscapeState::default();
            handle_info(&args_of(&[sub]), &mut s);
            assert!(s.dynamic_barrier(), "{sub} should be pessimistic");
        }
    }

    #[test]
    fn info_exists_literal_escapes_target() {
        let mut s = EscapeState::default();
        handle_info(&args_of(&["exists", "myvar"]), &mut s);
        assert!(s.is_frame_helper("myvar"));
    }

    #[test]
    fn info_exists_dynamic_target_is_pessimistic() {
        let mut s = EscapeState::default();
        handle_info(&args_of(&["exists", "$dyn"]), &mut s);
        assert!(s.dynamic_barrier());
    }

    #[test]
    fn info_safe_subcommand_does_nothing() {
        let mut s = EscapeState::default();
        handle_info(&args_of(&["patchlevel"]), &mut s);
        assert!(!s.dynamic_barrier());
        assert!(s.tags.is_empty());
    }

    #[test]
    fn info_default_escapes_its_varname_target() {
        // `info default procname arg varname` writes `varname` in the current
        // frame (registry VarWrite arg role) — it must escape even though
        // `default` is otherwise an interpreter-global read.
        let mut s = EscapeState::default();
        handle_info(&args_of(&["default", "myproc", "argA", "outvar"]), &mut s);
        assert!(
            s.is_frame_helper("outvar"),
            "outvar must escape to the frame"
        );
        // The other operands (procname / arg name) are not var writes.
        assert!(!s.is_frame_helper("myproc"));
        assert!(!s.is_frame_helper("argA"));
        // It is not a full pessimistic spill — only the write target escapes.
        assert!(!s.dynamic_barrier());
    }

    #[test]
    fn info_default_dynamic_varname_is_pessimistic() {
        let mut s = EscapeState::default();
        handle_info(&args_of(&["default", "p", "a", "$dyn"]), &mut s);
        assert!(s.dynamic_barrier());
    }

    #[test]
    fn info_unknown_subcommand_is_pessimistic() {
        let mut s = EscapeState::default();
        handle_info(&args_of(&["totally_made_up"]), &mut s);
        assert!(s.dynamic_barrier());
    }

    #[test]
    fn source_first_names_keep_actual_values_and_terminal_unknown_targets() {
        let registry = tcl_registry::CommandRegistry::build_default();
        for source in [
            "append {$literal} value",
            "append {} value",
            "append $selector value",
        ] {
            let module = crate::lowering::lower_to_ir(source, &registry);
            let statement = &module.top_level.statements[0];
            let tokens = module
                .top_level
                .retained_source_tokens_for_statement(statement)
                .unwrap();
            let roles = crate::registry_invocation::original_source_name_roles(&registry, tokens)
                .expect("actual original FIRST_ARG source owner");
            assert!(
                roles
                    .facts()
                    .traits
                    .contains(tcl_registry::Traits::FIRST_ARG_VARNAME)
            );
            let known = roles
                .original_variable_name_operand(roles.facts().argument_offset)
                .is_some();
            let mut state = EscapeState::new(["protected".to_owned()]);
            state.note_literal_assign("selector", "guessed_target");
            handle_source_name_first(&roles.facts().canonical_command, Some(&roles), &mut state);
            assert_eq!(
                state.dynamic_barrier(),
                !known || roles.has_unknown_residual()
            );
            if !known {
                assert!(state.tags.contains_key("protected"));
                assert!(!state.tags.contains_key("guessed_target"));
            }
            let mut changed = tokens.clone();
            changed.argv_texts[1].push_str(" changed");
            assert!(
                crate::registry_invocation::original_source_name_roles(&registry, &changed)
                    .is_none()
            );
            let mut refused = EscapeState::new(["protected".to_owned()]);
            refused.note_literal_assign("selector", "guessed_target");
            handle_source_name_first(&roles.facts().canonical_command, None, &mut refused);
            assert!(refused.dynamic_barrier());
            assert!(refused.tags.contains_key("protected"));
            assert!(!refused.tags.contains_key("guessed_target"));
        }
    }

    #[test]
    fn dynamic_name_first_resolves_to_literal_when_tracked() {
        let mut s = EscapeState::default();
        s.note_literal_assign("n", "real_target");
        handle_dynamic_name_first("set", &args_of(&["$n", "value"]), &mut s);
        assert!(s.is_frame_helper("real_target"));
    }

    #[test]
    fn dynamic_name_first_unresolved_spills_all_known() {
        let mut s = EscapeState::new(["a".to_string(), "b".to_string()]);
        handle_dynamic_name_first("set", &args_of(&["$n", "value"]), &mut s);
        assert!(s.is_frame_helper("a"));
        assert!(s.is_frame_helper("b"));
    }

    #[test]
    fn dynamic_name_first_static_name_is_noop() {
        // Static name → no dynamic-name-first behaviour.
        let mut s = EscapeState::default();
        handle_dynamic_name_first("set", &args_of(&["plain_name", "value"]), &mut s);
        assert!(s.tags.is_empty());
    }

    // Test-only convenience: read the effective tag for *name*.
    impl EscapeState {
        fn is_frame_helper(&self, name: &str) -> bool {
            self.tags.get(name) == Some(&EscapeTag::Frame)
        }
    }
    #[test]
    fn original_empty_and_sigil_name_operands_keep_escape_reasons() {
        for (source, expected) in [
            ("info exists {}", ""),
            ("info exists {$literal}", "$literal"),
        ] {
            let invocation = crate::var_escape::info_subcommands::original_info_invocation(source)
                .expect("actual original invocation owner");
            assert!(invocation.original_variable_name_operand(0).is_none());
            let mut state = EscapeState::default();
            handle_introspection(&invocation, &mut state);
            assert_eq!(
                state.tags.get(expected),
                Some(&crate::var_escape::types::EscapeTag::Frame)
            );
            assert!(!state.dynamic_barrier());
            assert!(state.tag_reasons.get(expected).is_some_and(|reasons| {
                reasons
                    .iter()
                    .any(|reason| reason.kind == EscapeReasonKind::InfoExists)
            }));
        }
        let registry = tcl_registry::CommandRegistry::build_default();
        let module = crate::lowering::lower_to_ir(
            "proc p {} {info exists {}; info exists {$literal}}",
            &registry,
        );
        let (_, procedure) = module.original_source_procedure_bodies().next().unwrap();
        for (statement, expected) in procedure.body.statements.iter().zip(["", "$literal"]) {
            let tokens = procedure
                .body
                .retained_source_tokens_for_statement(statement)
                .unwrap();
            let names = crate::registry_invocation::original_source_name_roles(&registry, tokens)
                .expect("actual readonly source body owner");
            assert!(names.original_variable_name_operand(0).is_none());
            let mut state = EscapeState::default();
            handle_source_introspection(&names, &mut state);
            assert_eq!(
                state.tags.get(expected),
                Some(&crate::var_escape::types::EscapeTag::Frame)
            );
            assert!(state.tag_reasons.get(expected).is_some_and(|reasons| {
                reasons
                    .iter()
                    .any(|reason| reason.kind == EscapeReasonKind::InfoExists)
            }));
            let mut changed = tokens.clone();
            changed.argv_texts[2].push_str(" changed");
            assert!(
                crate::registry_invocation::original_source_name_roles(&registry, &changed)
                    .is_none()
            );
        }
    }

    #[test]
    fn owned_info_operands_preserve_original_literal_names() {
        for (source, expected) in [
            (r"info exists {$literal}", "$literal"),
            (r"info exists \$literal", "$literal"),
            (r"info exists {[literal]}", "[literal]"),
            (r"info exists \[literal\]", "[literal]"),
        ] {
            let invocation =
                crate::var_escape::info_subcommands::original_info_invocation(source).unwrap();
            let mut state = EscapeState::default();
            handle_introspection(&invocation, &mut state);
            assert!(!state.dynamic_barrier(), "{source}");
            assert!(state.is_frame_helper(expected), "{source}: {expected}");
        }
    }

    #[test]
    fn owned_info_unknown_ambiguous_and_missing_operands_are_pessimistic() {
        for source in [
            "info cancel",
            "info constants",
            "info c",
            "info exists",
            "info exists $dynamic",
        ] {
            let invocation =
                crate::var_escape::info_subcommands::original_info_invocation(source).unwrap();
            let mut state = EscapeState::default();
            handle_introspection(&invocation, &mut state);
            assert!(state.dynamic_barrier(), "{source}");
        }
        let invocation =
            crate::var_escape::info_subcommands::original_info_invocation("info consts").unwrap();
        let mut state = EscapeState::default();
        handle_introspection(&invocation, &mut state);
        assert!(!state.dynamic_barrier());
    }
}
