//! `DNS::return` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "DNS::return",
        traits: Traits::DIAGRAM_ACTION,
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Skips all further processing after TCL execution and sends the DNS packet in the",
            &["DNS::return"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
