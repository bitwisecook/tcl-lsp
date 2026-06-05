//! `DOSL7::is_mitigated` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "DOSL7::is_mitigated",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Returns TRUE if certain HTTP request was mitigated by DOSL7.",
            synopsis: &["DOSL7::is_mitigated"],
            snippet: "Returns TRUE if certain HTTP request was mitigated by DOSL7.",
            source: "https://clouddocs.f5.com/api/irules/DOSL7__is_mitigated.html",
            examples: "",
            return_value: "Returns TRUE if request was mitigated",
        }),
        forms: &[FormSpec {
            kind: FormKind::Default,
            synopsis: "DOSL7::is_mitigated",
        }],
        ..CommandSpec::DEFAULT
    }
}
