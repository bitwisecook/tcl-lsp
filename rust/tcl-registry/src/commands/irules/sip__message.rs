//! `SIP::message` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "SIP::message",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Returns the full content of the SIP request or response message.",
            synopsis: &["SIP::message"],
            snippet: "The SIP::message command returns full content of the SIP request\nor response message.",
            source: "https://clouddocs.f5.com/api/irules/SIP__message.html",
            examples: "when SIP_REQUEST {\n  log local0. [SIP::message]\n}",
            return_value: "Returns content of the current message",
        }),
        event_requires: Some(EventRequires {
            client_side: false,
            server_side: false,
            transport: None,
            profiles: &["SIP"],
            also_in: &["MR_EGRESS", "MR_FAILED", "MR_INGRESS"],
            init_only: false,
            flow: false,
            capability: None,
        }),
        forms: &[
            FormSpec { kind: FormKind::Default, synopsis: "SIP::message" },
        ],
        side_effects: &[
            SideEffect {
                target: SideEffectTarget::NetworkIo,
                reads: true,
                writes: true,
                connection_side: ConnectionSide::Both,
            },
        ],
        taint_source: Some(TaintColour::TAINTED),
        ..CommandSpec::DEFAULT
    }
}
