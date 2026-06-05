//! `TCP::payload` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "TCP::payload",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::new(0, 4),
hover: Some(HoverSnippet {
            summary: "Returns or changes the data collected by TCP::collect.",
            synopsis: &["TCP::payload ?<size>?", "TCP::payload replace <offset> <length> <data>", "TCP::payload length"],
            snippet: "Returns the accumulated TCP data content, or replaces collected payload with the specified data.",
            source: "https://clouddocs.f5.com/api/irules/TCP__payload.html",
            examples: "when CLIENT_ACCEPTED {\n  TCP::collect\n}",
            return_value: "",
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
            FormSpec { kind: FormKind::Getter, synopsis: "TCP::payload ?<size>?" },
        ],
        ..CommandSpec::DEFAULT
    }
}
