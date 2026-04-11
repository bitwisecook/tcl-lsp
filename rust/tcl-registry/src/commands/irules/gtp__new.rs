//! `GTP::new` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "GTP::new",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Creates a new GTP message for given version & request-type.",
            &["GTP::new VERSION TYPE"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
