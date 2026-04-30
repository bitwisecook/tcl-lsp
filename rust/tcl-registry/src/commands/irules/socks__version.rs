//! `SOCKS::version` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "SOCKS::version",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "This command gets the version of the SOCKS protocol.",
            &["SOCKS::version"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
