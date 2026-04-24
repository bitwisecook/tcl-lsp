//! `DSLITE::remote_addr` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "DSLITE::remote_addr",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Returns the remote DS-Lite tunnel.",
            &["DSLITE::remote_addr"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
