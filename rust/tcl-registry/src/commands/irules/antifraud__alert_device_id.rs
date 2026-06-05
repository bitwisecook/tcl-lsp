//! `ANTIFRAUD::alert_device_id` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "ANTIFRAUD::alert_device_id",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Deprecated: Returns flash GUID.",
            synopsis: &["ANTIFRAUD::alert_device_id"],
            snippet: "Returns flash GUID.",
            source: "https://clouddocs.f5.com/api/irules/ANTIFRAUD__alert_device_id.html",
            examples: "",
            return_value: "",
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
            synopsis: "ANTIFRAUD::alert_device_id",
        }],
        side_effects: &[SideEffect {
            target: SideEffectTarget::AsmState,
            reads: true,
            writes: false,
            connection_side: ConnectionSide::Client,
        }],
        deprecated_replacement: Some("ANTIFRAUD::device_id"),
        ..CommandSpec::DEFAULT
    }
}
