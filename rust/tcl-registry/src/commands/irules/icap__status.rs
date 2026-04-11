//! `ICAP::status` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "ICAP::status",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Returns the ICAP response status code.",
            &["ICAP::status"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
