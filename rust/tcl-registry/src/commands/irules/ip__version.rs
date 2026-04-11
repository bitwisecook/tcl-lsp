//! `IP::version` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "IP::version",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Returns the IP version of a connection.",
            &["IP::version"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
