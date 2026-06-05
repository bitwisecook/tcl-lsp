//! `XLAT::src_addr` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "XLAT::src_addr",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Retrieve the source translation address.",
            synopsis: &["XLAT::src_addr"],
            snippet: "Retrieve the source translation address.",
            source: "https://clouddocs.f5.com/api/irules/XLAT__src_addr.html",
            examples: "when SA_PICKED {\n    log local0. \"[XLAT::src_addr]\"\n}",
            return_value: "Return the string representation of the source translation address.",
        }),
        event_requires: Some(EventRequires {
            client_side: false,
            server_side: false,
            transport: None,
            profiles: &[],
            also_in: &["SA_PICKED"],
            init_only: false,
            flow: false,
            capability: None,
        }),
        forms: &[FormSpec {
            kind: FormKind::Default,
            synopsis: "XLAT::src_addr",
        }],
        ..CommandSpec::DEFAULT
    }
}
