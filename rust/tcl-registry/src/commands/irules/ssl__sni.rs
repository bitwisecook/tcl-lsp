//! `SSL::sni` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "SSL::sni",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Returns Server Name Indication information.",
            &["SSL::sni (name | required)"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
