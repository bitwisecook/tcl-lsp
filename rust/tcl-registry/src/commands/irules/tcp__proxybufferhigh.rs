//! `TCP::proxybufferhigh` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "TCP::proxybufferhigh",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Gets proxy buffer high threshold.",
            &["TCP::proxybufferhigh"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
