//! `PEM::disable` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "PEM::disable",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "PEM iRule command to disable PEM feature on current flow.",
            &["PEM::disable"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
