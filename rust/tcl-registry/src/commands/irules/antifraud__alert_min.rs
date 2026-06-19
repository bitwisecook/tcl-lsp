//! `ANTIFRAUD::alert_min` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "ANTIFRAUD::alert_min",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Returns or sets variable data from client side, e.g.",
            synopsis: &["ANTIFRAUD::alert_min (VALUE)?"],
            snippet: "ANTIFRAUD::alert_min ;\n                Returns variable data from client side, e.g. forbidden added HTML element for the external_sources alert or bait signatures for the trojan_bait alert.\n\n            ANTIFRAUD::alert_min VALUE ;\n                Sets variable data from client side, e.g. forbidden added HTML element for the external_sources alert or bait signatures for the trojan_bait alert.",
            source: "https://clouddocs.f5.com/api/irules/ANTIFRAUD__alert_min.html",
            examples: "when ANTIFRAUD_ALERT {\n                if {[ANTIFRAUD::alert_type] eq \"js_vhtml\"} {\n                    if {[ANTIFRAUD::alert_component] eq \"external_sources\"} {\n                        log local0. \"Alert forbidden added element: [ANTIFRAUD::alert_min]\"\n                    }\n                    elseif {[ANTIFRAUD::alert_component] eq \"trojan_bait\"} {\n                        log local0. \"Alert bait signatures: [ANTIFRAUD::alert_min]\"\n                    }\n                }",
            return_value: "ANTIFRAUD::alert_min ; Returns variable data from client side, e.g. forbidden added HTML element for the external_sources alert or bait signatures for the trojan_bait alert.",
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
            synopsis: "ANTIFRAUD::alert_min (VALUE)?",
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
