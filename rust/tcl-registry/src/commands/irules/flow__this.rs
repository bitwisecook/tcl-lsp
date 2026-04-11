//! `FLOW::this` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "FLOW::this",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Returns the TCL handle for the current flow.",
            &["FLOW::this"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
