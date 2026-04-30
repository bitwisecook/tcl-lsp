//! `DIAMETER::message` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "DIAMETER::message",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Returns the whole Diameter message as a TCL string object.",
            &["DIAMETER::message"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
