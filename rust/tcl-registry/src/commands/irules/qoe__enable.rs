//! `QOE::enable` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "QOE::enable",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Deprecated: Enables the video QOE filter and allows processing video on a connec",
            &["QOE::enable"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
