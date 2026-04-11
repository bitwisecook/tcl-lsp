//! `DOSL7::health` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "DOSL7::health",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Returns DOSL7 server health value for current virtual server",
            &["DOSL7::health"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
