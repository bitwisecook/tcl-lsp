//! `BOTDEFENSE::bot_categories` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "BOTDEFENSE::bot_categories",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Returns the list of category names to which the current client belongs.",
            synopsis: &["BOTDEFENSE::bot_categories"],
            snippet: "Returns the list of category names to which the current client belongs. These categories are determined by the anomalies found for the respective client. Note these categories are additional to the bot signature category which is applicable if a bot signature was found.",
            source: "https://clouddocs.f5.com/api/irules/BOTDEFENSE__bot_categories.html",
            examples: "when BOTDEFENSE_ACTION {\n    foreach {cat} [BOTDEFENSE::bot_categories] {\n        log.local0. \"Found category: $cat\"\n    }\n}",
            return_value: "Returns a list of all category names to which the current client belongs based on the anomalies found for the client. The categories come in addition to the bot signature category optionally detected and returned in BOTDEFENSE::bot_signature_category. If no anomaly found then the list will be empty.",
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
            FormSpec { kind: FormKind::Default, synopsis: "BOTDEFENSE::bot_categories" },
        ],
        side_effects: &[
            SideEffect {
                target: SideEffectTarget::AsmState,
                reads: true,
                writes: false,
                connection_side: ConnectionSide::Client,
            },
        ],
        ..CommandSpec::DEFAULT
    }
}
