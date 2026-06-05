//! `XLAT::src_port` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "XLAT::src_port",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Retrieve the source translation port.",
            synopsis: &["XLAT::src_port"],
            snippet: "Retrieve the source translation port.",
            source: "https://clouddocs.f5.com/api/irules/XLAT__src_port.html",
            examples: "when SA_PICKED {\n    log local0. \"[XLAT::src_port]\"\n}",
            return_value: "Return the source translation port.",
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
            synopsis: "XLAT::src_port",
        }],
        ..CommandSpec::DEFAULT
    }
}
