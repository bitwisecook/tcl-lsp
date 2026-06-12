//! `WS::message` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "WS::message",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "This command can be used to drop an entire Websocket message.",
            synopsis: &["WS::message ( 'drop' )"],
            snippet: "WS::message drop\n    Drop an entire Websocket message.",
            source: "https://clouddocs.f5.com/api/irules/WS__message.html",
            examples: "when WS_CLIENT_FRAME {\n    WS::message drop\n}",
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
            synopsis: "WS::message ( 'drop' )",
        }],
        side_effects: &[SideEffect {
            target: SideEffectTarget::NetworkIo,
            reads: true,
            writes: false,
            connection_side: ConnectionSide::Both,
        }],
        taint_source: Some(TaintColour::TAINTED),
        ..CommandSpec::DEFAULT
    }
}
