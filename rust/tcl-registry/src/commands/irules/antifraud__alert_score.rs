//! `ANTIFRAUD::alert_score` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "ANTIFRAUD::alert_score",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Returns or sets alert severity.",
            synopsis: &["ANTIFRAUD::alert_score (VALUE)?"],
            snippet: "ANTIFRAUD::alert_score ;\n                Returns alert severity.\n\n            ANTIFRAUD::alert_score VALUE ;\n                Sets alert severity.",
            source: "https://clouddocs.f5.com/api/irules/ANTIFRAUD__alert_score.html",
            examples: "when ANTIFRAUD_ALERT {\n                log local0. \"original Alert score: [ANTIFRAUD::alert_score].\"\n                ANTIFRAUD::alert_score new_value\n                log local0. \"new Alert score: [ANTIFRAUD::alert_score].\"\n            }",
            return_value: "ANTIFRAUD::alert_score ; Returns alert severity.",
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
