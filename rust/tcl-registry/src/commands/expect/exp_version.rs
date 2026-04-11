//! `exp_version` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "exp_version",
        dialects: Some(DialectSet::EXPECT),
        arity: Arity::new(0, 1),
        hover: Some(HoverSnippet::brief(
            "Query or require a minimum Expect version.",
            &["exp_version ?version?"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
