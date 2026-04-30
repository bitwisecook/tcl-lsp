//! `ADAPT::preview_size` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "ADAPT::preview_size",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Sets or returns the preview-size attribute.",
            &["ADAPT::preview_size (ADAPT_CTX)? (ADAPT_SIDE)? (SIZE)?"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
