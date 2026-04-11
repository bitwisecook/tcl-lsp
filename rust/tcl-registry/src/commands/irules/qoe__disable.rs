//! `QOE::disable` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "QOE::disable",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Deprecated: Disables the video QOE filter from processing any video or non-video",
            &["QOE::disable"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
