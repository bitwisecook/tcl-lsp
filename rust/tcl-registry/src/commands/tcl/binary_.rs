//! `binary` — manipulate binary data.
use crate::prelude::*;

const SIDE_EFFECTS: &[SideEffect] = &[SideEffect {
    target: SideEffectTarget::Unknown,
    reads: true,
    writes: false,
    connection_side: ConnectionSide::None,
}];

const FORMS: &[FormSpec] = &[
    FormSpec {
        kind: FormKind::Default,
        synopsis: "binary format formatString ?arg arg ...?",
    },
    FormSpec {
        kind: FormKind::Default,
        synopsis: "binary scan string formatString ?varName varName ...?",
    },
    FormSpec {
        kind: FormKind::Default,
        synopsis: "binary encode format ?-option value ...? data",
    },
    FormSpec {
        kind: FormKind::Default,
        synopsis: "binary decode format ?-option value ...? data",
    },
];

static SUBCOMMANDS: &[SubCommand] = &[
    SubCommand {
        name: "decode",
        arity: Arity::at_least(2),
        detail: "Decode binary data.",
        synopsis: "binary decode format data",
        pure: true,
        return_type: Some(TclType::ByteArray),
        arg_types: &[(
            0,
            ArgTypeHint {
                expected: None,
                shimmers: true,
            },
        )],
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "encode",
        arity: Arity::at_least(2),
        detail: "Encode binary data.",
        synopsis: "binary encode format data",
        pure: true,
        return_type: Some(TclType::String),
        arg_types: &[(
            0,
            ArgTypeHint {
                expected: None,
                shimmers: true,
            },
        )],
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "format",
        arity: Arity::at_least(1),
        detail: "Format values into a binary string.",
        synopsis: "binary format formatString ?arg ...?",
        pure: true,
        return_type: Some(TclType::ByteArray),
        arg_types: &[(
            0,
            ArgTypeHint {
                expected: Some(TclType::String),
                shimmers: true,
            },
        )],
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "scan",
        arity: Arity::at_least(2),
        detail: "Parse a binary string.",
        synopsis: "binary scan string formatString ?varName ...?",
        return_type: Some(TclType::Int),
        arg_types: &[
            (
                0,
                ArgTypeHint {
                    expected: None,
                    shimmers: true,
                },
            ),
            (
                1,
                ArgTypeHint {
                    expected: Some(TclType::String),
                    shimmers: true,
                },
            ),
        ],
        arg_roles: &[
            (2, ArgRole::VarWrite),
            (3, ArgRole::VarWrite),
            (4, ArgRole::VarWrite),
            (5, ArgRole::VarWrite),
            (6, ArgRole::VarWrite),
            (7, ArgRole::VarWrite),
            (8, ArgRole::VarWrite),
            (9, ArgRole::VarWrite),
            (10, ArgRole::VarWrite),
            (11, ArgRole::VarWrite),
            (12, ArgRole::VarWrite),
            (13, ArgRole::VarWrite),
            (14, ArgRole::VarWrite),
            (15, ArgRole::VarWrite),
            (16, ArgRole::VarWrite),
            (17, ArgRole::VarWrite),
            (18, ArgRole::VarWrite),
            (19, ArgRole::VarWrite),
        ],
        ..SubCommand::DEFAULT
    },
];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "binary",
        traits: Traits::BYTE_COMPILED | Traits::CSE_CANDIDATE,
        arity: Arity::at_least(1),
        subcommands: SUBCOMMANDS,
hover: Some(HoverSnippet {
    summary: "Manipulate binary data",
    synopsis: &["binary format formatString ?arg arg ...?", "binary scan string formatString ?varName varName ...?", "binary encode format ?-option value ...? data", "binary decode format ?-option value ...? data", "binary subcommand ?arg ...?"],
    snippet: "This command provides facilities for manipulating binary data. The principal operations are inserting values into a binary string and extracting values from a binary string.",
    source: "Tcl man page binary.n",
    examples: "",
    return_value: "",
}),
        forms: FORMS,
        side_effects: SIDE_EFFECTS,
        ..CommandSpec::DEFAULT
    }
}
