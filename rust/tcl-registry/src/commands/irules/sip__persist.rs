//! `SIP::persist` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "SIP::persist",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Returns or replaces the persistence key being used for the current message.",
            synopsis: &[
                "SIP::persist",
                "SIP::persist reset",
                "SIP::persist use",
                "SIP::persist replace",
            ],
            snippet: "The SIP::persist command returns the persistence key being used for the\ncurrent message. If new-persist-key is provided, the existing\npersistence key is replaced. The value of the new-persist-key MUST be\none of valid header value in the message. A header name should not be\ngiven as the new-persist-key value.\nOnly valid for MRF SIP (sipsession profile) in 11.6+",
            source: "https://clouddocs.f5.com/api/irules/SIP__persist.html",
            examples: "when SIP_REQUEST {\n                SIP::persist \"[SIP::uri]-[SIP::from]-[SIP::to]\"\n            }",
            return_value: "",
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
        forms: &[FormSpec {
            kind: FormKind::Default,
            synopsis: "SIP::persist",
        }],
        side_effects: &[SideEffect {
            target: SideEffectTarget::PersistenceTable,
            reads: false,
            writes: true,
            connection_side: ConnectionSide::Both,
        }],
        ..CommandSpec::DEFAULT
    }
}
