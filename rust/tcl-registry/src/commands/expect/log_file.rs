//! `log_file` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "log_file",
        dialects: Some(DialectSet::EXPECT),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Control logging of session output to a file.",
            &["log_file ?-option ...? ?file?"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
