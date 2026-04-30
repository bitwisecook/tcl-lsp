//! `ADAPT::context_delete_all` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "ADAPT::context_delete_all",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Deletes all dynamic contexts.",
            &["ADAPT::context_delete_all"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
