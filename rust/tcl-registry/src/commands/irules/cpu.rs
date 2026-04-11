//! `cpu` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "cpu",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Returns the average TMM cpu load for the given interval.",
            &["cpu usage ("],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
