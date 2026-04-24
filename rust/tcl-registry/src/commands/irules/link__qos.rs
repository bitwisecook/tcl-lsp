//! `LINK::qos` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "LINK::qos",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Returns the QoS level set for the current packet.",
            &["LINK::qos"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
