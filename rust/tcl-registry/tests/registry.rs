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

//! Port of `tests/test_command_registry.py` (the C-Tcl-observable parts) — the
//! command-metadata registry. Verifies the Rust registry registers the Tcl core
//! commands and classifies the control-flow / needs-start-cmd traits.
//!
//! C-Tcl proof: the core command set is exactly what tclsh reports via
//! `info commands` — the names below were taken from `info commands {[a-z]*}`
//! on tclsh9.0 (and all exist on tclsh8.6 too).

use tcl_registry::dialects::DialectSet;
use tcl_registry::{CommandRegistry, TclType, Traits, VarWriteTyping, registry_for_dialect};

/// Commands present in both tclsh8.6 and tclsh9.0 `info commands`.
const CORE_COMMANDS: &[&str] = &[
    "after",
    "append",
    "apply",
    "array",
    "binary",
    "break",
    "catch",
    "cd",
    "chan",
    "clock",
    "close",
    "concat",
    "continue",
    "dict",
    "encoding",
    "eof",
    "error",
    "eval",
    "exec",
    "exit",
    "expr",
    "fblocked",
    "fconfigure",
    "fcopy",
    "file",
    "fileevent",
    "flush",
    "for",
    "foreach",
    "format",
    "gets",
    "glob",
    "global",
    "if",
    "incr",
    "info",
    "interp",
    "join",
    "lappend",
    "lassign",
    "lindex",
    "linsert",
    "list",
    "llength",
    "lmap",
    "load",
    "lrange",
    "lrepeat",
    "lreplace",
    "lreverse",
    "lsearch",
    "lset",
    "lsort",
    "namespace",
    "open",
    "package",
    "pid",
    "proc",
    "puts",
    "pwd",
    "read",
    "regexp",
    "regsub",
    "rename",
    "return",
    "scan",
    "seek",
    "set",
    "socket",
    "source",
    "split",
    "string",
    "subst",
    "switch",
    "tailcall",
    "tell",
    "throw",
    "time",
    "trace",
    "try",
    "unset",
    "update",
    "uplevel",
    "upvar",
    "variable",
    "vwait",
    "while",
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
    assert!(!reg.is_empty());
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
    assert!(
        !reg.get("set")
            .unwrap()
            .traits
            .contains(Traits::NEEDS_START_CMD)
    );
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

#[test]
fn var_write_typing_declares_destructuring_writers() {
    // The registry is the single source of truth for how a command types the
    // variables it *writes* — distinct from what it *returns* (issue #867).
    // A destructuring writer returns a leftover list / match count while
    // writing element-wise pieces, so the compiler must read this rather than
    // broadcast the return type onto the targets.
    let reg = CommandRegistry::build_default();

    let resolve = |name: &str, args: &[&str]| {
        reg.resolve_call(name, args, DialectSet::empty())
            .unwrap_or_else(|| panic!("{name} resolves"))
            .var_write_typing()
    };

    // Destructuring writers widen their targets to "unknown intrep": a list
    // element (`lassign`) or a format-dependent conversion (`scan`), and
    // `regexp` capture fragments.
    for (name, args) in [
        ("lassign", &["$l", "a"][..]),
        ("scan", &["$s", "%d", "a"][..]),
        ("regexp", &["re", "$s", "m"][..]),
    ] {
        assert_eq!(
            resolve(name, args),
            VarWriteTyping::Destructured,
            "{name} must declare Destructured var-write typing"
        );
    }

    // `regsub`'s `varName` form writes one whole substituted *string* (not a
    // format-/element-dependent piece), so it is `Fixed(String)` — keeping real
    // string-in-arithmetic shimmer diagnostics rather than widening away.
    assert_eq!(
        resolve("regsub", &["re", "$s", "sub", "out"]),
        VarWriteTyping::Fixed(TclType::String),
        "regsub writes a substituted String"
    );

    // `binary scan` carries the typing at the *subcommand* level; the bare
    // `binary` command (and `binary format`) does not destructure.
    assert_eq!(
        resolve("binary", &["scan", "$d", "a3", "chars"]),
        VarWriteTyping::Destructured,
        "binary scan must declare Destructured var-write typing"
    );
    assert_eq!(
        resolve("binary", &["format", "a3", "abc"]),
        VarWriteTyping::ReturnValue,
        "binary format does not destructure"
    );

    // Fixed-intrep writers: the line `gets` reads is a String; the list `lpop`
    // leaves behind is a List — neither is the value the command returns.
    assert_eq!(
        resolve("gets", &["$ch", "line"]),
        VarWriteTyping::Fixed(TclType::String),
        "gets writes a String line"
    );
    assert_eq!(
        registry_for_dialect("tcl9.0")
            .resolve_call("lpop", &["l"], DialectSet::empty())
            .expect("lpop resolves on 9.0")
            .var_write_typing(),
        VarWriteTyping::Fixed(TclType::List),
        "lpop leaves a List in its variable"
    );

    // Control: an ordinary single-target writer keeps the default — its stored
    // value IS its return value.
    for name in ["append", "lappend"] {
        assert_eq!(
            resolve(name, &["v", "x"]),
            VarWriteTyping::ReturnValue,
            "{name} stores its return value — default ReturnValue"
        );
    }
}
