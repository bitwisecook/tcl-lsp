//! `BOTDEFENSE::bot_anomalies` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "BOTDEFENSE::bot_anomalies",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Returns the list of names of anomalies detected for the client that sent the current request.",
            synopsis: &["BOTDEFENSE::bot_anomalies"],
            snippet: "Returns the list of names of anomalies detected for the client that sent the current request. Some anomalies may have been detected in previous requests of the same client and are still valid.",
            source: "https://clouddocs.f5.com/api/irules/BOTDEFENSE__bot_anomalies.html",
            examples: "when BOTDEFENSE_ACTION {\n    foreach {anomaly} [BOTDEFENSE::bot_anomalies] {\n        log.local0. \"Found anomaly: $anomaly\"\n    }\n}",
            return_value: "Returns a list of names of all anomalies detected for the sending client. In case no anomalies found it returns an empty list.",
        }),
        event_requires: Some(EventRequires {
            client_side: false,
            server_side: false,
            transport: None,
            profiles: &["BOTDEFENSE"],
            also_in: &[],
            init_only: false,
            flow: false,
            capability: None,
        }),
        forms: &[
            FormSpec { kind: FormKind::Default, synopsis: "BOTDEFENSE::bot_anomalies" },
        ],
        ..CommandSpec::DEFAULT
    }
}
