//! `ip_protocol` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "ip_protocol",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Returns the IP protocol value.",
            &["ip_protocol"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
