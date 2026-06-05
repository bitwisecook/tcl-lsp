//! `ANTIFRAUD::alert_http_referrer` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "ANTIFRAUD::alert_http_referrer",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Returns or sets alert HTTP referrer.",
            synopsis: &["ANTIFRAUD::alert_http_referrer (VALUE)?"],
            snippet: "ANTIFRAUD::alert_http_referrer ;\n                Returns alert HTTP referrer.\n\n            ANTIFRAUD::alert_http_referrer VALUE ;\n                Sets alert HTTP referrer.",
            source: "https://clouddocs.f5.com/api/irules/ANTIFRAUD__alert_http_referrer.html",
            examples: "when ANTIFRAUD_ALERT {\n                log local0. \"original Alert HTTP referrer: [ANTIFRAUD::alert_http_referrer].\"\n                ANTIFRAUD::alert_http_referrer new_value\n                log local0. \"new Alert HTTP referrer: [ANTIFRAUD::alert_http_referrer].\"\n            }",
            return_value: "ANTIFRAUD::alert_http_referrer ; Returns alert HTTP referrer.",
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
            FormSpec { kind: FormKind::Default, synopsis: "ANTIFRAUD::alert_http_referrer (VALUE)?" },
        ],
        ..CommandSpec::DEFAULT
    }
}
