//! `set_db` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "set_db",
        dialects: Some(DialectSet::CADENCE),
        arity: Arity::at_least(2),
        hover: Some(HoverSnippet::brief(
            "Set a database attribute value.",
            &["set_db object_or_attr value"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
