//! `IP::intelligence` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "IP::intelligence",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Return a Tcl list of IP intelligence category names for a given IP address.",
            &["IP::intelligence IP_ADDR"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
