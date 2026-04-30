//! `TCP::close` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "TCP::close",
        traits: Traits::DIAGRAM_ACTION,
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Closes the TCP connection.",
            &["TCP::close"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
