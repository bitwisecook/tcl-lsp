//! `REWRITE::disable` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "REWRITE::disable",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Changes the REWRITE plugin from full patching mode to passthrough mode.",
            &["REWRITE::disable"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
