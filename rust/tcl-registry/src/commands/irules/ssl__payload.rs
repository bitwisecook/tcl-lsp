//! `SSL::payload` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "SSL::payload",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Returns and manipulates plaintext data collected via SSL::collect.",
            &["SSL::payload (length |"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
