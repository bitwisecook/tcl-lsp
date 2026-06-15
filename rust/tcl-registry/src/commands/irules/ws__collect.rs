//! `WS::collect` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "WS::collect",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "This command can be used to collect payload of current Websocket frame.",
            synopsis: &["WS::collect ('frame' (LENGTH)? )"],
            snippet: "WS::collect frame\nCollects the entire Websocket frame payload.\n\nNote that if multiple iRules invoke WS::collect simultaneously,\n(perhaps by being called by the same event in multiple iRule scripts)\nthen the result is undefined.  This is because the amount of payload\ncollected for the WS_CLIENT_DATA or WS_SERVER_DATA event cannot\nsatisfy the perhaps differing amounts wanted by the callers. iRules\nshould arbitrate amoungst themselves to prevent this situation from\noccuring, and have only one WS::collect call outstanding at a time.",
            source: "https://clouddocs.f5.com/api/irules/WS__collect.html",
            examples: "when WS_CLIENT_FRAME {\n    WS::collect frame\n}",
            return_value: "",
        }),
        event_requires: Some(EventRequires {
            client_side: false,
            server_side: false,
            transport: None,
            profiles: &["HTTP"],
            also_in: &[],
            init_only: false,
            flow: false,
            capability: None,
        }),
        forms: &[FormSpec {
            kind: FormKind::Default,
            synopsis: "WS::collect ('frame' (LENGTH)? )",
        }],
        side_effects: &[SideEffect {
            target: SideEffectTarget::NetworkIo,
            reads: false,
            writes: true,
            connection_side: ConnectionSide::Both,
        }],
        ..CommandSpec::DEFAULT
    }
}
