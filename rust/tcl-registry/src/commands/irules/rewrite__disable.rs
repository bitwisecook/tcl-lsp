//! `REWRITE::disable` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "REWRITE::disable",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Changes the REWRITE plugin from full patching mode to passthrough mode.",
            synopsis: &["REWRITE::disable"],
            snippet: "Changes the REWRITE plugin from full patching to passthrough mode.",
            source: "https://clouddocs.f5.com/api/irules/REWRITE__disable.html",
            examples: "when ACCESS_ACL_ALLOWED {\n  set host [HTTP::host]\n}",
            return_value: "",
        }),
        event_requires: Some(EventRequires {
            client_side: false,
            server_side: false,
            transport: None,
            profiles: &["ACCESS", "FASTHTTP", "REWRITE"],
            also_in: &[],
            init_only: false,
            flow: false,
            capability: None,
        }),
        forms: &[FormSpec {
            kind: FormKind::Default,
            synopsis: "REWRITE::disable",
        }],
        ..CommandSpec::DEFAULT
    }
}
