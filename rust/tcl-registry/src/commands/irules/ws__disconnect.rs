//! `WS::disconnect` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "WS::disconnect",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "This command can be used to disconnect a Websocket connection.",
            synopsis: &["WS::disconnect ( CODE (RSN)? )"],
            snippet: "WS::disconnect <close-reason> <reason>\n    The Websocket connection is disconnected by sending a close frame to both end-points when the current frame is done. The specified code and reason will be sent in the header and payload of the frame respectively.",
            source: "https://clouddocs.f5.com/api/irules/WS__disconnect.html",
            examples: "when WS_CLIENT_FRAME_DONE {\n    WS::disconnect 1000 \"some random reason\"\n}",
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
        forms: &[
            FormSpec { kind: FormKind::Default, synopsis: "WS::disconnect ( CODE (RSN)? )" },
        ],
        side_effects: &[
            SideEffect {
                target: SideEffectTarget::ConnectionControl,
                reads: false,
                writes: true,
                connection_side: ConnectionSide::Both,
            },
        ],
        ..CommandSpec::DEFAULT
    }
}
