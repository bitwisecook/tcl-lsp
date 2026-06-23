//! `uuid::uuid` command.
use crate::prelude::*;

/// The command's subcommands.
const SUBCOMMANDS: &[SubCommand] = &[
    SubCommand {
        name: "equal",
        arity: Arity::exact(2),
        detail: "",
        synopsis: "",
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "generate",
        arity: Arity::exact(0),
        detail: "",
        synopsis: "",
        ..SubCommand::DEFAULT
    },
];

const SIDE_EFFECTS: &[SideEffect] = &[SideEffect {
    target: SideEffectTarget::Unknown,
    reads: true,
    writes: false,
    connection_side: ConnectionSide::None,
}];

const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "uuid::uuid subcommand ?args?",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "uuid::uuid",
        dialects: None,
        arity: Arity::at_least(1),
        hover: Some(HoverSnippet {
            summary: "Generate or manipulate UUIDs.",
            synopsis: &["uuid::uuid generate", "uuid::uuid equal id1 id2"],
            snippet: "Generate RFC 4122 version 4 universally unique identifiers.",
            source: "tcllib uuid package",
            examples: "set id [uuid::uuid generate]",
            return_value: "A UUID string.",
        }),
        forms: FORMS,
        side_effects: SIDE_EFFECTS,
        subcommands: SUBCOMMANDS,
        tcllib_package: Some("uuid"),
        ..CommandSpec::DEFAULT
    }
}
