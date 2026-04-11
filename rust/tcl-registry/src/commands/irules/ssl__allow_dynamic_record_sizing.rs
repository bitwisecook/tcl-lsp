//! `SSL::allow_dynamic_record_sizing` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "SSL::allow_dynamic_record_sizing",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Get or set dynamic record sizing.",
            &["SSL::allow_dynamic_record_sizing (ZERO_ONE)?"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
