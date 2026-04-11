//! `DNS::rr` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "DNS::rr",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Creates a new resource record object with specified attributes or as a complete ",
            &["DNS::rr ANY_CHARS"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
