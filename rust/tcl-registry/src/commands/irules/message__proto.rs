//! `MESSAGE::proto` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "MESSAGE::proto",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Returns protocol of the message.",
            synopsis: &["MESSAGE::proto"],
            snippet: "returns protocol of the message. For example, SIP, and DIAMETER.\nThis is valid for messages of the following protocols:\n\n    DIAMETER\n    SIP",
            source: "https://clouddocs.f5.com/api/irules/MESSAGE__proto.html",
            examples: "when MR_INGRESS {\n    log local0. \"[MESSAGE::proto]\"\n}",
            return_value: "returns protocol of the message. For example, SIP, and DIAMETER.",
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
        forms: &[
            FormSpec { kind: FormKind::Default, synopsis: "MESSAGE::proto" },
        ],
        side_effects: &[
            SideEffect {
                target: SideEffectTarget::MessageState,
                reads: true,
                writes: false,
                connection_side: ConnectionSide::Both,
            },
        ],
        ..CommandSpec::DEFAULT
    }
}
