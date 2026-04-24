//! `STREAM::replace` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "STREAM::replace",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Changes a replacement string in the Stream profile.",
            &["STREAM::replace (TARGET_STRING)?"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
