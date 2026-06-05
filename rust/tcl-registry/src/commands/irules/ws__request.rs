//! `WS::request` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "WS::request",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "This command returns the values of the various Websocket header fields seen in a client request.",
            synopsis: &["WS::request ('protocol' | 'extension' | 'version' | 'key' )"],
            snippet: "WS::request protocol\n    Returns the value of Sec-WebSocket-Protocol header field in client request.\n\nWS::request extension\n    Returns the value of Sec-WebSocket-Extensions header field in client request.\n\nWS::request version\n    Returns the value of Sec-WebSocket-Version header field in client request.\n\nWS::request key\n    Returns the value of Sec-WebSocket-Key header field in client request.",
            source: "https://clouddocs.f5.com/api/irules/WS__request.html",
            examples: "when WS_REQUEST {\n    if { [WS::request protocol] equals \"chat\" } {\n        WS::enabled false\n    }\n}",
            return_value: "This command can be used to lookup the values of various Websocket header fields seen in a client request.",
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
            FormSpec { kind: FormKind::Default, synopsis: "WS::request <field>" },
        ],
        ..CommandSpec::DEFAULT
    }
}
