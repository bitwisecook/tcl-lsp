//! Port of `tests/test_command_registry.py` (the C-Tcl-observable parts) — the
//! command-metadata registry. Verifies the Rust registry registers the Tcl core
//! commands and classifies the control-flow / needs-start-cmd traits.
//!
//! C-Tcl proof: the core command set is exactly what tclsh reports via
//! `info commands` — the names below were taken from `info commands {[a-z]*}`
//! on tclsh9.0 (and all exist on tclsh8.6 too).

use tcl_registry::{registry_for_dialect, CommandRegistry, Traits};

/// Commands present in both tclsh8.6 and tclsh9.0 `info commands`.
const CORE_COMMANDS: &[&str] = &[
    "after", "append", "apply", "array", "binary", "break", "catch", "cd", "chan", "clock",
    "close", "concat", "continue", "dict", "encoding", "eof", "error", "eval", "exec", "exit",
    "expr", "fblocked", "fconfigure", "fcopy", "file", "fileevent", "flush", "for", "foreach",
    "format", "gets", "glob", "global", "if", "incr", "info", "interp", "join", "lappend",
    "lassign", "lindex", "linsert", "list", "llength", "lmap", "load", "lrange", "lrepeat",
    "lreplace", "lreverse", "lsearch", "lset", "lsort", "namespace", "open", "package", "pid",
    "proc", "puts", "pwd", "read", "regexp", "regsub", "rename", "return", "scan", "seek", "set",
    "socket", "source", "split", "string", "subst", "switch", "tailcall", "tell", "throw", "time",
    "trace", "try", "unset", "update", "uplevel", "upvar", "variable", "vwait", "while",
];

#[test]
fn registry_covers_tcl_core_commands() {
    let reg = registry_for_dialect("tcl8.6");
    let missing: Vec<&str> = CORE_COMMANDS
        .iter()
        .copied()
        .filter(|c| reg.get(c).is_none())
        .collect();
    assert!(
        missing.is_empty(),
        "registry is missing core Tcl commands (per tclsh `info commands`): {missing:?}"
    );
}

#[test]
fn build_default_is_non_empty_and_iterates() {
    let reg = CommandRegistry::build_default();
    assert!(reg.len() > 0);
    let names: Vec<&str> = reg.command_names().collect();
    assert!(names.contains(&"set"));
    assert!(names.contains(&"proc"));
    // get() round-trips for a known command.
    assert!(reg.get("string").is_some());
    // An unknown command is absent.
    assert!(reg.get("definitely_not_a_command_xyz").is_none());
}

#[test]
fn control_flow_trait_classifies_core_control_commands() {
    let reg = registry_for_dialect("tcl8.6");
    for cmd in ["if", "for", "while", "foreach", "switch"] {
        let spec = reg.get(cmd).unwrap_or_else(|| panic!("{cmd} registered"));
        assert!(
            spec.traits.contains(Traits::CONTROL_FLOW),
            "{cmd} must be CONTROL_FLOW"
        );
    }
    // Plain data commands are not control flow.
    for cmd in ["set", "puts", "incr", "append"] {
        let spec = reg.get(cmd).unwrap();
        assert!(
            !spec.traits.contains(Traits::CONTROL_FLOW),
            "{cmd} must not be CONTROL_FLOW"
        );
    }
}

#[test]
fn needs_start_cmd_trait() {
    let reg = registry_for_dialect("tcl8.6");
    // `expr`, `break`, `continue` require a start-command context.
    for cmd in ["expr", "break", "continue"] {
        let spec = reg.get(cmd).unwrap_or_else(|| panic!("{cmd} registered"));
        assert!(
            spec.traits.contains(Traits::NEEDS_START_CMD),
            "{cmd} must be NEEDS_START_CMD"
        );
    }
    // A typical data command does not.
    assert!(!reg.get("set").unwrap().traits.contains(Traits::NEEDS_START_CMD));
}

#[test]
fn dialect_registries_share_the_tcl_core() {
    // The 8.6 and 9.0 dialect registries both carry the shared core (`set`).
    for d in ["tcl8.6", "tcl9.0"] {
        assert!(registry_for_dialect(d).get("set").is_some(), "{d}");
        assert!(registry_for_dialect(d).get("foreach").is_some(), "{d}");
    }
}

#[test]
fn nine_oh_only_commands_gated_by_dialect() {
    // `const` is a Tcl 9.0 command (absent from 8.6 `info commands`); the
    // registry should reflect that dialect gate.
    let nine = registry_for_dialect("tcl9.0");
    assert!(nine.get("const").is_some(), "const is a 9.0 builtin");
}
