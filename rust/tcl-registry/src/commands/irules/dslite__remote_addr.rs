//! `DSLITE::remote_addr` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "DSLITE::remote_addr",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Returns the remote DS-Lite tunnel.",
            synopsis: &["DSLITE::remote_addr"],
            snippet: "Returns the remote DS-Lite tunnel endpoint IP address.",
            source: "https://clouddocs.f5.com/api/irules/DSLITE__remote_addr.html",
            examples: "",
            return_value: "",
        }),
        forms: &[FormSpec {
            kind: FormKind::Default,
            synopsis: "DSLITE::remote_addr",
        }],
        ..CommandSpec::DEFAULT
    }
}
