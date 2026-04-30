//! `accumulate` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "accumulate",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::any(),
        hover: Some(HoverSnippet::brief(
            "Deprecated: use TCP::collect instead",
            &["accumulate"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
