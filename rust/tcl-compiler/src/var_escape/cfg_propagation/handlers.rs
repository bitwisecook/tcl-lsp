//! Per-call escape handlers, CFG-flavoured (C33e3).
//!
//! Each function takes the call's args, the per-statement
//! `defs: &HashMap<String, Version>` (from the surrounding
//! `SsaStatement`), and a `&mut CfgState`. The handlers tag every
//! escape at the SSA version live at this statement.
//!
//! Mirrors `_handle_<command>` in
//! `core/compiler/var_escape/_cfg_propagation.py`. The shape
//! follows the intra-procedural [`super::super::handlers`]
//! exactly — only the ``state.escape`` call site is different
//! (`CfgState`'s `escape` takes a defs map).

use std::collections::HashMap;

use crate::ssa::Version;
use crate::var_escape::cfg_propagation::state::CfgState;
use crate::var_escape::helpers::{is_dynamic_name, is_dynamic_token, is_name_first_command};
use crate::var_escape::info_subcommands::{
    is_frame_inspecting_info_subcommand, is_safe_info_subcommand,
};
use crate::var_scoping::{
    global_declaration_indices, upvar_local_declaration_indices, variable_declaration_indices,
};

/// Detect the `upvar ?level? src dst ...` shape and apply escape
/// rules. Mirrors the intra-procedural variant.
#[allow(clippy::implicit_hasher)]
pub fn handle_upvar(args: &[String], state: &mut CfgState, defs: &HashMap<String, Version>) {
    if args.is_empty() {
        return;
    }
    let head = &args[0];
    let head_no_dash = head.trim_start_matches('-');
    let is_level_literal = (!head_no_dash.is_empty()
        && head_no_dash.chars().all(|c| c.is_ascii_digit()))
        || (head.starts_with('#') && head[1..].chars().all(|c| c.is_ascii_digit()));
    if head.starts_with('$') && !is_level_literal {
        state.mark_pessimistic();
        return;
    }
    for idx in upvar_local_declaration_indices("upvar", args) {
        if let Some(name) = args.get(idx) {
            state.escape(name, defs);
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
#[allow(clippy::implicit_hasher)]
pub fn handle_global(args: &[String], state: &mut CfgState, defs: &HashMap<String, Version>) {
    let owned: Vec<String> = args.to_vec();
    for idx in global_declaration_indices(&owned) {
        if let Some(name) = args.get(idx) {
            state.escape(name, defs);
        }
    }
}

/// Escape every var declared by `variable name ?value? ...`.
#[allow(clippy::implicit_hasher)]
pub fn handle_variable(args: &[String], state: &mut CfgState, defs: &HashMap<String, Version>) {
    let owned: Vec<String> = args.to_vec();
    for idx in variable_declaration_indices(&owned) {
        if let Some(name) = args.get(idx) {
            state.escape(name, defs);
        }
    }
}

/// Handle `namespace upvar ns src dst ...` — only the upvar
/// subcommand matters here.
#[allow(clippy::implicit_hasher)]
pub fn handle_namespace_call(
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
#[allow(clippy::implicit_hasher)]
pub fn handle_info(args: &[String], state: &mut CfgState, defs: &HashMap<String, Version>) {
    let Some(sub) = args.first() else {
        state.mark_pessimistic();
        return;
    };
    if is_dynamic_token(sub) {
        state.mark_pessimistic();
        return;
    }
    if is_frame_inspecting_info_subcommand(sub) {
        state.mark_pessimistic();
        return;
    }
    if sub == "exists" {
        let Some(target) = args.get(1) else {
            return;
        };
        if is_dynamic_token(target) {
            state.mark_pessimistic();
            return;
        }
        state.escape(target, defs);
        return;
    }
    if is_safe_info_subcommand(sub) {
        return;
    }
    state.mark_pessimistic();
}

/// Handle `set` / `incr` / `append` / `lappend` / `unset` whose
/// first arg is a variable name.
#[allow(clippy::implicit_hasher)]
pub fn handle_dynamic_name_first(
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
        state.escape(&literal, defs);
    } else {
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
        assert!(s.dynamic_barrier);
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
        assert!(s.dynamic_barrier);
    }

    #[test]
    fn info_frame_inspecting_is_pessimistic() {
        for sub in ["level", "frame", "vars", "locals"] {
            let mut s = CfgState::default();
            handle_info(&args_of(&[sub]), &mut s, &HashMap::new());
            assert!(s.dynamic_barrier, "{sub} should be pessimistic");
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
        assert!(!s.dynamic_barrier);
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
