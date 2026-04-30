//! `check_library` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "check_library",
        dialects: Some(DialectSet::SYNOPSYS),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Check libraries for consistency issues.",
            &["check_library ?library_list?"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
