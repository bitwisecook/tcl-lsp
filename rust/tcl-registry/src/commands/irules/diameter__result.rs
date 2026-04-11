//! `DIAMETER::result` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "DIAMETER::result",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Gets or sets the value of the result-code attribute-value pair.",
            &["DIAMETER::result (DIAMETER_RESULT_CODE)?"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
