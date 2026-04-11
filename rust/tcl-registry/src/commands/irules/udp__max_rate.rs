//! `UDP::max_rate` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "UDP::max_rate",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "This command can be used to set/get the maximum transmission rate (bytes per sec",
            &["UDP::max_rate (UDP_MAX_RATE)?"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
