//! `safe::setLogCmd` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "safe::setLogCmd",
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Set or query the logging command for Safe Base messages.",
            &["safe::setLogCmd ?cmd arg...?"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
