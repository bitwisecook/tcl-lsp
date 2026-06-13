//! `SSL::alpn` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "SSL::alpn",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Handle the ALPN TLS extension.",
            synopsis: &["SSL::alpn set (ARG)+", "SSL::alpn"],
            snippet: "Sets or retrieves the Application Layer Protocol Negotiation (ALPN) string.\n\nSSL::alpn\n  Retrieve the selected ALPN string\n\nSSL::alpn set str1[ str2...]\n  Set the advertised ALPN string",
            source: "https://clouddocs.f5.com/api/irules/SSL__alpn.html",
            examples: "when CLIENTSSL_CLIENTHELLO {\n    SSL::alpn set \"spdy/1\" \"spdy/2\" \"http/2\"\n}",
            return_value: "SSL::alpn Returns the negotiated ALPN string SSL::alpn set ... There is no return value.",
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
            FormSpec { kind: FormKind::Default, synopsis: "SSL::alpn set (ARG)+" },
        ],
        side_effects: &[
            SideEffect {
                target: SideEffectTarget::SslState,
                reads: true,
                writes: false,
                connection_side: ConnectionSide::Both,
            },
        ],
        ..CommandSpec::DEFAULT
    }
}
