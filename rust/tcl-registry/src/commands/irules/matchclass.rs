//! `matchclass` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "matchclass",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Performs comparison against the contents of data group.",
            &["matchclass CLASS_OR_VALUE KEYWORDS VALUE_OR_CLASS"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
