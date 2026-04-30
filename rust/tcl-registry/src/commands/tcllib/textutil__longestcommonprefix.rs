//! `textutil::longestCommonPrefix` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "textutil::longestCommonPrefix",
        traits: Traits::PURE,
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Find the longest common prefix of strings.",
            &["textutil::longestCommonPrefix ?string ...?"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
