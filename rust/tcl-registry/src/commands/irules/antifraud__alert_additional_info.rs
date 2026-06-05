//! `ANTIFRAUD::alert_additional_info` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "ANTIFRAUD::alert_additional_info",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Returns or sets a list of keys and values that describes integrity parameters check failure or parameter values too long error.",
            synopsis: &["ANTIFRAUD::alert_additional_info (VALUE)?"],
            snippet: "ANTIFRAUD::alert_additional_info ;\n                Returns a list of keys and values that describes integrity parameters check failure or parameter values too long error.\n\n            ANTIFRAUD::alert_additional_info VALUE ;\n                Sets a list of keys and values that describes integrity parameters check failure or parameter values too long error.",
            source: "https://clouddocs.f5.com/api/irules/ANTIFRAUD__alert_additional_info.html",
            examples: "when ANTIFRAUD_ALERT {\n                log local0. \"original Alert additional info: [ANTIFRAUD::alert_additional_info].\"\n                ANTIFRAUD::alert_additional_info new_value\n                log local0. \"new Alert additional info: [ANTIFRAUD::alert_additional_info].\"\n            }",
            return_value: "ANTIFRAUD::alert_additional_info ; Returns a list of keys and values that describes integrity parameters check failure or parameter values too long error.",
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
        forms: &[
            FormSpec { kind: FormKind::Default, synopsis: "ANTIFRAUD::alert_additional_info (VALUE)?" },
        ],
        ..CommandSpec::DEFAULT
    }
}
