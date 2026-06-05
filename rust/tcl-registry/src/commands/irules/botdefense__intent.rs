//! `BOTDEFENSE::intent` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "BOTDEFENSE::intent",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Returns the intent found for the bot that sent the current request.",
            synopsis: &["BOTDEFENSE::intent"],
            snippet: "Returns the intent found for the bot that sent the current request. The intent is based on the micro-service anomaly found for that client and may have been detected in a previous request of the client, not necessarily the present request",
            source: "https://clouddocs.f5.com/api/irules/BOTDEFENSE__intent.html",
            examples: "when BOTDEFENSE_ACTION {\n    if {[BOTDEFENSE::intent] contains \"OAT\"} {\n        BOTDEFENSE::action block\n    }\n}",
            return_value: "Returns the intent found for the bot that sent the current request based on a micro-service anomaly found for that bot, or empty string if no intent was found. The possible intents are those available per the various micro-services types.",
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
        ..CommandSpec::DEFAULT
    }
}
