//! `AUTH::start` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "AUTH::start",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Initializes an authentication session.",
            &["AUTH::start TYPE SERVICE"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
