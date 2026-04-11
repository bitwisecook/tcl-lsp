//! `REWRITE::enable` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "REWRITE::enable",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Changes the REWRITE plugin from passthrough to full patching mode.",
            &["REWRITE::enable"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
