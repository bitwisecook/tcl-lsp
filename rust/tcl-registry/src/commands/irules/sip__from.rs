//! `SIP::from` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "SIP::from",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Returns the value of the From header in a SIP request.",
            synopsis: &["SIP::from"],
            snippet: "Returns the value of the From header in a SIP request.",
            source: "https://clouddocs.f5.com/api/irules/SIP__from.html",
            examples: "when SIP_REQUEST {\n    log local0. \"SIP Protocol - REQUEST: Values From & To\"\n    log local0. \"From: [SIP::from] To: [SIP::to]\"\n}",
            return_value: "Returns the value of the From header in a SIP request",
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
            FormSpec { kind: FormKind::Default, synopsis: "SIP::from" },
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
