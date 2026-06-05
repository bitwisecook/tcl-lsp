//! `DIAMETER::header` iRules command.
use crate::prelude::*;

/// iRules subcommands ported from the Python source of truth.
const SUBCOMMANDS: &[SubCommand] = &[
    SubCommand {
        name: "version",
        arity: Arity::new(0, 1),
        detail: "Get/set the DIAMETER protocol version.",
        synopsis: "DIAMETER::header version ?value?",
        pure: true,
        mutator: true,
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "length",
        arity: Arity::exact(0),
        detail: "Get the message length (read-only).",
        synopsis: "DIAMETER::header length ?value?",
        pure: true,
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "rflag",
        arity: Arity::new(0, 1),
        detail: "Get/set the request flag (R-bit).",
        synopsis: "DIAMETER::header rflag ?value?",
        pure: true,
        mutator: true,
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "pflag",
        arity: Arity::new(0, 1),
        detail: "Get/set the proxiable flag (P-bit).",
        synopsis: "DIAMETER::header pflag ?value?",
        pure: true,
        mutator: true,
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "eflag",
        arity: Arity::new(0, 1),
        detail: "Get/set the error flag (E-bit).",
        synopsis: "DIAMETER::header eflag ?value?",
        pure: true,
        mutator: true,
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "tflag",
        arity: Arity::new(0, 1),
        detail: "Get/set the retransmit flag (T-bit).",
        synopsis: "DIAMETER::header tflag ?value?",
        pure: true,
        mutator: true,
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "command_code",
        arity: Arity::new(0, 1),
        detail: "Get/set the command code.",
        synopsis: "DIAMETER::header command_code ?value?",
        pure: true,
        mutator: true,
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "application_id",
        arity: Arity::new(0, 1),
        detail: "Get/set the application ID.",
        synopsis: "DIAMETER::header application_id ?value?",
        pure: true,
        mutator: true,
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "hop_by_hop_id",
        arity: Arity::new(0, 1),
        detail: "Get/set the hop-by-hop identifier.",
        synopsis: "DIAMETER::header hop_by_hop_id ?value?",
        pure: true,
        mutator: true,
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "end_to_end_id",
        arity: Arity::new(0, 1),
        detail: "Get/set the end-to-end identifier.",
        synopsis: "DIAMETER::header end_to_end_id ?value?",
        pure: true,
        mutator: true,
        ..SubCommand::DEFAULT
    },
];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "DIAMETER::header",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Gets or sets the DIAMETER header fields.",
            synopsis: &["DIAMETER::header <field> ?value?", "DIAMETER::header command_code ?value?", "DIAMETER::header application_id ?value?"],
            snippet: "This iRule command is used to get and set header fields in the current DIAMETER message.",
            source: "https://clouddocs.f5.com/api/irules/DIAMETER__header.html",
            examples: "when DIAMETER_INGRESS {\n    if { [DIAMETER::header tflag] } {\n        log local0. \"Received a potentially retransmitted Diameter message\"\n    }\n}",
            return_value: "",
        }),
        event_requires: Some(EventRequires {
            client_side: false,
            server_side: false,
            transport: None,
            profiles: &["DIAMETER", "MR"],
            also_in: &[],
            init_only: false,
            flow: false,
            capability: None,
        }),
        forms: &[
            FormSpec { kind: FormKind::Default, synopsis: "DIAMETER::header <field> ?value?" },
        ],
        subcommands: SUBCOMMANDS,
        side_effects: &[
            SideEffect {
                target: SideEffectTarget::NetworkIo,
                reads: true,
                writes: false,
                connection_side: ConnectionSide::Both,
            },
        ],
        ..CommandSpec::DEFAULT
    }
}
