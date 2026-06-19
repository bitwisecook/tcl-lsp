//! `MESSAGE::type` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "MESSAGE::type",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Returns the type of the current message.",
            synopsis: &["MESSAGE::type"],
            snippet: "returns the type of the current message. For example, for SIP request, it will return 'request'.\nThis is valid for messages of the following protocols:\n\n    DIAMETER\n    SIP",
            source: "https://clouddocs.f5.com/api/irules/MESSAGE__type.html",
            examples: "when MR_INGRESS {\n    log local0. \"[MESSAGE::proto] [MESSAGE::type]\"\n}",
            return_value: "returns the type of the current message. For example, for SIP request, it will return 'request'.",
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
            synopsis: "MESSAGE::type",
        }],
        side_effects: &[SideEffect {
            target: SideEffectTarget::MessageState,
            reads: true,
            writes: false,
            connection_side: ConnectionSide::Both,
        }],
        ..CommandSpec::DEFAULT
    }
}
