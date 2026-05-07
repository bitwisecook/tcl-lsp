//! Per-call escape handlers (C33b4).
//!
//! Each function inspects a single `Statement::Call` shape and
//! mutates the supplied `EscapeState`. Mirrors the
//! `_handle_<command>` family from
//! `core/compiler/var_escape/_propagation.py`.

use crate::ir::CommandTokens;
use crate::var_escape::helpers::{is_dynamic_token, is_name_first_command};
use crate::var_escape::info_subcommands::{
    is_frame_inspecting_info_subcommand, is_safe_info_subcommand,
};
use crate::var_escape::state::EscapeState;
use crate::var_scoping::{
    global_declaration_indices, upvar_local_declaration_indices, variable_declaration_indices,
};

/// True if any word in *tokens* is `{*}`-expanded.
#[must_use]
pub fn has_expand_word(tokens: Option<&CommandTokens>) -> bool {
    tokens
        .and_then(|t| t.expand_word.as_ref())
        .is_some_and(|ew| ew.iter().any(|&e| e))
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
    let head_no_dash = head.trim_start_matches('-');
    let is_level_literal = (!head_no_dash.is_empty()
        && head_no_dash.chars().all(|c| c.is_ascii_digit()))
        || (head.starts_with('#') && head[1..].chars().all(|c| c.is_ascii_digit()));
    if head.starts_with('$') && !is_level_literal {
        // Dynamic level — pessimistic.
        state.mark_pessimistic();
        return;
    }
    for idx in upvar_local_declaration_indices("upvar", args) {
        if let Some(name) = args.get(idx) {
            state.escape(name);
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

/// Classify `info <subcmd> ...` against the allow-list.
pub fn handle_info(args: &[String], state: &mut EscapeState) {
    let Some(sub) = args.first() else {
        // ``info`` with no subcommand: usage error at runtime —
        // be safe.
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
        // ``info exists name`` reads the name by string lookup —
        // escape it.
        state.escape(target);
        return;
    }
    if is_safe_info_subcommand(sub) {
        return;
    }
    // Unknown subcommand — be conservative.
    state.mark_pessimistic();
}

/// Handle `set` / `incr` / `append` / `lappend` / `unset` whose
/// first arg is a variable name. If the name is a `$n` reference
/// and `n` was assigned exactly one literal identifier earlier,
/// treat the call as targeting that literal (escape just that
/// name). Otherwise fall back to spilling every known proc-local.
pub fn handle_dynamic_name_first(cmd: &str, args: &[String], state: &mut EscapeState) {
    debug_assert!(is_name_first_command(cmd));
    let Some(name) = args.first() else {
        return;
    };
    if !crate::var_escape::helpers::is_dynamic_name(name) {
        return;
    }
    if let Some(literal) = state.resolve_literal(name) {
        state.escape(&literal);
    } else {
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
    fn info_unknown_subcommand_is_pessimistic() {
        let mut s = EscapeState::default();
        handle_info(&args_of(&["totally_made_up"]), &mut s);
        assert!(s.dynamic_barrier());
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
}
