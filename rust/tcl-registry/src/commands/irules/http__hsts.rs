//! `HTTP::hsts` iRules command.
use crate::prelude::*;

/// Subcommands ported from the Python source of truth.
const SUBCOMMANDS: &[SubCommand] = &[
    SubCommand {
        name: "mode",
        arity: Arity::exact(1),
        detail: "Enable or disable HSTS on a per-flow basis.",
        synopsis: "HTTP::hsts mode <enable | disable>",
        mutator: true,
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "maximum-age",
        arity: Arity::exact(1),
        detail: "Set HSTS maximum-age on a per-flow basis.",
        synopsis: "HTTP::hsts maximum-age <seconds>",
        mutator: true,
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "include-subdomains",
        arity: Arity::exact(1),
        detail: "Enable or disable HSTS include-subdomains on a per-flow basis.",
        synopsis: "HTTP::hsts include-subdomains <enable | disable>",
        mutator: true,
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "preload",
        arity: Arity::exact(1),
        detail: "Enable or disable HSTS preload on a per-flow basis (v13+).",
        synopsis: "HTTP::hsts preload <enable | disable>",
        mutator: true,
        ..SubCommand::DEFAULT
    },
];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "HTTP::hsts",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::new(0, 2),
hover: Some(HoverSnippet {
            summary: "Controls HTTP Strict Transport Security.",
            synopsis: &["HTTP::hsts", "HTTP::hsts mode <enable | disable>", "HTTP::hsts maximum-age <seconds>", "HTTP::hsts include-subdomains <enable | disable>", "HTTP::hsts preload <enable | disable>"],
            snippet: "Controls HSTS options on a per-flow basis, overriding the configured values in the HTTP profile.",
            source: "https://clouddocs.f5.com/api/irules/HTTP__hsts.html",
            examples: "when HTTP_REQUEST {\n    if { [HTTP::uri] contains \"secure\"} {\n        HTTP::hsts mode enable\n        HTTP::hsts maximum-age 8600\n        HTTP::hsts include-subdomains disable\n        HTTP::hsts preload enable\n    }\n}",
            return_value: "With no arguments, returns the currently configured HSTS header value for this connection.",
        }),
        event_requires: Some(EventRequires {
            client_side: false,
            server_side: false,
            transport: Some("tcp"),
            profiles: &["FASTHTTP", "HTTP"],
            also_in: &[],
            init_only: false,
            flow: false,
            capability: None,
        }),
        forms: &[
            FormSpec { kind: FormKind::Getter, synopsis: "HTTP::hsts" },
        ],
        subcommands: SUBCOMMANDS,
        side_effects: &[
            SideEffect {
                target: SideEffectTarget::HttpHeader,
                reads: false,
                writes: true,
                connection_side: ConnectionSide::Client,
            },
        ],
        ..CommandSpec::DEFAULT
    }
}
