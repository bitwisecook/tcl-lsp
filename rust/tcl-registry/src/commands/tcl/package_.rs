//! `package` — facilities for package loading and version control.

use crate::prelude::*;

static SUBCOMMANDS: &[SubCommand] = &[
    SubCommand {
        name: "files",
        arity: Arity::exact(1),
        detail: "Lists all files forming part of package.",
        synopsis: "package files package",
        pure: true,
        return_type: Some(TclType::List),
        dialects: Some(DialectSet::TCL90),
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "forget",
        arity: Arity::any(),
        detail: "Removes all information about each specified package from this interpreter.",
        synopsis: "package forget ?package package ...?",
        return_type: Some(TclType::String),
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "ifneeded",
        arity: Arity::new(2, 3),
        detail: "Set up or query the package ifneeded script.",
        synopsis: "package ifneeded package version ?script?",
        return_type: Some(TclType::String),
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "names",
        arity: Arity::exact(0),
        detail: "Returns a list of the names of all packages in the interpreter.",
        synopsis: "package names",
        pure: true,
        return_type: Some(TclType::List),
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "prefer",
        arity: Arity::new(0, 1),
        detail: "Returns or sets the current mode of selection logic used by package require.",
        synopsis: "package prefer ?latest|stable?",
        return_type: Some(TclType::String),
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "present",
        arity: Arity::at_least(1),
        detail: "Equivalent to package require except that it does not try to load the package if it is not already loaded.",
        synopsis: "package present ?-exact? package ?requirement...?",
        return_type: Some(TclType::String),
        options: &[OptionSpec {
            name: "-exact",
            takes_value: false,
            value_hint: "",
            detail: "",
        dialects: None,
    }],
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "provide",
        arity: Arity::new(1, 2),
        detail: "Indicates that a version of a package is now present in the interpreter.",
        synopsis: "package provide package ?version?",
        return_type: Some(TclType::String),
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "require",
        arity: Arity::at_least(1),
        detail: "Load a package, finding and sourcing the appropriate script if needed.",
        synopsis: "package require package ?requirement...?",
        return_type: Some(TclType::String),
        options: &[OptionSpec {
            name: "-exact",
            takes_value: false,
            value_hint: "",
            detail: "",
        dialects: None,
    }],
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "unknown",
        arity: Arity::new(0, 1),
        detail: "Supplies a command to invoke during package require if no suitable version can be found.",
        synopsis: "package unknown ?command?",
        return_type: Some(TclType::String),
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "vcompare",
        arity: Arity::exact(2),
        detail: "Compares the two version numbers given by version1 and version2.",
        synopsis: "package vcompare version1 version2",
        pure: true,
        return_type: Some(TclType::Int),
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "versions",
        arity: Arity::exact(1),
        detail: "Returns a list of all the version numbers of package for which information has been provided.",
        synopsis: "package versions package",
        pure: true,
        return_type: Some(TclType::List),
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "vsatisfies",
        arity: Arity::at_least(2),
        detail: "Returns 1 if the version satisfies at least one of the given requirements, and 0 otherwise.",
        synopsis: "package vsatisfies version requirement...",
        pure: true,
        return_type: Some(TclType::Boolean),
        ..SubCommand::DEFAULT
    },
];

/// Command spec for `package`.
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "package",
        traits: Traits::NOT_PROC_FACTORY | Traits::BYTE_COMPILED | Traits::LANGUAGE_KEYWORD,
        arity: Arity::at_least(1),
        subcommands: SUBCOMMANDS,
        side_effects: &[SideEffect {
            target: SideEffectTarget::InterpState,
            reads: false,
            writes: true,
            connection_side: ConnectionSide::None,
        }],
        hover: Some(HoverSnippet::brief(
            "Facilities for package loading and version control.",
            &[
                "package require package ?requirement...?",
                "package provide package ?version?",
                "package names",
            ],
            "Tcl package(1)",
        )),
        ..CommandSpec::DEFAULT
    }
}
