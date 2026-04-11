//! `SSL::maximum_record_size` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "SSL::maximum_record_size",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Get or set the maximum egress record size.",
            &["SSL::maximum_record_size (SSL_RECORD_SIZE)?"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
