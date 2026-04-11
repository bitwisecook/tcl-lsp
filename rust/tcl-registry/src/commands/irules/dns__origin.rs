//! `DNS::origin` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "DNS::origin",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Returns the originator of the DNS message.",
            &["DNS::origin"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
