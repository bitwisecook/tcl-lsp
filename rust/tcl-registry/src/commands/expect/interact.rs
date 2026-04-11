//! `interact` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "interact",
        dialects: Some(DialectSet::EXPECT),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Give control of the current process to the user for interactive use.",
            &["interact ?-opts? ?string body ...?"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
