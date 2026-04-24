//! `DIAMETER::payload` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "DIAMETER::payload",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Gets or sets DIAMETER message payload.",
            &["DIAMETER::payload ('replace' PAYLOAD)?"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
