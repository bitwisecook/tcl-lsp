//! `FIX::tag` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "FIX::tag",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Defines/deletes the mapping between senderCompID and a tag map data group.",
            &["FIX::tag map set SENDER DATA_GROUP"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
