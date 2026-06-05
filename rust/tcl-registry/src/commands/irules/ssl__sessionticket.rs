//! `SSL::sessionticket` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "SSL::sessionticket",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Returns the session ticket associated with the SSL flow.",
            synopsis: &["SSL::sessionticket"],
            snippet: "This command returns the session ticket associated with the SSL flow.",
            source: "https://clouddocs.f5.com/api/irules/SSL__sessionticket.html",
            examples: "when CLIENTSSL_HANDSHAKE {\n    set st [SSL::sessionticket]\n    set stlen [string length $st]\n    log local0. \"stlen $stlen\"\n    log local0. \"st $st\"\n}",
            return_value: "This command returns the session ticket associated with the SSL flow.",
        }),
        event_requires: Some(EventRequires {
            client_side: false,
            server_side: false,
            transport: Some("tcp"),
            profiles: &["CLIENTSSL", "SERVERSSL"],
            also_in: &[],
            init_only: false,
            flow: false,
            capability: None,
        }),
        ..CommandSpec::DEFAULT
    }
}
