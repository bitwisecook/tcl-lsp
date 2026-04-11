//! `transcript` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "transcript",
        dialects: Some(DialectSet::MENTOR),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Control transcript file output.",
            &["transcript ?on | off | file file_name?"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
