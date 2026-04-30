//! `exp_pid` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "exp_pid",
        traits: Traits::PURE,
        dialects: Some(DialectSet::EXPECT),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Return the process id of a spawned process.",
            &["exp_pid ?-i spawn_id?"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
