//! `SSL::respond` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "SSL::respond",
        traits: Traits::DIAGRAM_ACTION,
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Return data back to the origin via SSL.",
            &["SSL::respond DATA"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
