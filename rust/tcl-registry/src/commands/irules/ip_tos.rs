//! `ip_tos` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "ip_tos",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Returns the ToS level of a packet.",
            &["ip_tos"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
