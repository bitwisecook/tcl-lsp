//! `SSL::verify_result` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "SSL::verify_result",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Gets or sets the result code for peer certificate verification.",
            &["SSL::verify_result (RESULT_CODE)?"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
