//! `SOCKS::destination` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "SOCKS::destination",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "This command allows you to get or set the SOCKS destination host or port.",
            &["SOCKS::destination ('host')? (HOST_ADDRESS)?"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
