//! `SOCKS::allowed` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "SOCKS::allowed",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "This command allows you to change whether the SOCKS request is allowed or not.",
            &["SOCKS::allowed ('0' | '1')?"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
