//! `ISTATS::remove` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "ISTATS::remove",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Removes the given iStat entirely.",
            &["ISTATS::remove KEY"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
