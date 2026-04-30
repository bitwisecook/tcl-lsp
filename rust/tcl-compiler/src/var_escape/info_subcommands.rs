//! Allow-list of `info` subcommands for the var-escape analysis (C33c).
//!
//! Audited against Tcl 9.0's `info` dispatch table (`tclCmdIL.c`).
//! Most `info` subcommands reflect on the interpreter's global
//! state (command table, version info, script path). A small set
//! reads the current frame by name — those force the whole proc
//! to the pessimistic fallback. `info exists <literal>` is treated
//! specially: safe if the target is a bare name (escapes only that
//! name), pessimistic if dynamic.
//!
//! Mirrors `core/compiler/var_escape/_info_subcommands.py`.

/// Subcommands that read frame-local state by name and thus force
/// the containing proc to the pessimistic fallback.
pub const FRAME_INSPECTING_SUBCOMMANDS: &[&str] = &[
    "level",      // Returns caller frame args or a depth int.
    "frame",      // Returns a frame descriptor dict (file, line, cmd).
    "vars",       // Enumerates vars visible in the current frame.
    "locals",     // Enumerates locals of the current frame.
    "coroutine",  // Exposes the current coroutine frame.
    "errorstack", // Exposes the error callstack — caller-frame data.
];

/// Subcommands that are purely interpreter-global — safe.
pub const INTERPRETER_GLOBAL_SUBCOMMANDS: &[&str] = &[
    // Proc / method introspection (reads the global proc table,
    // not the current frame).
    "body",
    "args",
    "default",
    "commands",
    "procs",
    "class",
    "functions",
    "cmdtype",
    // Version / build / environment.
    "patchlevel",
    "tclversion",
    "nameofexecutable",
    "sharedlibextension",
    "library",
    "hostname",
    "script",
    // Runtime bookkeeping — no frame access.
    "cmdcount",
    "complete",
    "cancel",
    "loaded",
    // TclOO object introspection — reads object metadata.
    "object",
    // Tcl 9 additions: constants and globals expose data by name
    // but not via the current proc frame.
    "constant",
    "constants",
    "globals",
    // Handled separately at call sites — listed for completeness.
    "exists",
];

/// Return true if this `info <subcmd>` never needs frame access.
#[must_use]
pub fn is_safe_info_subcommand(subcmd: &str) -> bool {
    INTERPRETER_GLOBAL_SUBCOMMANDS.contains(&subcmd)
}

/// Return true if `info <subcmd>` enumerates or reads frame state.
#[must_use]
pub fn is_frame_inspecting_info_subcommand(subcmd: &str) -> bool {
    FRAME_INSPECTING_SUBCOMMANDS.contains(&subcmd)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn frame_inspecting_set_is_pessimistic() {
        assert!(is_frame_inspecting_info_subcommand("level"));
        assert!(is_frame_inspecting_info_subcommand("frame"));
        assert!(is_frame_inspecting_info_subcommand("vars"));
        assert!(is_frame_inspecting_info_subcommand("locals"));
        assert!(is_frame_inspecting_info_subcommand("coroutine"));
        assert!(is_frame_inspecting_info_subcommand("errorstack"));
    }

    #[test]
    fn interpreter_global_set_is_safe() {
        for sub in [
            "body",
            "args",
            "commands",
            "procs",
            "tclversion",
            "patchlevel",
            "loaded",
            "exists",
            "globals",
            "constants",
        ] {
            assert!(is_safe_info_subcommand(sub), "{sub} should be safe");
        }
    }

    #[test]
    fn unknown_subcommand_neither_safe_nor_frame_inspecting() {
        assert!(!is_safe_info_subcommand("totally_made_up"));
        assert!(!is_frame_inspecting_info_subcommand("totally_made_up"));
    }

    #[test]
    fn frame_and_safe_sets_are_disjoint() {
        for f in FRAME_INSPECTING_SUBCOMMANDS {
            assert!(
                !INTERPRETER_GLOBAL_SUBCOMMANDS.contains(f),
                "{f} appears in both sets"
            );
        }
    }
}
