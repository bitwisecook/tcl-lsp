//! `HTTP::class` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "HTTP::class",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::new(0, 2),
        hover: Some(HoverSnippet::brief(
            "Returns or sets the HTTP class selected by the HTTP selector.",
            &["HTTP::class"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
