//! `ANTIFRAUD::guid` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "ANTIFRAUD::guid",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Returns GUID value, only in context of ANTIFRAUD_LOGIN event.",
            synopsis: &["ANTIFRAUD::guid"],
            snippet: "Returns GUID value, only in context of ANTIFRAUD_LOGIN event.",
            source: "https://clouddocs.f5.com/api/irules/ANTIFRAUD__guid.html",
            examples: "when ANTIFRAUD_LOGIN {\n                log local0. \"Infected username with GUID [ANTIFRAUD::guid] tried to log in.\"\n            }",
            return_value: "Returns GUID value, only in context of ANTIFRAUD_LOGIN event.",
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
            FormSpec { kind: FormKind::Default, synopsis: "ANTIFRAUD::guid" },
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
