//! `tcpdump` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "tcpdump",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "F5 iRules command `tcpdump`.",
            synopsis: &["tcpdump"],
            snippet: "",
            source: "https://clouddocs.f5.com/api/irules/tcpdump.html",
            examples: "",
            return_value: "",
        }),
        forms: &[FormSpec {
            kind: FormKind::Default,
            synopsis: "tcpdump",
        }],
        ..CommandSpec::DEFAULT
    }
}
