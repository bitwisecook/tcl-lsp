//! `MR::collect` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "MR::collect",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Collect the specified amount of MR message payload data.",
            synopsis: &["MR::collect (COLLECT)?"],
            snippet: "Collects the specified amount of MR message payload data before triggering a MR_DATA event.\n\nSYNTAX\n\nMR::collect [<collect_bytes>]\n\nMR::collect\n        Collect the entire payload of the MR message. To stop collecting use MR::release command. MR_DATA event will be raised on every ingress invocation of the proxy.\n\nMR::collect <collect_bytes>\n        Collect <collect_bytes> bytes of payload of the MR message.\n        If payload is smaller than <collect_bytes> collect entire payload.\n        The collected data can be accessed via the MR::payload command.",
            source: "https://clouddocs.f5.com/api/irules/MR__collect.html",
            examples: "",
            return_value: "",
        }),
        event_requires: Some(EventRequires {
            client_side: false,
            server_side: false,
            transport: None,
            profiles: &["MR"],
            also_in: &[],
            init_only: false,
            flow: false,
            capability: None,
        }),
        forms: &[FormSpec {
            kind: FormKind::Default,
            synopsis: "MR::collect (COLLECT)?",
        }],
        side_effects: &[SideEffect {
            target: SideEffectTarget::MessageState,
            reads: false,
            writes: true,
            connection_side: ConnectionSide::Both,
        }],
        ..CommandSpec::DEFAULT
    }
}
