//! `DOSL7::is_ip_slowdown` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "DOSL7::is_ip_slowdown",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Returns TRUE if source IP exists in greylist table",
            &["DOSL7::is_ip_slowdown"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
