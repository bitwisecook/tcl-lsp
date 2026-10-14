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

//! Per-call escape handlers, CFG-flavoured.
//!
//! Each function takes the call's args, the per-statement
//! `defs: &HashMap<String, Version>` (from the surrounding
//! `SsaStatement`), and a `&mut CfgState`. The handlers tag every
//! escape at the SSA version live at this statement.
//!
//! The shape follows the intra-procedural
//! [`super::super::handlers`] exactly — only the ``state.escape``
//! call site is different (`CfgState`'s `escape` takes a defs map).

use std::collections::HashMap;
use tcl_registry::InvocationFacts;

use tcl_registry::{ArgRole, CallerFrameSelection, OwnedSubcommandResolution, VariableAliasTarget};

use crate::ssa::Version;
use crate::var_escape::cfg_propagation::state::CfgState;
#[cfg(test)]
use crate::var_escape::helpers::is_dynamic_upvar_level;
use crate::var_escape::helpers::{is_dynamic_name, is_dynamic_token, is_name_first_command};
use crate::var_escape::types::{Barrier, BarrierKind, EscapeReason, EscapeReasonKind};
#[cfg(test)]
use crate::var_scoping::{
    global_declaration_indices, upvar_local_declaration_indices, variable_declaration_indices,
};

/// Apply the purpose-limited normal alias projection without requiring a compiler hook.
pub(crate) fn handle_normal_variable_aliases(
    normal: &crate::registry_invocation::NormalTransferInvocation,
    state: &mut CfgState,
    defs: &HashMap<String, Version>,
) -> bool {
    apply_variable_aliases(normal.variable_alias_transitions(), state, defs, |level| {
        normal.variable_alias_frame_level(&CallerFrameSelection::Explicit(
            tcl_registry::TransitionSubject::Literal(level.to_owned()),
        ))
    })
}

fn apply_variable_aliases<'a>(
    aliases: impl Iterator<Item = &'a tcl_registry::VariableCellAliasTransition>,
    state: &mut CfgState,
    defs: &HashMap<String, Version>,
    parse_level: impl Fn(&str) -> Option<tcl_registry::FrameLevel>,
) -> bool {
    let mut handled = false;
    for alias in aliases {
        handled = true;
        let Some(local) = alias.local.literal() else {
            state.escape_all_known(defs);
            continue;
        };
        state.escape_resolved_name_with_reason(
            local,
            defs,
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
pub(crate) fn handle_introspection(
    invocation: &crate::registry_invocation::ResolvedStatementInvocation,
    state: &mut CfgState,
    defs: &HashMap<String, Version>,
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
        |argument| invocation.argument_literal(argument),
        state,
        defs,
    );
}

pub(crate) fn handle_source_introspection(
    invocation: &crate::registry_invocation::SourceNameRoleInventory,
    state: &mut CfgState,
    defs: &HashMap<String, Version>,
) {
    transfer_introspection(
        invocation.facts(),
        invocation.roles(),
        |argument| invocation.argument_literal(argument),
        state,
        defs,
    );
    if invocation.has_unknown_residual() {
        state.mark_pessimistic();
    }
}

fn transfer_introspection(
    facts: &InvocationFacts,
    roles: &[(usize, ArgRole)],
    argument_literal: impl Fn(usize) -> Option<String>,
    state: &mut CfgState,
    defs: &HashMap<String, Version>,
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
        state.mark_pessimistic();
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
            state.escape_all_known(defs);
            continue;
        };
        if facts.operation
            == tcl_registry::SemanticOperationId::Intrinsic(tcl_registry::IntrinsicId::InfoExists)
        {
            state.escape_resolved_name_with_reason(
                &target,
                defs,
                EscapeReason::with_detail(
                    EscapeReasonKind::InfoExists,
                    format!(
                        "{} {} {target}",
                        facts.canonical_command,
                        facts.subcommand.canonical_name().unwrap_or("<subcommand>")
                    ),
                ),
            );
        } else if *role == ArgRole::VarWrite {
            state.escape_resolved_name_with_reason(
                &target,
                defs,
                EscapeReason::with_detail(
                    EscapeReasonKind::InfoVarWrite,
                    format!("{} writes {target}", facts.canonical_command),
                ),
            );
        } else {
            state.escape_resolved_name_with_reason(
                &target,
                defs,
                EscapeReason::with_detail(
                    EscapeReasonKind::InfoVarRead,
                    format!("{} reads {target}", facts.canonical_command),
                ),
            );
        }
    }
}

/// Detect the `upvar ?level? src dst ...` shape and apply escape
/// rules. Mirrors the intra-procedural variant.
#[cfg(test)]
pub(crate) fn handle_upvar(args: &[String], state: &mut CfgState, defs: &HashMap<String, Version>) {
    if args.is_empty() {
        return;
    }
    let head = &args[0];
    let is_level_literal = args.len() % 2 == 1;
    if is_level_literal && is_dynamic_upvar_level(head) {
        // Dynamic level — pessimistic. See the matching block in
        // [`super::super::handlers::handle_upvar`] for the
        // unbounded-upvar rationale.
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
                defs,
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
#[cfg(test)]
pub(crate) fn handle_global(
    args: &[String],
    state: &mut CfgState,
    defs: &HashMap<String, Version>,
) {
    let owned: Vec<String> = args.to_vec();
    for idx in global_declaration_indices(&owned) {
        if let Some(name) = args.get(idx) {
            state.escape(name, defs);
        }
    }
}

/// Escape every var declared by `variable name ?value? ...`.
#[cfg(test)]
pub(crate) fn handle_variable(
    args: &[String],
    state: &mut CfgState,
    defs: &HashMap<String, Version>,
) {
    let owned: Vec<String> = args.to_vec();
    for idx in variable_declaration_indices(&owned) {
        if let Some(name) = args.get(idx) {
            state.escape(name, defs);
        }
    }
}

/// Handle `namespace upvar ns src dst ...` — only the upvar
/// subcommand matters here.
#[cfg(test)]
pub(crate) fn handle_namespace_call(
    args: &[String],
    state: &mut CfgState,
    defs: &HashMap<String, Version>,
) {
    let Some(sub) = args.first() else {
        return;
    };
    if sub != "upvar" {
        return;
    }
    let owned: Vec<String> = args.to_vec();
    for idx in upvar_local_declaration_indices("namespace", &owned) {
        if let Some(name) = args.get(idx) {
            state.escape(name, defs);
        }
    }
}

#[cfg(test)]
fn handle_info(args: &[String], state: &mut CfgState, defs: &HashMap<String, Version>) {
    if !args.is_empty() {
        let source = format!("info {}", args.join(" "));
        if let Some(invocation) =
            crate::var_escape::info_subcommands::original_info_invocation(&source)
        {
            handle_introspection(&invocation, state, defs);
            return;
        }
    }
    state.record_barrier(Barrier::with_detail(
        BarrierKind::Info,
        "info invocation unavailable",
    ));
}

/// Handle `set` / `incr` / `append` / `lappend` / `unset` whose
/// first arg is a variable name.
pub(crate) fn handle_dynamic_name_first(
    cmd: &str,
    args: &[String],
    state: &mut CfgState,
    defs: &HashMap<String, Version>,
) {
    debug_assert!(is_name_first_command(cmd));
    let Some(name) = args.first() else {
        return;
    };
    if !is_dynamic_name(name) {
        return;
    }
    if let Some(literal) = state.resolve_literal(name) {
        state.escape_with_reason(
            &literal,
            defs,
            EscapeReason::with_detail(
                EscapeReasonKind::DynNameResolved,
                format!("{cmd} {name} (resolved to {literal})"),
            ),
        );
    } else {
        state.record_barrier(Barrier::with_detail(
            BarrierKind::DynName,
            format!("{cmd} {name}"),
        ));
        state.escape_all_known(defs);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::var_escape::types::EscapeTag;

    fn args_of(words: &[&str]) -> Vec<String> {
        words.iter().map(|w| (*w).to_string()).collect()
    }

    fn defs_with(name: &str, version: Version) -> HashMap<String, Version> {
        let mut m = HashMap::new();
        m.insert(name.to_string(), version);
        m
    }

    #[test]
    fn upvar_escapes_dst_at_def_version() {
        let mut s = CfgState::default();
        let defs = defs_with("dst", 4);
        handle_upvar(&args_of(&["src", "dst"]), &mut s, &defs);
        assert_eq!(s.ssa_tags.get(&("dst".into(), 4)), Some(&EscapeTag::Frame));
    }

    #[test]
    fn upvar_records_caller_source_for_default_level() {
        let mut s = CfgState::default();
        handle_upvar(
            &args_of(&["caller_var", "local_alias"]),
            &mut s,
            &HashMap::new(),
        );
        assert!(s.upvar_source_names.contains("caller_var"));
    }

    #[test]
    fn upvar_dynamic_level_marks_pessimistic() {
        let mut s = CfgState::default();
        handle_upvar(&args_of(&["$lvl", "src", "dst"]), &mut s, &HashMap::new());
        assert!(s.dynamic_barrier());
        // Dynamic-level path also flags the unbounded-upvar source.
        assert!(s.unbounded_upvar_source());
    }

    #[test]
    fn upvar_command_substitution_level_marks_pessimistic() {
        let mut s = CfgState::default();
        handle_upvar(
            &args_of(&["[expr {$n}]", "src", "dst"]),
            &mut s,
            &HashMap::new(),
        );
        assert!(s.dynamic_barrier());
        assert!(s.unbounded_upvar_source());
    }

    #[test]
    fn upvar_quoted_substituted_level_marks_pessimistic() {
        let mut s = CfgState::default();
        handle_upvar(
            &args_of(&["\"#${n}\"", "src", "dst"]),
            &mut s,
            &HashMap::new(),
        );
        assert!(s.dynamic_barrier());
        assert!(s.unbounded_upvar_source());
    }

    #[test]
    fn upvar_namespaced_dollar_level_marks_pessimistic() {
        let mut s = CfgState::default();
        handle_upvar(
            &args_of(&["$::ns::level", "src", "dst"]),
            &mut s,
            &HashMap::new(),
        );
        assert!(s.dynamic_barrier());
        assert!(s.unbounded_upvar_source());
    }

    #[test]
    fn upvar_hash_substituted_level_marks_pessimistic() {
        let mut s = CfgState::default();
        handle_upvar(&args_of(&["#$n", "src", "dst"]), &mut s, &HashMap::new());
        assert!(s.dynamic_barrier());
        assert!(s.unbounded_upvar_source());
    }

    #[test]
    fn upvar_global_level_zero_does_not_record_caller_source() {
        let mut s = CfgState::default();
        handle_upvar(
            &args_of(&["#0", "global_var", "alias"]),
            &mut s,
            &HashMap::new(),
        );
        assert!(s.upvar_source_names.is_empty());
        assert_eq!(
            s.ssa_tags.get(&("alias".into(), 0)),
            Some(&EscapeTag::Frame)
        );
    }

    #[test]
    fn global_escapes_named_vars() {
        let mut s = CfgState::default();
        handle_global(&args_of(&["a", "b"]), &mut s, &HashMap::new());
        assert!(s.ssa_tags.contains_key(&("a".into(), 0)));
        assert!(s.ssa_tags.contains_key(&("b".into(), 0)));
    }

    #[test]
    fn variable_escapes_declared_names() {
        let mut s = CfgState::default();
        handle_variable(
            &args_of(&["a", "1", "b", "2", "c"]),
            &mut s,
            &HashMap::new(),
        );
        assert!(s.ssa_tags.contains_key(&("a".into(), 0)));
        assert!(s.ssa_tags.contains_key(&("b".into(), 0)));
        assert!(s.ssa_tags.contains_key(&("c".into(), 0)));
    }

    #[test]
    fn namespace_upvar_escapes_dst() {
        let mut s = CfgState::default();
        handle_namespace_call(
            &args_of(&["upvar", "ns", "src", "dst"]),
            &mut s,
            &HashMap::new(),
        );
        assert!(s.ssa_tags.contains_key(&("dst".into(), 0)));
    }

    #[test]
    fn info_no_subcommand_is_pessimistic() {
        let mut s = CfgState::default();
        handle_info(&[], &mut s, &HashMap::new());
        assert!(s.dynamic_barrier());
    }

    #[test]
    fn info_frame_inspecting_is_pessimistic() {
        for sub in ["level", "frame", "vars", "locals"] {
            let mut s = CfgState::default();
            handle_info(&args_of(&[sub]), &mut s, &HashMap::new());
            assert!(s.dynamic_barrier(), "{sub} should be pessimistic");
        }
    }

    #[test]
    fn info_exists_literal_escapes_target() {
        let mut s = CfgState::default();
        handle_info(&args_of(&["exists", "myvar"]), &mut s, &HashMap::new());
        assert!(s.ssa_tags.contains_key(&("myvar".into(), 0)));
    }

    #[test]
    fn info_safe_subcommand_does_nothing() {
        let mut s = CfgState::default();
        handle_info(&args_of(&["patchlevel"]), &mut s, &HashMap::new());
        assert!(!s.dynamic_barrier());
        assert!(s.ssa_tags.is_empty());
    }

    #[test]
    fn dynamic_name_first_resolves_to_literal_when_tracked() {
        let mut s = CfgState::default();
        s.note_literal_assign("n", "real_target");
        handle_dynamic_name_first("set", &args_of(&["$n", "value"]), &mut s, &HashMap::new());
        assert!(s.ssa_tags.contains_key(&("real_target".into(), 0)));
    }

    #[test]
    fn dynamic_name_first_unresolved_spills_all_known() {
        let mut s = CfgState::new(["a".to_string(), "b".to_string()]);
        handle_dynamic_name_first("set", &args_of(&["$n", "value"]), &mut s, &HashMap::new());
        assert!(s.ssa_tags.contains_key(&("a".into(), 0)));
        assert!(s.ssa_tags.contains_key(&("b".into(), 0)));
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
            let mut state = CfgState::default();
            handle_introspection(&invocation, &mut state, &HashMap::new());
            assert!(!state.dynamic_barrier(), "{source}");
            assert!(
                state.ssa_tags.contains_key(&(expected.to_owned(), 0)),
                "{source}: {expected}"
            );
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
            let mut state = CfgState::default();
            handle_introspection(&invocation, &mut state, &HashMap::new());
            assert!(state.dynamic_barrier(), "{source}");
        }
        let invocation =
            crate::var_escape::info_subcommands::original_info_invocation("info consts").unwrap();
        let mut state = CfgState::default();
        handle_introspection(&invocation, &mut state, &HashMap::new());
        assert!(!state.dynamic_barrier());
    }
}
