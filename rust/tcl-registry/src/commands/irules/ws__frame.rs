//! `WS::frame` iRules command.
use crate::prelude::*;

/// Subcommands ported from the Python source of truth.
const SUBCOMMANDS: &[SubCommand] = &[
    SubCommand {
        name: "eom",
        arity: Arity::exact(0),
        detail: "Check if frame is end of message.",
        synopsis: "WS::frame eom",
        pure: true,
        side_effects: &[SideEffect {
            target: SideEffectTarget::NetworkIo,
            reads: true,
            writes: false,
            connection_side: ConnectionSide::Both,
        }],
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "drop",
        arity: Arity::exact(0),
        detail: "Drop the current frame.",
        synopsis: "WS::frame drop",
        mutator: true,
        side_effects: &[SideEffect {
            target: SideEffectTarget::NetworkIo,
            reads: true,
            writes: true,
            connection_side: ConnectionSide::Both,
        }],
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "orig_masked",
        arity: Arity::exact(0),
        detail: "Check if frame was masked.",
        synopsis: "WS::frame orig_masked",
        pure: true,
        side_effects: &[SideEffect {
            target: SideEffectTarget::NetworkIo,
            reads: true,
            writes: false,
            connection_side: ConnectionSide::Both,
        }],
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "type",
        arity: Arity::exact(0),
        detail: "Get frame type.",
        synopsis: "WS::frame type",
        pure: true,
        side_effects: &[SideEffect {
            target: SideEffectTarget::NetworkIo,
            reads: true,
            writes: false,
            connection_side: ConnectionSide::Both,
        }],
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "mask",
        arity: Arity::exact(0),
        detail: "Get frame mask.",
        synopsis: "WS::frame mask",
        pure: true,
        side_effects: &[SideEffect {
            target: SideEffectTarget::NetworkIo,
            reads: true,
            writes: false,
            connection_side: ConnectionSide::Both,
        }],
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "insert",
        arity: Arity::new(2, 3),
        detail: "Insert a new frame.",
        synopsis: "WS::frame insert <type> <payload> ?mask?",
        mutator: true,
        side_effects: &[SideEffect {
            target: SideEffectTarget::NetworkIo,
            reads: true,
            writes: true,
            connection_side: ConnectionSide::Both,
        }],
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "prepend",
        arity: Arity::exact(1),
        detail: "Prepend data to frame.",
        synopsis: "WS::frame prepend <data>",
        mutator: true,
        side_effects: &[SideEffect {
            target: SideEffectTarget::NetworkIo,
            reads: true,
            writes: true,
            connection_side: ConnectionSide::Both,
        }],
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "append",
        arity: Arity::exact(1),
        detail: "Append data to frame.",
        synopsis: "WS::frame append <data>",
        mutator: true,
        side_effects: &[SideEffect {
            target: SideEffectTarget::NetworkIo,
            reads: true,
            writes: true,
            connection_side: ConnectionSide::Both,
        }],
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "replace",
        arity: Arity::exact(1),
        detail: "Replace frame contents.",
        synopsis: "WS::frame replace <data>",
        mutator: true,
        side_effects: &[SideEffect {
            target: SideEffectTarget::NetworkIo,
            reads: true,
            writes: true,
            connection_side: ConnectionSide::Both,
        }],
        ..SubCommand::DEFAULT
    },
];

pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "WS::frame",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "This command allows you to perform various operations on a Websocket frame, determine whether this frame indicates the end of the message, insert a new frame, drop the current frame, or manipulate the frame by prepending, appending or replacing the contents of the frame.",
            synopsis: &[
                "WS::frame <subcommand> ?args?",
                "WS::frame insert <type> <payload> ?mask?",
            ],
            snippet: "WS::frame eom\n    The command can be used to determine whether current frame is last one in the Websocket message.\n\nWS::frame orig_masked\n    The command can be used to determine whether current frame received from the client or server was masked.\n\nWS::frame type\n    The command can be used to determine the type of current frame received from the client or server.\n\nWS::frame mask\n    The command can be used to determine the mask of the current frame.\n\nWS::frame drop\n    The command can be used to drop the current frame.",
            source: "https://clouddocs.f5.com/api/irules/WS__frame.html",
            examples: "when WS_SERVER_FRAME {\n    log local0. \"Websocket frame eom: [WS::frame eom]\"\n    log local0. \"Websocket frame received mask: [WS::frame orig_masked]\"\n    log local0. \"Websocket frame type: [WS::frame type]\"\n    log local0. \"Websocket frame mask: [WS::frame mask]\"\n    WS::frame drop\n    WS::frame insert 1 \"abcdefghi\"\n    WS::frame prepend \"Using WS I sent \"\n    WS::frame append \"message was sent\"\n    WS::frame replace \"replaced\"\n}",
            return_value: "The eom, orig_masked, type and mask commands return the values of corresponding fields in the Websockets frame header. Drop, insert, prepend, append and replace can be used to manipulate the frame contents.",
        }),
        event_requires: Some(EventRequires {
            client_side: false,
            server_side: false,
            transport: None,
            profiles: &["HTTP"],
            also_in: &[],
            init_only: false,
            flow: false,
            capability: None,
        }),
        forms: &[FormSpec {
            kind: FormKind::Default,
            synopsis: "WS::frame <subcommand> ?args?",
        }],
        subcommands: SUBCOMMANDS,
        side_effects: &[SideEffect {
            target: SideEffectTarget::NetworkIo,
            reads: true,
            writes: false,
            connection_side: ConnectionSide::Both,
        }],
        ..CommandSpec::DEFAULT
    }
}
