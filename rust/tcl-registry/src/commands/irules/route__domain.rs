//! `ROUTE::domain` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "ROUTE::domain",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Returns the current routing domain of the current connection.",
            &["ROUTE::domain"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
