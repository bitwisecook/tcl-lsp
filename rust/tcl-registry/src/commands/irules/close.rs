//! `close` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "close",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::new(1, 2),
        hover: Some(HoverSnippet::brief(
            "Closes an existing sideband connection.",
            &["close CONNECTION"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
