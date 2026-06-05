//! `ANTIFRAUD::alert_id` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "ANTIFRAUD::alert_id",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Returns or sets alert id.",
            synopsis: &["ANTIFRAUD::alert_id (VALUE)?"],
            snippet: "ANTIFRAUD::alert_id ;\n                Returns alert id.\n\n            ANTIFRAUD::alert_id VALUE ;\n                Sets alert id.",
            source: "https://clouddocs.f5.com/api/irules/ANTIFRAUD__alert_id.html",
            examples: "when ANTIFRAUD_ALERT {\n                log local0. \"original Alert ID: [ANTIFRAUD::alert_id].\"\n                ANTIFRAUD::alert_id new_value\n                log local0. \"new Alert ID: [ANTIFRAUD::alert_id].\"\n            }",
            return_value: "ANTIFRAUD::alert_id ; Returns alert id.",
        }),
        event_requires: Some(EventRequires {
            client_side: false,
            server_side: false,
            transport: None,
            profiles: &["ANTIFRAUD"],
            also_in: &[],
            init_only: false,
            flow: false,
            capability: None,
        }),
        ..CommandSpec::DEFAULT
    }
}
