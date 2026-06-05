//! `ANTIFRAUD::alert_defined_value` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "ANTIFRAUD::alert_defined_value",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Returns or sets defined (configured) value.",
            synopsis: &["ANTIFRAUD::alert_defined_value (VALUE)?"],
            snippet: "ANTIFRAUD::alert_defined_value ;\n                Returns defined (configured) value.\n\n            ANTIFRAUD::alert_defined_value VALUE ;\n                Sets defined (configured) value.",
            source: "https://clouddocs.f5.com/api/irules/ANTIFRAUD__alert_defined_value.html",
            examples: "when ANTIFRAUD_ALERT {\n                log local0. \"original Alert defined value: [ANTIFRAUD::alert_defined_value].\"\n                ANTIFRAUD::alert_defined_value new_value\n                log local0. \"new Alert defined value: [ANTIFRAUD::alert_defined_value].\"\n            }",
            return_value: "ANTIFRAUD::alert_defined_value ; Returns defined (configured) value.",
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
