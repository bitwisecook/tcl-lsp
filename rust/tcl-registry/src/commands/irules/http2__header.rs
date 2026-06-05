//! `HTTP2::header` iRules command.
use crate::prelude::*;

/// iRules subcommands ported from the Python source of truth.
const SUBCOMMANDS: &[SubCommand] = &[
    SubCommand {
        name: "replace",
        arity: Arity::new(1, 2),
        detail: "Replace pseudo-header value.",
        synopsis: "HTTP2::header replace <name> ?<string>?",
        mutator: true,
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "remove",
        arity: Arity::exact(1),
        detail: "Remove a pseudo-header.",
        synopsis: "HTTP2::header remove <name>",
        mutator: true,
        ..SubCommand::DEFAULT
    },
];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "HTTP2::header",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::new(1, 3),
hover: Some(HoverSnippet {
            summary: "Queries or modifies HTTP/2 pseudo-headers.",
            synopsis: &["HTTP2::header <name>", "HTTP2::header replace <name> ?<string>?", "HTTP2::header remove <name>"],
            snippet: "Queries or modifies HTTP/2 pseudo-headers.\nThe HTTP/2 pseudo-header names are lowercase and start with a ':' character.\nIntroduced in BIG-IP 16.1.0.",
            source: "https://clouddocs.f5.com/api/irules/HTTP2__header.html",
            examples: "when HTTP_REQUEST {\n  if { [HTTP2::header :authority] starts_with \"uat\" }  {\n    pool uat_pool\n  } else {\n    pool main_pool\n  }\n}",
            return_value: "",
        }),
        event_requires: Some(EventRequires {
            client_side: false,
            server_side: false,
            transport: Some("tcp"),
            profiles: &["HTTP"],
            also_in: &["MR_EGRESS", "MR_INGRESS", "SERVER_CONNECTED"],
            init_only: false,
            flow: false,
            capability: None,
        }),
        forms: &[
            FormSpec { kind: FormKind::Getter, synopsis: "HTTP2::header <name>" },
        ],
        subcommands: SUBCOMMANDS,
        ..CommandSpec::DEFAULT
    }
}
