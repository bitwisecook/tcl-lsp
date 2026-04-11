//! `PROFILE::udp` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "PROFILE::udp",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Returns the value of a UDP profile setting.",
            &["PROFILE::udp ATTR"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
