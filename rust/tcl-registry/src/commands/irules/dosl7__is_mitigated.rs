//! `DOSL7::is_mitigated` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "DOSL7::is_mitigated",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Returns TRUE if certain HTTP request was mitigated by DOSL7.",
            &["DOSL7::is_mitigated"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
