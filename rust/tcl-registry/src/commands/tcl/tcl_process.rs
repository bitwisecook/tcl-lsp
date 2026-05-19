//! `tcl::process` — subprocess management ensemble (Tcl 9.0+, TIP 463).
//!
//! Pure-host capability — never reachable in WASM/WASI builds (no
//! process model), so the registration exists for LSP recognition
//! and the runtime drops it on the trapping-stub list.  Two name
//! forms are registered for the namespace-relative and fully-
//! qualified spellings.  Mirrors the Python
//! `core/commands/registry/tcl/tcl_process.py` PR #433 spec.

use crate::prelude::*;

fn make_spec(name: &'static str) -> CommandSpec {
    CommandSpec {
        name,
        dialects: Some(DialectSet::TCL90),
        arity: Arity::at_least(1),
        subcommands: &SUBCOMMANDS,
        return_type: Some(TclType::String),
        side_effects: &[SideEffect {
            target: SideEffectTarget::Unknown,
            reads: true,
            writes: true,
            connection_side: ConnectionSide::None,
        }],
        hover: Some(HoverSnippet::brief(
            "Manage subprocesses started by exec / open |cmd.",
            &["tcl::process subcommand ?arg ...?"],
            "Tcl man page process.n",
        )),
        ..CommandSpec::DEFAULT
    }
}

static STATUS_OPTIONS: [OptionSpec; 2] = [
    OptionSpec {
        name: "-wait",
        takes_value: false,
        value_hint: "",
        detail: "Block until status is available.",
        dialects: None,
    },
    OptionSpec {
        name: "--",
        takes_value: false,
        value_hint: "",
        detail: "Marks end of switches.",
        dialects: None,
    },
];

static SUBCOMMANDS: [SubCommand; 4] = [
    SubCommand {
        name: "autopurge",
        arity: Arity::new(0, 1),
        detail: "Query (no args) or set (boolean) the autopurge flag.",
        synopsis: "tcl::process autopurge ?flag?",
        return_type: Some(TclType::Boolean),
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "list",
        arity: Arity::new(0, 0),
        detail: "Return list of subprocess PIDs.",
        synopsis: "tcl::process list",
        return_type: Some(TclType::List),
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "purge",
        arity: Arity::new(0, 1),
        detail: "Purge terminated subprocesses (optionally filtered by PID list).",
        synopsis: "tcl::process purge ?pids?",
        return_type: Some(TclType::String),
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "status",
        arity: Arity::at_least(0),
        detail: "Return dict mapping PID -> status. `-wait` blocks until status available.",
        synopsis: "tcl::process status ?switches? ?pids?",
        return_type: Some(TclType::Dict),
        options: &STATUS_OPTIONS,
        ..SubCommand::DEFAULT
    },
];

/// Command spec for the namespace-relative form `tcl::process`.
pub fn spec() -> CommandSpec {
    make_spec("tcl::process")
}

/// Command spec for the fully-qualified form `::tcl::process`.
pub fn spec_qualified() -> CommandSpec {
    make_spec("::tcl::process")
}
