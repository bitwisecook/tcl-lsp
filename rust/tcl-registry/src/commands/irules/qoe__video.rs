//! `QOE::video` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "QOE::video",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Deprecated: Returns a set of video QOE attributes from the current video connect",
            &["QOE::video"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
