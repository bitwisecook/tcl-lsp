//! `log` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "log",
        traits: Traits::DIAGRAM_ACTION,
        dialects: Some(DialectSet::IRULES),
        arity: Arity::new(1, 2),
        hover: Some(HoverSnippet::brief(
            "Write a message to BIG-IP logging facilities.",
            &["log ?facility.level? message"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
