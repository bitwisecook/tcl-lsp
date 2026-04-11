//! `RADIUS::code` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "RADIUS::code",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "This command returns the RADIUS message code",
            &["RADIUS::code"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
