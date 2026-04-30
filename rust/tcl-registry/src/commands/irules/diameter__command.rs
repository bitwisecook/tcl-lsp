//! `DIAMETER::command` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "DIAMETER::command",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Gets or sets the command-code.",
            &["DIAMETER::command (DIAMETER_COMMAND_CODE)?"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
