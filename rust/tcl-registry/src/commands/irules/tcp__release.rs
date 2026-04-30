//! `TCP::release` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "TCP::release",
        traits: Traits::DIAGRAM_ACTION,
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Release data gathered by TCP::collect to the upper layer.",
            &["TCP::release (LENGTH)?"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
