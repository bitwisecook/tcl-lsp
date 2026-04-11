//! `TDS::session` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "TDS::session",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Returns TDS session data.",
            &["TDS::session"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
