//! `exit` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "exit",
        dialects: Some(DialectSet::EXPECT),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Exit Expect, optionally running an onexit handler.",
            &["exit ?-onexit command? ?status?"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
