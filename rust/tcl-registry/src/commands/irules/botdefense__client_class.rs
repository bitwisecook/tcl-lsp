//! `BOTDEFENSE::client_class` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "BOTDEFENSE::client_class",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Returns the classification of the client based on the current request and its browsing history.",
            synopsis: &["BOTDEFENSE::client_class"],
            snippet: "Returns the classification of the client that sent the request. The returned value is one of the following strings:* unknown* browser* mobile_application* trusted_bot* untrusted_bot* malicious_bot* suspicious_browser. The command is similar to BOTDEFENSE::client_type but with higher resolution for bot classification: when BOTDEFENSE::client_type returns \"bot\", BOTDEFENSE::client_class returns the exact type of bot: malicious, trusted or untrusted.",
            source: "https://clouddocs.f5.com/api/irules/BOTDEFENSE__client_class.html",
            examples: "when BOTDEFENSE_ACTION {\n    log.local0. \"Client type after processing request: [BOTDEFENSE::client_class]\"\n}",
            return_value: "Returns the classification of the client that sent the request. When invoked in the BOTDEFENSE_REQUEST event it returns the type based on the previous requests of the same client, or \"unknown\" if the client is not recognized.",
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
            FormSpec { kind: FormKind::Default, synopsis: "BOTDEFENSE::client_class" },
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
