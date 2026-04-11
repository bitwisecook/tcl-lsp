//! `BOTDEFENSE::cookie_status` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "BOTDEFENSE::cookie_status",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Returns the status of the Bot Defense cookie.",
            &["BOTDEFENSE::cookie_status"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
