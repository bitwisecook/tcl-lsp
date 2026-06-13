//! `SIP::respond` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "SIP::respond",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Terminates a SIP response and responds with one of your creation.",
            synopsis: &["SIP::respond RESPONSE_CODE (PHRASE (HEADER_NAME HEADER_VALUE)*)?"],
            snippet: "This command allows you to terminate a SIP request and send a custom\nformatted response directly from the iRule.",
            source: "https://clouddocs.f5.com/api/irules/SIP__respond.html",
            examples: "when SIP_REQUEST {\n  log local0. [SIP::uri]\n  log local0. [SIP::header Via 0]\n  if {[SIP::method] == \"INVITE\"} {\n    SIP::respond 401 \"no way\" X-Header \"xxx here\"\n  }\n}",
            return_value: "",
        }),
        event_requires: Some(EventRequires {
            client_side: false,
            server_side: false,
            transport: None,
            profiles: &["SIP"],
            also_in: &[],
            init_only: false,
            flow: false,
            capability: None,
        }),
        forms: &[
            FormSpec { kind: FormKind::Default, synopsis: "SIP::respond RESPONSE_CODE (PHRASE (HEADER_NAME HEADER_VALUE)*)?" },
        ],
        side_effects: &[
            SideEffect {
                target: SideEffectTarget::NetworkIo,
                reads: false,
                writes: true,
                connection_side: ConnectionSide::Both,
            },
        ],
        ..CommandSpec::DEFAULT
    }
}
