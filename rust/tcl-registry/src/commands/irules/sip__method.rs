//! `SIP::method` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "SIP::method",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Returns the type of SIP request method.",
            synopsis: &["SIP::method"],
            snippet: "Returns the type of SIP request method.\n\nSIP::method\n\n     * Returns the type of SIP request method.",
            source: "https://clouddocs.f5.com/api/irules/SIP__method.html",
            examples: "when SIP_REQUEST {\n  log local0. [SIP::uri]\n  log local0. [SIP::header Via 0]\n  if {[SIP::method] == \"INVITE\"} {\n    SIP::respond 401 \"no way\" X-Header \"xxx here\"\n  }\n}",
            return_value: "Returns the type of SIP request method",
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
            FormSpec { kind: FormKind::Default, synopsis: "SIP::method" },
        ],
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
