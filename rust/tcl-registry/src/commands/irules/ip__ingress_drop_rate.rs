//! `IP::ingress_drop_rate` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "IP::ingress_drop_rate",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Adds ip with specified drop rate to black list table.",
            &["IP::ingress_drop_rate"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
