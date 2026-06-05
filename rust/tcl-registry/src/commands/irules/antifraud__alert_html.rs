//! `ANTIFRAUD::alert_html` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "ANTIFRAUD::alert_html",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "For js_vhtml alert: returns or sets the whole HTML in an escaped base64 format.",
            synopsis: &["ANTIFRAUD::alert_html (VALUE)?"],
            snippet: "ANTIFRAUD::alert_html ;\n                For js_vhtml alert: returns the whole HTML in an escaped base64 format.\n\n            ANTIFRAUD::alert_html VALUE ;\n                For client side alerts: sets the whole HTML in an escaped base64 format.",
            source: "https://clouddocs.f5.com/api/irules/ANTIFRAUD__alert_html.html",
            examples: "when ANTIFRAUD_ALERT {\n                log local0. \"original Alert HTML: [ANTIFRAUD::alert_html].\"\n                ANTIFRAUD::alert_html new_value\n                log local0. \"new Alert HTML: [ANTIFRAUD::alert_html].\"\n            }",
            return_value: "ANTIFRAUD::alert_html ; For js_vhtml alert: returns the whole HTML in an escaped base64 format.",
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
            FormSpec { kind: FormKind::Default, synopsis: "ANTIFRAUD::alert_html (VALUE)?" },
        ],
        ..CommandSpec::DEFAULT
    }
}
