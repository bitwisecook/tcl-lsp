//! `whereis` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "whereis",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Returns geographical information on an IP address.",
            &["whereis (ldns | IP_ADDR)"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
