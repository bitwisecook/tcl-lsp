//! `tmsh::show` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "tmsh::show",
        dialects: Some(DialectSet::IAPPS),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Runs the ``show`` command using the specified arguments.",
            &["tmsh::show ?component? ?name? ?options?"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
