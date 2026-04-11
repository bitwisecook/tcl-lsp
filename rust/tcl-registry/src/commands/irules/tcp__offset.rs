//! `TCP::offset` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "TCP::offset",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Returns the number of bytes held in memory via TCP::collect.",
            &["TCP::offset"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
