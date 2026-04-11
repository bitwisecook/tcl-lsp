//! `URI::encode_component` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "URI::encode_component",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Percent-encodes a single URI component.",
            &["URI::encode_component STRING"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
