//! `IP::ingress_rate_limit` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "IP::ingress_rate_limit",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "F5 iRules command `IP::ingress_rate_limit`.",
            &["IP::ingress_rate_limit"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
