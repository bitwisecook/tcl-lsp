//! `link_qos` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "link_qos",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Returns the QoS level.",
            &["link_qos (QOS_LEVEL)?"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
