//! `IP::addr` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "IP::addr",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "IP address comparison.",
            &["IP::addr IP_ADDR_MASK 'equals' IP_ADDR_MASK"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
