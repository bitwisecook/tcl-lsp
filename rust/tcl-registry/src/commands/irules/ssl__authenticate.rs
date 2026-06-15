//! `SSL::authenticate` iRules command.
use crate::prelude::*;

/// Subcommands ported from the Python source of truth.
const SUBCOMMANDS: &[SubCommand] = &[
    SubCommand {
        name: "once",
        arity: Arity::exact(0),
        detail: "Authenticate once per session.",
        synopsis: "SSL::authenticate once",
        mutator: true,
        side_effects: &[SideEffect {
            target: SideEffectTarget::SslState,
            reads: false,
            writes: true,
            connection_side: ConnectionSide::Both,
        }],
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "always",
        arity: Arity::exact(0),
        detail: "Authenticate on every connection.",
        synopsis: "SSL::authenticate always",
        mutator: true,
        side_effects: &[SideEffect {
            target: SideEffectTarget::SslState,
            reads: false,
            writes: true,
            connection_side: ConnectionSide::Both,
        }],
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "depth",
        arity: Arity::exact(1),
        detail: "Set max certificate chain traversal depth.",
        synopsis: "SSL::authenticate depth <number>",
        mutator: true,
        side_effects: &[SideEffect {
            target: SideEffectTarget::SslState,
            reads: false,
            writes: true,
            connection_side: ConnectionSide::Both,
        }],
        ..SubCommand::DEFAULT
    },
];

pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "SSL::authenticate",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Overrides the current setting for authentication frequency or for the maximum depth of certificate chain traversal.",
            synopsis: &["SSL::authenticate (once | always | (depth DEPTH))"],
            snippet: "Overrides the current setting for authentication frequency or for the maximum depth of certificate chain traversal.\n\nSSL::authenticate <\"once\" | \"always\">\n    Valid in a client-side context only, this command overrides the client-side SSL connection's current setting regarding authentication frequency.\n\nSSL::authenticate depth <number>\n    When the system evaluates the command in a client-side context, the command overrides the client-side SSL connection's current setting regarding maximum certificate chain traversal depth.",
            source: "https://clouddocs.f5.com/api/irules/SSL__authenticate.html",
            examples: "when CLIENT_ACCEPTED {\n    set session_flag 0\n}",
            return_value: "",
        }),
        forms: &[FormSpec {
            kind: FormKind::Default,
            synopsis: "SSL::authenticate <once | always | depth <number>>",
        }],
        subcommands: SUBCOMMANDS,
        side_effects: &[SideEffect {
            target: SideEffectTarget::SslState,
            reads: false,
            writes: true,
            connection_side: ConnectionSide::Both,
        }],
        ..CommandSpec::DEFAULT
    }
}
