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

use tcl_registry::{
    ArgRole, CallerFrameSelection, InvocationFacts, OwnedSubcommandResolution, StateTransition,
    VariableAliasTarget,
};

use crate::ssa::Version;
use crate::var_escape::cfg_propagation::state::CfgState;
#[cfg(test)]
use crate::var_escape::helpers::{default_registry, is_dynamic_upvar_level};
use crate::var_escape::helpers::{is_dynamic_name, is_dynamic_token, is_name_first_command};
#[cfg(test)]
use crate::var_escape::info_subcommands::{
    is_frame_inspecting_info_subcommand, is_safe_info_subcommand,
};
use crate::var_escape::types::{Barrier, BarrierKind, EscapeReason, EscapeReasonKind};
#[cfg(test)]
use crate::var_scoping::{
    global_declaration_indices, looks_like_level, upvar_local_declaration_indices,
    variable_declaration_indices,
};

/// Apply every variable-cell alias declared by the registry invocation.
/// Returns whether at least one alias transition was present.
pub(crate) fn handle_variable_aliases(
    facts: &InvocationFacts,
    state: &mut CfgState,
    defs: &HashMap<String, Version>,
    registry: &tcl_registry::CommandRegistry,
) -> bool {
    let Some(transitions) = facts.state_transitions.declared() else {
        return false;
    };
    let mut handled = false;
    for fact in transitions.facts() {
        let StateTransition::VariableCellAlias(alias) = &fact.transition else {
            continue;
        };
        handled = true;
        let Some(local) = alias.local.literal() else {
            state.escape_all_known(defs);
            continue;
        };
        state.escape_with_reason(
            local,
            defs,
            EscapeReason::with_detail(
                EscapeReasonKind::UpvarSource,
                format!("{} aliases {local}", facts.canonical_command),
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
                        format!("{} has dynamic frame selection", facts.canonical_command),
                    ));
                    state.record_unbounded_upvar();
                    continue;
                };
                tcl_registry::frame_effect::FrameLevel::parse_in(level, registry)
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
    facts: &InvocationFacts,
    args: &[String],
    state: &mut CfgState,
    defs: &HashMap<String, Version>,
) {
    if facts
        .traits
        .contains(tcl_registry::prelude::Traits::CURRENT_FRAME_INTROSPECTION)
        || matches!(
            &facts.subcommand,
            OwnedSubcommandResolution::Indeterminate { .. }
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
    for (index, role) in &facts.arg_roles {
        if !matches!(role, ArgRole::VarRead | ArgRole::VarWrite) {
            continue;
        }
        let Some(target) = args.get(facts.argument_offset + usize::from(*index)) else {
            continue;
        };
        if is_dynamic_token(target) {
            state.escape_all_known(defs);
        } else if facts.operation
            == tcl_registry::SemanticOperationId::Intrinsic(tcl_registry::IntrinsicId::InfoExists)
        {
            state.escape_with_reason(
                target,
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
        } else {
            state.escape(target, defs);
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
    let is_level_literal = looks_like_level(head);
    if !is_level_literal && is_dynamic_upvar_level(head) {
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

/// Classify `info <subcmd> ...` against the allow-list.
#[cfg(test)]
pub(crate) fn handle_info(args: &[String], state: &mut CfgState, defs: &HashMap<String, Version>) {
    let Some(sub) = args.first() else {
        state.record_barrier(Barrier::with_detail(
            BarrierKind::Info,
            "info (no subcommand)",
        ));
        return;
    };
    if is_dynamic_token(sub) {
        state.record_barrier(Barrier::with_detail(
            BarrierKind::Info,
            format!("info {sub} (dynamic subcommand)"),
        ));
        return;
    }
    if is_frame_inspecting_info_subcommand(sub) {
        state.record_barrier(Barrier::with_detail(
            BarrierKind::Info,
            format!("info {sub}"),
        ));
        return;
    }
    if sub == "exists" {
        let Some(target) = args.get(1) else {
            return;
        };
        if is_dynamic_token(target) {
            state.record_barrier(Barrier::with_detail(
                BarrierKind::Info,
                format!("info exists {target}"),
            ));
            return;
        }
        state.escape_with_reason(
            target,
            defs,
            EscapeReason::with_detail(
                EscapeReasonKind::InfoExists,
                format!("info exists {target}"),
            ),
        );
        return;
    }
    // Escape any registry-declared variable-*write* argument of this `info`
    // subcommand (`info default procname arg varname` writes `varname` in the
    // current frame) before the safe-subcommand short-circuit drops it — issue
    // 151. Registry-driven; no subcommand name or index hardcoded here.
    let arg_refs: Vec<&str> = args.iter().map(String::as_str).collect();
    for idx in
        default_registry().arg_indices_for_role("info", &arg_refs, tcl_registry::ArgRole::VarWrite)
    {
        let Some(target) = args.get(idx) else {
            continue;
        };
        if is_dynamic_token(target) {
            state.record_barrier(Barrier::with_detail(
                BarrierKind::Info,
                format!("info {sub} (dynamic var-write target)"),
            ));
            state.escape_all_known(defs);
        } else {
            state.escape_with_reason(
                target,
                defs,
                EscapeReason::with_detail(
                    EscapeReasonKind::InfoVarWrite,
                    format!("info {sub} writes {target}"),
                ),
            );
        }
    }

    if is_safe_info_subcommand(sub) {
        return;
    }
    state.record_barrier(Barrier::with_detail(
        BarrierKind::Info,
        format!("info {sub} (unknown)"),
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
}
