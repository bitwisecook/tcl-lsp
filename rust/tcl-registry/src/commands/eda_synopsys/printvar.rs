//! `printvar` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "printvar",
        dialects: Some(DialectSet::SYNOPSYS),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Print the value of an application variable.",
            &["printvar ?variable_name?"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
