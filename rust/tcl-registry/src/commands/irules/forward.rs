//! `forward` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "forward",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Sets the connection to forward IP packets.",
            &["forward"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
