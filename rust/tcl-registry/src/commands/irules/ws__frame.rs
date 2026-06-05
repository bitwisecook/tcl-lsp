//! `WS::frame` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "WS::frame",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "This command allows you to perform various operations on a Websocket frame, determine whether this frame indicates the end of the message, insert a new frame, drop the current frame, or manipulate the frame by prepending, appending or replacing the contents of the frame.",
            synopsis: &["WS::frame <subcommand> ?args?", "WS::frame insert <type> <payload> ?mask?"],
            snippet: "WS::frame eom\n    The command can be used to determine whether current frame is last one in the Websocket message.\n\nWS::frame orig_masked\n    The command can be used to determine whether current frame received from the client or server was masked.\n\nWS::frame type\n    The command can be used to determine the type of current frame received from the client or server.\n\nWS::frame mask\n    The command can be used to determine the mask of the current frame.\n\nWS::frame drop\n    The command can be used to drop the current frame.",
            source: "https://clouddocs.f5.com/api/irules/WS__frame.html",
            examples: "when WS_SERVER_FRAME {\n    log local0. \"Websocket frame eom: [WS::frame eom]\"\n    log local0. \"Websocket frame received mask: [WS::frame orig_masked]\"\n    log local0. \"Websocket frame type: [WS::frame type]\"\n    log local0. \"Websocket frame mask: [WS::frame mask]\"\n    WS::frame drop\n    WS::frame insert 1 \"abcdefghi\"\n    WS::frame prepend \"Using WS I sent \"\n    WS::frame append \"message was sent\"\n    WS::frame replace \"replaced\"\n}",
            return_value: "The eom, orig_masked, type and mask commands return the values of corresponding fields in the Websockets frame header. Drop, insert, prepend, append and replace can be used to manipulate the frame contents.",
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
            FormSpec { kind: FormKind::Default, synopsis: "WS::frame <subcommand> ?args?" },
        ],
        ..CommandSpec::DEFAULT
    }
}
