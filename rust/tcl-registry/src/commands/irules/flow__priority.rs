//! `FLOW::priority` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "FLOW::priority",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Set/Get flow's internal packet priority.",
            &["FLOW::priority FLOW_PRIORITY"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
