//! `ANTIFRAUD::alert_license_id` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "ANTIFRAUD::alert_license_id",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Returns crc32 of the license id in hex.",
            synopsis: &["ANTIFRAUD::alert_license_id"],
            snippet: "Returns crc32 of the license id in hex.",
            source: "https://clouddocs.f5.com/api/irules/ANTIFRAUD__alert_license_id.html",
            examples: "when ANTIFRAUD_ALERT {\n                log local0. \"Alert license ID: [ANTIFRAUD::alert_license_id].\"\n            }",
            return_value: "Returns crc32 of the license id in hex.",
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
            FormSpec { kind: FormKind::Default, synopsis: "ANTIFRAUD::alert_license_id" },
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
