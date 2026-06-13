//! `ANTIFRAUD::disable_injection` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "ANTIFRAUD::disable_injection",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Disables Anti-Fraud injections for the current transaction.",
            synopsis: &["ANTIFRAUD::disable_injection"],
            snippet: "Disables Anti-Fraud injections for the current transaction.",
            source: "https://clouddocs.f5.com/api/irules/ANTIFRAUD__disable_injection.html",
            examples: "when HTTP_RESPONSE {\n                if { [HTTP::header exists \"Antifraud-Disable-Injection\" ] } {\n                    ANTIFRAUD::disable_injection\n                    log local0. \"Injections disabled\"\n                }\n            }",
            return_value: "Disables Anti-Fraud injections for the current transaction.",
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
        forms: &[
            FormSpec { kind: FormKind::Default, synopsis: "ANTIFRAUD::disable_injection" },
        ],
        side_effects: &[
            SideEffect {
                target: SideEffectTarget::AsmState,
                reads: false,
                writes: true,
                connection_side: ConnectionSide::Client,
            },
        ],
        ..CommandSpec::DEFAULT
    }
}
