//! `XML::payload` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "XML::payload",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Queries for or manipulates XML payload.",
            &["XML::payload (LENGTH | (OFFSET LENGTH))?"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
