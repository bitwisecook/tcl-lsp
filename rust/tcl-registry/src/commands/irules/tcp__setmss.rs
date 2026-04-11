//! `TCP::setmss` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "TCP::setmss",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Sets the TCP max segment size.",
            &["TCP::setmss TCP_MAX_SEGMENT_SIZE"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
