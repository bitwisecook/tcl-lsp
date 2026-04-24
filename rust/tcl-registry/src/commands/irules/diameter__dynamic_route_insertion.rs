//! `DIAMETER::dynamic_route_insertion` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "DIAMETER::dynamic_route_insertion",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Set whether dynamic route insertion is enabled.",
            &["DIAMETER::dynamic_route_insertion ( BOOLEAN )?"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
