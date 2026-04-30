//! `RESOLV::lookup` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "RESOLV::lookup",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Deprecated: The commands for making a DNS lookup.",
            &["RESOLV::lookup"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
