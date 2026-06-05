//! `SSL::tls13_secret` iRules command.
use crate::prelude::*;

/// iRules subcommands ported from the Python source of truth.
const SUBCOMMANDS: &[SubCommand] = &[
    SubCommand {
        name: "client",
        arity: Arity::exact(1),
        detail: "Client-side TLS 1.3 secret.",
        synopsis: "SSL::tls13_secret client (app | hs | early)",
        pure: true,
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "server",
        arity: Arity::exact(1),
        detail: "Server-side TLS 1.3 secret.",
        synopsis: "SSL::tls13_secret server (app | hs)",
        pure: true,
        ..SubCommand::DEFAULT
    },
];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "SSL::tls13_secret",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Return data about various TLS 1.3 secrets.",
            synopsis: &["SSL::tls13_secret client (app | hs | early)", "SSL::tls13_secret server (app | hs)"],
            snippet: "Return TLS 1.3 session secrets. Choose which side (client or server) and which secret. \"app\" references the first traffic secret, \"hs\" -- the handshake traffic secret and \"early\" -- the client early traffic secret.",
            source: "https://clouddocs.f5.com/api/irules/SSL__tls13_secret.html",
            examples: "when CLIENTSSL_HANDSHAKE {\n    log local0. \"ClientSSL: Client Handshake Traffic Secret [SSL::clientrandom] is -> [SSL::tls13_secret client hs]\"\n    log local0. \"ClientSSL: Server Handshake Traffic Secret [SSL::clientrandom] is -> [SSL::tls13_secret server hs]\"\n    log local0. \"ClientSSL: Client App Traffic Secret [SSL::clientrandom] is -> [SSL::tls13_secret client app]\"\n    log local0. \"ClientSSL: Server App Traffic Secret [SSL::clientrandom] is -> [SSL::tls13_secret server app]\"",
            return_value: "SSL::tls13_secret client app Returns the client app secret. SSL::tls13_secret server app Returns the server app secret. SSL::tls13_secret client hs Returns the client handshake secret SSL::tls13_secret server hs Returns the server handshake secret.",
        }),
        event_requires: Some(EventRequires {
            client_side: true,
            server_side: false,
            transport: Some("tcp"),
            profiles: &["CLIENTSSL"],
            also_in: &[],
            init_only: false,
            flow: false,
            capability: None,
        }),
        forms: &[
            FormSpec { kind: FormKind::Default, synopsis: "SSL::tls13_secret <side> <secret_type>" },
        ],
        subcommands: SUBCOMMANDS,
        ..CommandSpec::DEFAULT
    }
}
