//! `json::validate` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "json::validate",
        traits: Traits::PURE,
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::exact(1),
        return_type: Some(TclType::Boolean),
        hover: Some(HoverSnippet::brief(
            "Validate whether a string is valid JSON.",
            &["json::validate jsonText"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
