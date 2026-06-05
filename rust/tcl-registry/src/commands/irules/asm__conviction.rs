//! `ASM::conviction` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "ASM::conviction",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Inject conviction honey traps in case of behavioral enforcement is enabled",
            synopsis: &["ASM::conviction"],
            snippet: "Inject conviction honey traps in case of behavioral enforcement is enabled",
            source: "https://clouddocs.f5.com/api/irules/ASM__conviction.html",
            examples: "when ASM_REQUEST_DONE\n            {\n                ASM::conviction\n            }",
            return_value: "no return value",
        }),
        event_requires: Some(EventRequires {
            client_side: false,
            server_side: false,
            transport: None,
            profiles: &["ASM"],
            also_in: &[],
            init_only: false,
            flow: false,
            capability: None,
        }),
        ..CommandSpec::DEFAULT
    }
}
