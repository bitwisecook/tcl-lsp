//! `ICAP::method` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "ICAP::method",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Returns the ICAP request method.",
            synopsis: &["ICAP::method"],
            snippet: "The ICAP::method command returns the ICAP request method.\nThis will either be \"REQMOD\" or \"RESPMOD\"",
            source: "https://clouddocs.f5.com/api/irules/ICAP__method.html",
            examples: "when ICAP_REQUEST {\n                if {[ICAP::method] == \"REQMOD\"} {\n                    ICAP::header add X-Request $seqno\n                }\n            }",
            return_value: "Returns the ICAP request method.",
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
            FormSpec { kind: FormKind::Default, synopsis: "ICAP::method" },
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
