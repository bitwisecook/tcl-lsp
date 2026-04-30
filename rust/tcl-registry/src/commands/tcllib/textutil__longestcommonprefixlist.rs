//! `textutil::longestCommonPrefixList` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "textutil::longestCommonPrefixList",
        traits: Traits::PURE,
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::exact(1),
        hover: Some(HoverSnippet::brief(
            "Find the longest common prefix of a list of strings.",
            &["textutil::longestCommonPrefixList list"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
