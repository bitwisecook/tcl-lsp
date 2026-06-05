//! `ANTIFRAUD::alert_bait_signatures` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "ANTIFRAUD::alert_bait_signatures",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Deprecated: For the trojan_bait alert: returns the bait signatures in an escaped base64 format.",
            synopsis: &["ANTIFRAUD::alert_bait_signatures"],
            snippet: "For the trojan_bait alert: returns the bait signatures in an escaped base64 format.",
            source: "https://clouddocs.f5.com/api/irules/ANTIFRAUD__alert_bait_signatures.html",
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
        forms: &[
            FormSpec { kind: FormKind::Default, synopsis: "ANTIFRAUD::alert_bait_signatures" },
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
