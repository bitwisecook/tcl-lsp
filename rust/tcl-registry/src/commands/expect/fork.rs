//! `fork` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "fork",
        dialects: Some(DialectSet::EXPECT),
        arity: Arity::exact(0),
        hover: Some(HoverSnippet::brief(
            "Fork the Expect process, returning 0 in the child and the child pid in the paren",
            &["fork"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
