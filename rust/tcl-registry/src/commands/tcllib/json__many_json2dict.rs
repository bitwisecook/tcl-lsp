//! `json::many-json2dict` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "json::many-json2dict",
        traits: Traits::PURE,
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::new(1, 2),
        return_type: Some(TclType::List),
        hover: Some(HoverSnippet::brief(
            "Convert a string containing multiple JSON values to a list of dicts.",
            &["json::many-json2dict jsonText ?max?"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
