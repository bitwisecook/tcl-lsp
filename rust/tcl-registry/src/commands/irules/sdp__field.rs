//! `SDP::field` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "SDP::field",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Gets or Sets the value in a given SDP field.",
            &["SDP::field FIELD_NAME ((INDEX) (NEW_VALUE)?)?"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
