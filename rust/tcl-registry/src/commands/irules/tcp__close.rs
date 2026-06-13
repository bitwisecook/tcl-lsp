//! `TCP::close` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "TCP::close",
        traits: Traits::DIAGRAM_ACTION,
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Closes the TCP connection.",
            synopsis: &["TCP::close"],
            snippet: "Sends the FIN byte to gracefully close the connection.",
            source: "https://clouddocs.f5.com/api/irules/TCP__close.html",
            examples: "when HTTP_REQUEST {\n    set my_loc \"http://www.i-want-a-bigip-for-christmas.com\"\n    TCP::respond \"HTTP/1.1 302 Found\\r\\nLocation: $my_loc\\r\\nConnection: close\\r\\nContent-Length: 0\\r\\n\\r\\n\"\n    TCP::close\n}",
            return_value: "None.",
        }),
        event_requires: Some(EventRequires {
            client_side: false,
            server_side: false,
            transport: Some("tcp"),
            profiles: &[],
            also_in: &["SIP_REQUEST", "SIP_REQUEST_SEND", "SIP_RESPONSE"],
            init_only: false,
            flow: false,
            capability: None,
        }),
        forms: &[
            FormSpec { kind: FormKind::Default, synopsis: "TCP::close" },
        ],
        side_effects: &[
            SideEffect {
                target: SideEffectTarget::TcpState,
                reads: false,
                writes: true,
                connection_side: ConnectionSide::Both,
            },
        ],
        ..CommandSpec::DEFAULT
    }
}
