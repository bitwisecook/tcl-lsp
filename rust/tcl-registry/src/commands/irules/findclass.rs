//! `findclass` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "findclass",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Searches a data group list for a member that starts with the specified string an",
            &["findclass STRING DATA_GROUP (SEPARATOR)?"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
