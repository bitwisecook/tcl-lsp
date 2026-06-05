//! `WS::response` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "WS::response",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "This command returns the values of the various Websocket header fields seen in a server response.",
            synopsis: &["WS::response ('protocol' | 'extension' | 'version' | 'key' | 'valid' )"],
            snippet: "WS::response protocol\n    Returns the value of Sec-WebSocket-Protocol header field in server response.\n\nWS::response extension\n    Returns the value of Sec-WebSocket-Extensions header field in server response.\n\nWS::response version\n    Returns the value of Sec-WebSocket-Version header field in server response.\n\nWS::response key\n    Returns the value of Sec-WebSocket-Accept header field in server response.\n\nWS::response valid\n    Returns whether the client request and server response resulted in a successful Websocket upgrade.",
            source: "https://clouddocs.f5.com/api/irules/WS__response.html",
            examples: "when WS_RESPONSE {\n    if { [WS::response key] equals \"s3pPLMBiTxaQ9kYGzzhZRbK+xOo=\"} {\n        WS::enabled false\n    }\n}",
            return_value: "This command can be used to lookup the values of various Websocket header fields seen in a server response.",
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
        ..CommandSpec::DEFAULT
    }
}
