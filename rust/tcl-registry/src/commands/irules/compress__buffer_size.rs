//! `COMPRESS::buffer_size` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "COMPRESS::buffer_size",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Sets the compression buffer size.",
            &["COMPRESS::buffer_size (request | response)? NONNEGATIVE_INTEGER"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
