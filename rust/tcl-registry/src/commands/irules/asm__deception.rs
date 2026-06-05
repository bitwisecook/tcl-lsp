//! `ASM::deception` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "ASM::deception",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Mark a request as deceptive for further enforcement by asm",
            synopsis: &["ASM::deception"],
            snippet: "Mark a request as deceptive for further enforcement by asm",
            source: "https://clouddocs.f5.com/api/irules/ASM__deception.html",
            examples: "when ASM_REQUEST_DONE\n            {\n                ASM::deception\n            }",
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
        forms: &[
            FormSpec { kind: FormKind::Default, synopsis: "ASM::deception" },
        ],
        ..CommandSpec::DEFAULT
    }
}
