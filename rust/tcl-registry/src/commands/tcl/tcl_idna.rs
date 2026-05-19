//! `tcl::idna` — Internationalised Domain Name (IDNA/Punycode) helpers
//! (Tcl 9.0+, bundled package).
//!
//! Two name forms are registered because Tcl callers may use the
//! namespace-relative spelling `tcl::idna` (inside `namespace eval
//! ::tcl`) or the fully-qualified `::tcl::idna`.  Mirrors the Python
//! `core/commands/registry/tcl/tcl_idna.py` PR #433 spec.

use crate::prelude::*;

fn make_spec(name: &'static str) -> CommandSpec {
    CommandSpec {
        name,
        dialects: Some(DialectSet::TCL90),
        required_package: Some("tcl::idna"),
        arity: Arity::at_least(1),
        subcommands: &SUBCOMMANDS,
        return_type: Some(TclType::String),
        hover: Some(HoverSnippet::brief(
            "Internationalised Domain Name (IDNA/Punycode) helpers.",
            &["tcl::idna subcommand ?arg ...?"],
            "Tcl man page idna.n",
        )),
        ..CommandSpec::DEFAULT
    }
}

static SUBCOMMANDS: [SubCommand; 4] = [
    SubCommand {
        name: "decode",
        arity: Arity::new(1, 1),
        detail: "Decode punycode in a hostname for display.",
        synopsis: "tcl::idna decode hostname",
        return_type: Some(TclType::String),
        pure: true,
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "encode",
        arity: Arity::new(1, 1),
        detail: "Encode a hostname with punycode where needed.",
        synopsis: "tcl::idna encode hostname",
        return_type: Some(TclType::String),
        pure: true,
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "puny",
        arity: Arity::new(2, 3),
        detail: "Direct access to the punycode encoder/decoder.",
        synopsis: "tcl::idna puny decode|encode string ?case?",
        return_type: Some(TclType::String),
        pure: true,
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "version",
        arity: Arity::new(0, 0),
        detail: "Return the tcl::idna package version.",
        synopsis: "tcl::idna version",
        return_type: Some(TclType::String),
        pure: true,
        ..SubCommand::DEFAULT
    },
];

/// Command spec for the namespace-relative form `tcl::idna`.
pub fn spec() -> CommandSpec {
    make_spec("tcl::idna")
}

/// Command spec for the fully-qualified form `::tcl::idna`.
pub fn spec_qualified() -> CommandSpec {
    make_spec("::tcl::idna")
}
