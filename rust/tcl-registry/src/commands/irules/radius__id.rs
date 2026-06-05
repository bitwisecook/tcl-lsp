//! `RADIUS::id` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "RADIUS::id",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "This command returns the RADIUS message id",
            synopsis: &["RADIUS::id"],
            snippet: "This command returns the RADIUS message id",
            source: "https://clouddocs.f5.com/api/irules/RADIUS__id.html",
            examples: "when CLIENT_ACCEPTED {\n    let msg_id [RADIUS::id]\n    log local0. \"recieved radius message with id $msg_id\"\n}",
            return_value: "This command returns the RADIUS message id",
        }),
        event_requires: Some(EventRequires {
            client_side: false,
            server_side: false,
            transport: None,
            profiles: &[],
            also_in: &[
                "CLIENT_ACCEPTED",
                "CLIENT_CLOSED",
                "CLIENT_DATA",
                "SERVER_CLOSED",
                "SERVER_CONNECTED",
                "SERVER_DATA",
            ],
            init_only: false,
            flow: false,
            capability: None,
        }),
        forms: &[
            FormSpec { kind: FormKind::Default, synopsis: "RADIUS::id" },
        ],
        ..CommandSpec::DEFAULT
    }
}
