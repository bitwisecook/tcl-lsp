//! `recv` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "recv",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Receives data from a given sideband connection.",
            &["recv (("],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
