//! `DNS::ptype` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "DNS::ptype",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Returns the type of the DNS packet.",
            &["DNS::ptype"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
