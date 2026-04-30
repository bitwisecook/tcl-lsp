//! `SSL::allow_nonssl` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "SSL::allow_nonssl",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Get or set Allow Non-SSL connections.",
            &["SSL::allow_nonssl (ZERO_ONE)?"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
