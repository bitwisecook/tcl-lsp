//! `ip_addr` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "ip_addr",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::any(),
        hover: Some(HoverSnippet::brief(
            "Deprecated: use IP::addr instead",
            &["ip_addr"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
