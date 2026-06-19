//! `ANTIFRAUD::disable_phishing` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "ANTIFRAUD::disable_phishing",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Disables phishing detection for the current transaction.",
            synopsis: &["ANTIFRAUD::disable_phishing"],
            snippet: "Disables phishing detection for the current transaction.",
            source: "https://clouddocs.f5.com/api/irules/ANTIFRAUD__disable_phishing.html",
            examples: "when HTTP_REQUEST {\n                if { [HTTP::header exists \"Antifraud-Disable-Phishing\" ] } {\n                    ANTIFRAUD::disable_phishing\n                    log local0. \"Phishing Detection disabled\"\n                }\n            }",
            return_value: "Disables phishing detection for the current transaction.",
        }),
        event_requires: Some(EventRequires {
            client_side: false,
            server_side: false,
            transport: None,
            profiles: &["FASTHTTP"],
            also_in: &[],
            init_only: false,
            flow: false,
            capability: None,
        }),
        forms: &[FormSpec {
            kind: FormKind::Default,
            synopsis: "ANTIFRAUD::disable_phishing",
        }],
        side_effects: &[SideEffect {
            target: SideEffectTarget::AsmState,
            reads: false,
            writes: true,
            connection_side: ConnectionSide::Client,
        }],
        ..CommandSpec::DEFAULT
    }
}
