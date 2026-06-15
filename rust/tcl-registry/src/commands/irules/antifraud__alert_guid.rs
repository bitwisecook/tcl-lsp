//! `ANTIFRAUD::alert_guid` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "ANTIFRAUD::alert_guid",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Returns GUID that is used to identify which users have been infected with malware before the user logs in.",
            synopsis: &["ANTIFRAUD::alert_guid"],
            snippet: "Returns GUID that is used to identify which users have been infected with malware before the user logs in.",
            source: "https://clouddocs.f5.com/api/irules/ANTIFRAUD__alert_guid.html",
            examples: "when ANTIFRAUD_ALERT {\n                log local0. \"Alert GUID: [ANTIFRAUD::alert_guid].\"\n            }",
            return_value: "Returns GUID that is used to identify which users have been infected with malware before the user logs in.",
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
        forms: &[FormSpec {
            kind: FormKind::Default,
            synopsis: "ANTIFRAUD::alert_guid",
        }],
        side_effects: &[SideEffect {
            target: SideEffectTarget::AsmState,
            reads: true,
            writes: false,
            connection_side: ConnectionSide::Client,
        }],
        ..CommandSpec::DEFAULT
    }
}
