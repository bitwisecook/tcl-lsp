//! `HTTP2::stream` iRules command.
use crate::prelude::*;

/// The command's subcommands.
const SUBCOMMANDS: &[SubCommand] = &[
    SubCommand {
        name: "id",
        arity: Arity::exact(0),
        detail: "Returns the stream id. Returns 0 if HTTP/2 is not active.",
        synopsis: "HTTP2::stream id",
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "priority",
        arity: Arity::new(0, 1),
        detail: "Get or set the priority of the current stream.",
        synopsis: "HTTP2::stream priority ?<priority>?",
        subcommand_forms: &[
            SubCommandForm {
                name: "getter",
                ..SubCommandForm::DEFAULT
            },
            SubCommandForm {
                name: "setter",
                ..SubCommandForm::DEFAULT
            },
        ],
        ..SubCommand::DEFAULT
    },
];

pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "HTTP2::stream",
        traits: Traits::PURE,
        dialects: Some(DialectSet::IRULES),
        arity: Arity::new(0, 2),
        hover: Some(HoverSnippet {
            summary: "Gets or sets the stream attributes including id and priority.",
            synopsis: &["HTTP2::stream (id | (priority (PRIORITY)?))?"],
            snippet: "This command can be used to determine the stream attributes including id and priority. This command can also be used to set the priority for a current active stream.\n\nHTTP2::stream\n    Returns the stream id. Returns 0 if HTTP/2 is not active.\n\nHTTP2::stream id\n    Returns the stream id. Returns 0 if HTTP/2 is not active.\n\nHTTP2::stream priority\n    Returns the priority of the current stream.\n\nHTTP2::stream priority <priority>\n    Sets the priority of the current active stream. The return is 0 is the priority was set. Return is an error if the priority is out of bounds (exceeding 8 bits).",
            source: "https://clouddocs.f5.com/api/irules/HTTP2__stream.html",
            examples: "when HTTP_REQUEST {\n    if {[HTTP2::version] != 0} {\n        set new_pri [URI::query [HTTP::uri] pri]\n        if { $new_pri != \"\" } {\n             HTTP2::stream priority $new_pri\n        }\n        HTTP::header insert \"X-HTTP2-Values stream/pritority \" \"[HTTP2::stream]/[HTTP2::stream priority]\"\n    }\n}",
            return_value: "",
        }),
        event_requires: Some(EventRequires {
            client_side: false,
            server_side: false,
            transport: Some("tcp"),
            profiles: &["HTTP"],
            also_in: &[],
            init_only: false,
            flow: false,
            capability: None,
        }),
        forms: &[FormSpec {
            kind: FormKind::Getter,
            synopsis: "HTTP2::stream",
        }],
        subcommands: SUBCOMMANDS,
        side_effects: &[SideEffect {
            target: SideEffectTarget::Http2State,
            reads: true,
            writes: true,
            connection_side: ConnectionSide::Both,
        }],
        ..CommandSpec::DEFAULT
    }
}
