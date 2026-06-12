//! `ICAP::status` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "ICAP::status",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Returns the ICAP response status code.",
            synopsis: &["ICAP::status"],
            snippet: "The ICAP::status command gets the ICAP response status code. For\nexample, 100, 200, 204.",
            source: "https://clouddocs.f5.com/api/irules/ICAP__status.html",
            examples: "when ICAP_RESPONSE {\n                if {[ICAP::status] == 204} {\n                    log local0. \"ICAP server responded 204\"\n                }\n            }",
            return_value: "Return the ICAP response status code.",
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
            FormSpec { kind: FormKind::Default, synopsis: "ICAP::status" },
        ],
        side_effects: &[
            SideEffect {
                target: SideEffectTarget::IcapState,
                reads: true,
                writes: false,
                connection_side: ConnectionSide::Both,
            },
        ],
        ..CommandSpec::DEFAULT
    }
}
