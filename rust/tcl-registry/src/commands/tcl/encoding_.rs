//! `encoding` — manipulate character encodings.
use crate::prelude::*;

const SIDE_EFFECTS: &[SideEffect] = &[SideEffect {
    target: SideEffectTarget::Unknown,
    reads: true,
    writes: false,
    connection_side: ConnectionSide::None,
}];

const OPTIONS: &[OptionSpec] = &[OptionSpec {
    name: "-profile",
    takes_value: true,
    value_hint: "profile",
    detail: "Encoding profile (strict, tcl8, replace).",
    dialects: None,
}];

const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "encoding subcommand ?arg ...?",
}];

static SUBCOMMANDS: &[SubCommand] = &[
    SubCommand {
        name: "convertfrom",
        traits: Traits::TAINT_SOURCE,
        arity: Arity::new(1, 2),
        detail: "Convert from specified encoding.",
        synopsis: "encoding convertfrom ?encoding? data",
        pure: true,
        return_type: Some(TclType::String),
        arg_values: &[(
            0,
            &[
                ArgValue {
                    value: "strict",
                    detail: "Stop on conversion error. Unicode-conformant.",
                },
                ArgValue {
                    value: "tcl8",
                    detail: "Map invalid bytes to equivalent code points. Tcl 8 compatible.",
                },
                ArgValue {
                    value: "replace",
                    detail: "Replace invalid data with U+FFFD. Unicode-conformant.",
                },
            ],
        )],
        is_unescape: true,
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "convertto",
        arity: Arity::new(1, 2),
        detail: "Convert to specified encoding.",
        synopsis: "encoding convertto ?encoding? string",
        pure: true,
        return_type: Some(TclType::ByteArray),
        arg_values: &[(
            0,
            &[
                ArgValue {
                    value: "strict",
                    detail: "Stop on conversion error. Unicode-conformant.",
                },
                ArgValue {
                    value: "tcl8",
                    detail: "Map invalid bytes to equivalent code points. Tcl 8 compatible.",
                },
                ArgValue {
                    value: "replace",
                    detail: "Replace invalid data with U+FFFD. Unicode-conformant.",
                },
            ],
        )],
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "dirs",
        arity: Arity::any(),
        detail: "Manage encoding search path.",
        synopsis: "encoding dirs ?directoryList?",
        return_type: Some(TclType::List),
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "names",
        arity: Arity::exact(0),
        detail: "Return list of available encodings.",
        synopsis: "encoding names",
        pure: true,
        return_type: Some(TclType::List),
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "system",
        arity: Arity::new(0, 1),
        detail: "Get or set system encoding.",
        synopsis: "encoding system ?encoding?",
        return_type: Some(TclType::String),
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "profiles",
        arity: Arity::exact(0),
        // `encoding profiles` was added in Tcl 9.0 — absent in 8.6.x.
        dialects: Some(DialectSet::TCL90_PLUS),
        detail: "Return list of available profiles.",
        synopsis: "encoding profiles",
        pure: true,
        return_type: Some(TclType::List),
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "user",
        arity: Arity::exact(0),
        // `encoding user` was added in Tcl 9.0 — absent in 8.6.x.
        dialects: Some(DialectSet::TCL90_PLUS),
        detail: "Return user encoding.",
        synopsis: "encoding user",
        ..SubCommand::DEFAULT
    },
];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "encoding",
        traits: Traits::BYTE_COMPILED,
        arity: Arity::at_least(1),
        subcommands: SUBCOMMANDS,
        hover: Some(HoverSnippet {
            summary: "Convert between character encodings.",
            synopsis: &[
                "encoding convertfrom ?-profile profile? ?encoding? data",
                "encoding convertto ?-profile profile? ?encoding? string",
                "encoding names",
                "encoding system ?encoding?",
                "encoding subcommand ?arg ...?",
            ],
            snippet: "Use `encoding names` to list available encodings.",
            source: "Tcl man page encoding.n",
            examples: "",
            return_value: "",
        }),
        forms: FORMS,
        options: OPTIONS,
        side_effects: SIDE_EFFECTS,
        ..CommandSpec::DEFAULT
    }
}
