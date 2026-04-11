//! `RADIUS::subscriber` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "RADIUS::subscriber",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "RADIUS::subscriber",
            &["RADIUS::subscriber (SUBSCRIBER_ID)?"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
