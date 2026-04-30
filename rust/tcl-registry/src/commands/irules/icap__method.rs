//! `ICAP::method` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "ICAP::method",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Returns the ICAP request method.",
            &["ICAP::method"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
