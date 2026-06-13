//! `ICAP::header` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "ICAP::header",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Sets or returns ICAP attributes in the ICAP header.",
            synopsis: &["ICAP::header 'names'", "ICAP::header 'at' HEADER_INDEX", "ICAP::header 'count' (HEADER_NAME)?", "ICAP::header 'exists' HEADER_NAME"],
            snippet: "The ICAP::header command sets or returns attributes in the ICAP header.",
            source: "https://clouddocs.f5.com/api/irules/ICAP__header.html",
            examples: "when ICAP_RESPONSE {\n                ICAP::header remove X-ICAP-my-custom-header\n            }",
            return_value: "",
        }),
        event_requires: Some(EventRequires {
            client_side: false,
            server_side: false,
            transport: None,
            profiles: &["ICAP"],
            also_in: &[],
            init_only: false,
            flow: false,
            capability: None,
        }),
        forms: &[
            FormSpec { kind: FormKind::Default, synopsis: "ICAP::header 'names'" },
        ],
        side_effects: &[
            SideEffect {
                target: SideEffectTarget::IcapState,
                reads: true,
                writes: false,
                connection_side: ConnectionSide::Both,
            },
        ],
        taint_source: Some(TaintColour::TAINTED),
        ..CommandSpec::DEFAULT
    }
}
