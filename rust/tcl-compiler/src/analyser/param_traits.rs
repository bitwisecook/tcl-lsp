//! Proc argument trait inference — Rust port of
//! ``core/analysis/proc_arg_traits.py::infer_param_traits``.
//!
//! Walks a proc body to determine how each parameter is used:
//!
//! - `Eval` — passed to ``eval`` / ``uplevel`` / ``subst``
//! - `Body` — used as a loop / control body
//! - `VarWrite` — names a variable the proc writes (upvar +
//!   ``set`` / ``incr`` / ``append`` / ``lappend``, or a
//!   registry-marked variable-write site)
//! - `VarRead` — names a variable the proc reads via ``upvar``
//! - `Expr` — evaluated as an expression
//! - `LoopList` — used as the list arg in ``foreach`` / ``lmap``
//!
//! This is the **shallow** pass — top-level command scan only.
//! Mirrors Python's ``infer_param_traits``; the
//! ``infer_param_traits_deep`` recursive descent stays Python-only
//! for now (callers who need deep traits still go through the
//! Python supplement merge).

use std::collections::{HashMap, HashSet};

use tcl_registry::arg_role::ArgRole;
use tcl_registry::CommandRegistry;

use super::types::ProcArgTrait;
use crate::segmenter::segment_commands;

/// Top-level shallow trait inference.  Returns a map from
/// parameter name to a set of inferred traits.  Empty entries
/// (parameters with no detected trait) are dropped from the
/// returned map.
#[must_use]
pub fn infer_param_traits(
    params: &[&str],
    body_source: &str,
) -> HashMap<String, HashSet<ProcArgTrait>> {
    if params.is_empty() || body_source.trim().is_empty() {
        return HashMap::new();
    }
    let param_set: HashSet<&str> = params.iter().copied().collect();
    let mut traits: HashMap<&str, HashSet<ProcArgTrait>> =
        params.iter().map(|p| (*p, HashSet::new())).collect();
    let mut upvar_aliases: HashMap<String, &str> = HashMap::new();
    let registry = CommandRegistry::build_default();

    let commands = extract_commands(body_source);
    for (cmd_name, cmd_args) in &commands {
        scan_command(
            cmd_name,
            cmd_args,
            &param_set,
            &mut traits,
            &mut upvar_aliases,
            &registry,
        );
    }

    traits
        .into_iter()
        .filter(|(_, set)| !set.is_empty())
        .map(|(k, v)| (k.to_string(), v))
        .collect()
}

/// Extract `(command, args)` pairs from `source` via the
/// segmenter.  Mirrors ``_extract_commands`` in
/// ``core/analysis/proc_arg_traits.py:78-95``.
fn extract_commands(source: &str) -> Vec<(String, Vec<String>)> {
    let mut commands = Vec::new();
    let segments = segment_commands(source);
    for seg in segments {
        if seg.texts.is_empty() {
            continue;
        }
        let cmd_name = seg.texts[0].clone();
        let cmd_args: Vec<String> = seg.texts[1..].to_vec();
        commands.push((cmd_name, cmd_args));
    }
    commands
}

/// Extract a bare variable name from ``$var`` or ``${var}``.
/// Returns `None` when the text isn't a simple variable
/// reference.  Mirrors ``_extract_var_name`` in Python.
fn extract_var_name(text: &str) -> Option<&str> {
    let bytes = text.as_bytes();
    if bytes.len() < 2 || bytes[0] != b'$' {
        return None;
    }
    let (name_start, name_end) = if bytes[1] == b'{' {
        // ``${name}`` — find the closing ``}``.
        let close = text[2..].find('}')?;
        (2, 2 + close)
    } else {
        (1, bytes.len())
    };
    let name = &text[name_start..name_end];
    if name.is_empty() {
        return None;
    }
    // Verify identifier-like content (alphanumerics, underscore,
    // colons for namespace-qualified names — matches the Python
    // ``_SIMPLE_VAR_RE`` regex shape).
    let mut iter = name.chars();
    let first = iter.next().unwrap();
    if !first.is_ascii_alphabetic() && first != '_' {
        return None;
    }
    if !iter.all(|c| c.is_ascii_alphanumeric() || c == '_' || c == ':') {
        return None;
    }
    // Reject anything past the closing ``}`` for the braced form.
    if bytes[1] == b'{' && name_end + 1 < bytes.len() {
        return None;
    }
    Some(name)
}

/// Resolve a command's per-arg roles via the registry.  Mirrors
/// ``_resolve_arg_roles`` in Python — picks the
/// `arg_role_resolver` callback first, then static
/// `arg_roles`, then sub-command-level roles.
fn resolve_arg_roles(
    command: &str,
    args: &[String],
    registry: &CommandRegistry,
) -> HashMap<u8, ArgRole> {
    let mut roles: HashMap<u8, ArgRole> = HashMap::new();
    let arg_strs: Vec<&str> = args.iter().map(String::as_str).collect();
    for role in [
        ArgRole::Body,
        ArgRole::Expr,
        ArgRole::VarWrite,
        ArgRole::VarRead,
    ] {
        for idx in registry.arg_indices_for_role(command, &arg_strs, role) {
            if let Ok(idx_u8) = u8::try_from(idx) {
                roles.insert(idx_u8, role);
            }
        }
    }
    roles
}

#[allow(clippy::too_many_arguments)]
fn scan_command<'a>(
    cmd_name: &str,
    cmd_args: &'a [String],
    param_set: &HashSet<&'a str>,
    traits: &mut HashMap<&'a str, HashSet<ProcArgTrait>>,
    upvar_aliases: &mut HashMap<String, &'a str>,
    registry: &CommandRegistry,
) {
    let arg_roles = resolve_arg_roles(cmd_name, cmd_args, registry);

    // Per-arg role-driven trait recording.
    for (idx, arg) in cmd_args.iter().enumerate() {
        let Some(var_name) = extract_var_name(arg) else {
            continue;
        };
        let source_param = if let Some(p) = param_set.get(var_name) {
            *p
        } else if let Some(alias) = upvar_aliases.get(var_name) {
            *alias
        } else {
            continue;
        };
        let Ok(idx_u8) = u8::try_from(idx) else {
            continue;
        };
        match arg_roles.get(&idx_u8) {
            Some(ArgRole::Body) => {
                traits
                    .get_mut(source_param)
                    .map(|s| s.insert(ProcArgTrait::Body));
            }
            Some(ArgRole::Expr) => {
                traits
                    .get_mut(source_param)
                    .map(|s| s.insert(ProcArgTrait::Expr));
            }
            Some(ArgRole::VarWrite) => {
                traits
                    .get_mut(source_param)
                    .map(|s| s.insert(ProcArgTrait::VarWrite));
            }
            Some(ArgRole::VarRead) => {
                traits
                    .get_mut(source_param)
                    .map(|s| s.insert(ProcArgTrait::VarRead));
            }
            _ => {}
        }
    }

    // Code-evaluating commands — eval / uplevel / subst.
    // Mirrors Python's ``spec.evaluates_code`` /
    // ``spec.performs_substitution`` branch.  Done by name to
    // avoid a registry-traits round-trip for the common cases.
    match cmd_name {
        "eval" | "subst" => {
            for arg in cmd_args {
                if let Some(vn) = extract_var_name(arg) {
                    if param_set.contains(vn) {
                        if let Some(set) = traits.get_mut(vn) {
                            set.insert(ProcArgTrait::Eval);
                        }
                    }
                }
            }
        }
        "uplevel" => {
            // ``uplevel ?level? script`` — last arg is the script.
            if let Some(last) = cmd_args.last() {
                if let Some(vn) = extract_var_name(last) {
                    if param_set.contains(vn) {
                        if let Some(set) = traits.get_mut(vn) {
                            set.insert(ProcArgTrait::Eval);
                        }
                    }
                }
            }
        }
        _ => {}
    }

    // Per-command structural handlers — mirror the Python
    // ``_handle_*`` functions.
    match cmd_name {
        "upvar" => handle_upvar(cmd_args, param_set, traits, upvar_aliases),
        "foreach" | "lmap" => handle_foreach(cmd_args, param_set, traits),
        "while" => handle_while(cmd_args, param_set, traits),
        "for" => handle_for(cmd_args, param_set, traits),
        "after" => handle_after(cmd_args, param_set, traits),
        "scan" => handle_variadic_var_write(cmd_args, param_set, traits, 2),
        "lassign" => handle_variadic_var_write(cmd_args, param_set, traits, 1),
        "regexp" => handle_regexp_vars(cmd_args, param_set, traits),
        "regsub" => handle_regsub_var(cmd_args, param_set, traits),
        _ => {}
    }

    // Variable-writing commands where param is used as var name.
    // Mirrors the ``vwc = _var_write_commands()`` branch in
    // Python.  We restrict the registry hop to the canonical
    // commands the Python version covers by name to keep the
    // port tight.
    if let Some(idx) = var_write_index(cmd_name) {
        if idx < cmd_args.len() {
            if let Some(vn) = extract_var_name(&cmd_args[idx]) {
                if param_set.contains(vn) {
                    if let Some(set) = traits.get_mut(vn) {
                        set.insert(ProcArgTrait::VarWrite);
                    }
                }
            }
        }
    }

    // Track writes through upvar aliases — ``set local …`` where
    // ``local`` was registered as an alias for some param.
    if matches!(cmd_name, "set" | "incr" | "append" | "lappend") && !cmd_args.is_empty() {
        if let Some(target) = upvar_aliases.get(cmd_args[0].as_str()) {
            if let Some(set) = traits.get_mut(target) {
                set.insert(ProcArgTrait::VarWrite);
            }
        }
    }

    // foreach / lmap loop variables write through aliases.
    if matches!(cmd_name, "foreach" | "lmap") && cmd_args.len() >= 3 {
        let remaining = &cmd_args[..cmd_args.len() - 1];
        let mut i = 0;
        while i < remaining.len() {
            if let Some(target) = upvar_aliases.get(remaining[i].as_str()) {
                if let Some(set) = traits.get_mut(target) {
                    set.insert(ProcArgTrait::VarWrite);
                }
            }
            i += 2;
        }
    }
}

/// Return the var-write argument index for the canonical
/// variable-writing commands tracked by Python's
/// ``variable_writing_commands`` helper.  Skipping commands with
/// sub-commands here is fine because those go through
/// ``resolve_arg_roles`` (which picks the sub-command's role) at
/// the per-arg loop above.
fn var_write_index(cmd_name: &str) -> Option<usize> {
    match cmd_name {
        "set" | "incr" | "append" | "lappend" => Some(0),
        "global" => Some(0),
        "variable" => Some(0),
        _ => None,
    }
}

fn handle_upvar<'a>(
    args: &'a [String],
    param_set: &HashSet<&'a str>,
    traits: &mut HashMap<&'a str, HashSet<ProcArgTrait>>,
    upvar_aliases: &mut HashMap<String, &'a str>,
) {
    let mut start = 0usize;
    if !args.is_empty() {
        let head = args[0].as_str();
        if head.chars().all(|c| c.is_ascii_digit()) || head.starts_with('#') {
            start = 1;
        }
    }
    let mut i = start;
    while i + 1 < args.len() {
        let other_var = &args[i];
        let my_var = &args[i + 1];
        i += 2;

        if let Some(other_vn) = extract_var_name(other_var) {
            if let Some(p) = param_set.get(other_vn).copied() {
                if let Some(set) = traits.get_mut(p) {
                    set.insert(ProcArgTrait::VarRead);
                }
                upvar_aliases.insert(my_var.clone(), p);
            }
        }
        if let Some(my_vn) = extract_var_name(my_var) {
            if let Some(p) = param_set.get(my_vn).copied() {
                if let Some(set) = traits.get_mut(p) {
                    set.insert(ProcArgTrait::VarWrite);
                }
            }
        }
    }
}

fn handle_foreach<'a>(
    args: &[String],
    param_set: &HashSet<&'a str>,
    traits: &mut HashMap<&'a str, HashSet<ProcArgTrait>>,
) {
    if args.len() < 3 {
        return;
    }
    if let Some(body_vn) = extract_var_name(args.last().unwrap()) {
        if let Some(p) = param_set.get(body_vn).copied() {
            if let Some(set) = traits.get_mut(p) {
                set.insert(ProcArgTrait::Body);
            }
        }
    }
    let remaining = &args[..args.len() - 1];
    let mut i = 0;
    while i + 1 < remaining.len() {
        if let Some(list_vn) = extract_var_name(&remaining[i + 1]) {
            if let Some(p) = param_set.get(list_vn).copied() {
                if let Some(set) = traits.get_mut(p) {
                    set.insert(ProcArgTrait::LoopList);
                }
            }
        }
        i += 2;
    }
}

fn handle_while<'a>(
    args: &[String],
    param_set: &HashSet<&'a str>,
    traits: &mut HashMap<&'a str, HashSet<ProcArgTrait>>,
) {
    if args.len() < 2 {
        return;
    }
    if let Some(vn) = extract_var_name(&args[0]) {
        if let Some(p) = param_set.get(vn).copied() {
            if let Some(set) = traits.get_mut(p) {
                set.insert(ProcArgTrait::Expr);
            }
        }
    }
    if let Some(vn) = extract_var_name(&args[1]) {
        if let Some(p) = param_set.get(vn).copied() {
            if let Some(set) = traits.get_mut(p) {
                set.insert(ProcArgTrait::Body);
            }
        }
    }
}

fn handle_for<'a>(
    args: &[String],
    param_set: &HashSet<&'a str>,
    traits: &mut HashMap<&'a str, HashSet<ProcArgTrait>>,
) {
    if args.len() < 4 {
        return;
    }
    let pairs = [
        (&args[0], ProcArgTrait::Body),
        (&args[1], ProcArgTrait::Expr),
        (&args[2], ProcArgTrait::Body),
        (&args[3], ProcArgTrait::Body),
    ];
    for (arg, trait_) in pairs {
        if let Some(vn) = extract_var_name(arg) {
            if let Some(p) = param_set.get(vn).copied() {
                if let Some(set) = traits.get_mut(p) {
                    set.insert(trait_);
                }
            }
        }
    }
}

fn handle_after<'a>(
    args: &[String],
    param_set: &HashSet<&'a str>,
    traits: &mut HashMap<&'a str, HashSet<ProcArgTrait>>,
) {
    if args.len() < 2 {
        return;
    }
    if matches!(args[0].as_str(), "cancel" | "info") {
        return;
    }
    let mut start = 1usize;
    if start < args.len() && args[start] == "-periodic" {
        start += 1;
    }
    for arg in &args[start..] {
        if let Some(vn) = extract_var_name(arg) {
            if let Some(p) = param_set.get(vn).copied() {
                if let Some(set) = traits.get_mut(p) {
                    set.insert(ProcArgTrait::Eval);
                }
            }
        }
    }
}

fn handle_variadic_var_write<'a>(
    args: &[String],
    param_set: &HashSet<&'a str>,
    traits: &mut HashMap<&'a str, HashSet<ProcArgTrait>>,
    start: usize,
) {
    for arg in &args[start.min(args.len())..] {
        if let Some(vn) = extract_var_name(arg) {
            if let Some(p) = param_set.get(vn).copied() {
                if let Some(set) = traits.get_mut(p) {
                    set.insert(ProcArgTrait::VarWrite);
                }
            }
        }
    }
}

const REGEXP_SWITCHES: &[&str] = &[
    "-nocase",
    "-expanded",
    "-line",
    "-linestop",
    "-lineanchor",
    "-all",
    "-inline",
    "-indices",
    "--",
];
const REGEXP_VALUE_SWITCHES: &[&str] = &["-start"];

fn skip_regexp_switches(args: &[String]) -> usize {
    let mut i = 0usize;
    while i < args.len() {
        if args[i] == "--" {
            return i + 1;
        }
        if REGEXP_SWITCHES.iter().any(|s| *s == args[i].as_str()) {
            i += 1;
        } else if REGEXP_VALUE_SWITCHES.iter().any(|s| *s == args[i].as_str()) {
            i += 2;
        } else {
            break;
        }
    }
    i
}

fn handle_regexp_vars<'a>(
    args: &[String],
    param_set: &HashSet<&'a str>,
    traits: &mut HashMap<&'a str, HashSet<ProcArgTrait>>,
) {
    let pos = skip_regexp_switches(args);
    let var_start = pos + 2;
    handle_variadic_var_write(args, param_set, traits, var_start);
}

fn handle_regsub_var<'a>(
    args: &[String],
    param_set: &HashSet<&'a str>,
    traits: &mut HashMap<&'a str, HashSet<ProcArgTrait>>,
) {
    let pos = skip_regexp_switches(args);
    let var_idx = pos + 3;
    if var_idx < args.len() {
        if let Some(vn) = extract_var_name(&args[var_idx]) {
            if let Some(p) = param_set.get(vn).copied() {
                if let Some(set) = traits.get_mut(p) {
                    set.insert(ProcArgTrait::VarWrite);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn assert_trait(
        traits: &HashMap<String, HashSet<ProcArgTrait>>,
        param: &str,
        expected: ProcArgTrait,
    ) {
        let set = traits
            .get(param)
            .unwrap_or_else(|| panic!("no traits for {param}"));
        assert!(
            set.contains(&expected),
            "{param}: expected {expected:?}, got {set:?}",
        );
    }

    #[test]
    fn extract_var_name_simple() {
        assert_eq!(extract_var_name("$foo"), Some("foo"));
        assert_eq!(extract_var_name("${foo}"), Some("foo"));
        assert_eq!(extract_var_name("foo"), None);
        assert_eq!(extract_var_name("$"), None);
        assert_eq!(extract_var_name("$1abc"), None);
    }

    #[test]
    fn eval_param_records_eval_trait() {
        let traits = infer_param_traits(&["body"], "eval $body");
        assert_trait(&traits, "body", ProcArgTrait::Eval);
    }

    #[test]
    fn uplevel_records_eval_on_last_arg_only() {
        let traits = infer_param_traits(&["lvl", "script"], "uplevel $lvl $script");
        assert_trait(&traits, "script", ProcArgTrait::Eval);
        assert!(!traits
            .get("lvl")
            .is_some_and(|s| s.contains(&ProcArgTrait::Eval)));
    }

    #[test]
    fn foreach_records_loop_list_and_body() {
        let traits = infer_param_traits(&["items", "body"], "foreach x $items $body");
        assert_trait(&traits, "items", ProcArgTrait::LoopList);
        assert_trait(&traits, "body", ProcArgTrait::Body);
    }

    #[test]
    fn while_records_expr_and_body() {
        let traits = infer_param_traits(&["cond", "body"], "while $cond $body");
        assert_trait(&traits, "cond", ProcArgTrait::Expr);
        assert_trait(&traits, "body", ProcArgTrait::Body);
    }

    #[test]
    fn for_records_init_cond_next_body() {
        let traits = infer_param_traits(&["i", "c", "n", "b"], "for $i $c $n $b");
        assert_trait(&traits, "i", ProcArgTrait::Body);
        assert_trait(&traits, "c", ProcArgTrait::Expr);
        assert_trait(&traits, "n", ProcArgTrait::Body);
        assert_trait(&traits, "b", ProcArgTrait::Body);
    }

    #[test]
    fn upvar_records_var_read_and_aliases_writes() {
        let traits = infer_param_traits(&["var"], "upvar 1 $var local\nset local 1");
        assert_trait(&traits, "var", ProcArgTrait::VarRead);
        // Write through the alias upgrades to VarWrite.
        assert_trait(&traits, "var", ProcArgTrait::VarWrite);
    }

    #[test]
    fn lassign_records_var_writes() {
        let traits = infer_param_traits(&["a", "b"], "lassign {1 2} $a $b");
        assert_trait(&traits, "a", ProcArgTrait::VarWrite);
        assert_trait(&traits, "b", ProcArgTrait::VarWrite);
    }

    #[test]
    fn after_records_eval_skipping_cancel_info() {
        let traits = infer_param_traits(&["body"], "after 100 $body");
        assert_trait(&traits, "body", ProcArgTrait::Eval);
        let traits = infer_param_traits(&["x"], "after cancel $x");
        // ``after cancel`` doesn't take a script, so $x is not eval.
        assert!(traits
            .get("x")
            .is_none_or(|s| !s.contains(&ProcArgTrait::Eval)));
    }

    #[test]
    fn regsub_records_var_write() {
        let traits = infer_param_traits(&["out"], "regsub -all foo $line bar $out");
        assert_trait(&traits, "out", ProcArgTrait::VarWrite);
    }

    #[test]
    fn empty_body_returns_empty_map() {
        let traits = infer_param_traits(&["a"], "");
        assert!(traits.is_empty());
    }
}
