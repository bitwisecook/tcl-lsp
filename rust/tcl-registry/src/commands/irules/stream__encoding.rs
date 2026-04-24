//! `STREAM::encoding` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "STREAM::encoding",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Specifies non-default content encoding.",
            &["STREAM::encoding (ascii | utf-8 | unicode)"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
