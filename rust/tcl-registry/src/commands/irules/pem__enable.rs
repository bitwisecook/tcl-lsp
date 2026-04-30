//! `PEM::enable` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "PEM::enable",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "PEM iRule command to enable PEM feature on current flow.",
            &["PEM::enable"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
