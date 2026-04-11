//! `PSC::lease_time` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "PSC::lease_time",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Get the session lease time.",
            &["PSC::lease_time IP_ADDR (LEASE_TIME)?"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
