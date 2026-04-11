//! `HTTP::reject_reason` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "HTTP::reject_reason",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Returns the reason HTTP is aborting",
            &["HTTP::reject_reason ('as_num')?"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
