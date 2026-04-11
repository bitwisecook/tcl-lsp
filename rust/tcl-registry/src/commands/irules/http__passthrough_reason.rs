//! `HTTP::passthrough_reason` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "HTTP::passthrough_reason",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Returns the reason for the most recent switch to pass-through mode by the HTTP f",
            &["HTTP::passthrough_reason ('as_num')?"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
