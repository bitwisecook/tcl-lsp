//! `ANTIFRAUD::alert_view_id` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "ANTIFRAUD::alert_view_id",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Returns or sets the configured URL and view which triggered this alert.",
            synopsis: &["ANTIFRAUD::alert_view_id (VALUE)?"],
            snippet: "ANTIFRAUD::alert_view_id ;\n                Returns the configured URL and view which triggered this alert. Empty if not a view.\n\n            ANTIFRAUD::alert_view_id VALUE ;\n                Sets the configured URL and view which triggered this alert. Empty if not a view.",
            source: "https://clouddocs.f5.com/api/irules/ANTIFRAUD__alert_view_id.html",
            examples: "when ANTIFRAUD_ALERT {\n                log local0. \"original Alert View ID: [ANTIFRAUD::alert_view_id].\"\n                ANTIFRAUD::alert_view_id new_value\n                log local0. \"new Alert View ID: [ANTIFRAUD::alert_view_id].\"\n            }",
            return_value: "ANTIFRAUD::alert_view_id ; Returns the configured URL and view which triggered this alert. Empty if not a view.",
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
