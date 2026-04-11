//! `REWRITE::post_process` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "REWRITE::post_process",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Toggle post processing functionality.",
            &["REWRITE::post_process (SWITCH)?"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
