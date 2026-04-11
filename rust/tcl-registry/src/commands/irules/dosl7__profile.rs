//! `DOSL7::profile` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "DOSL7::profile",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Returns the DOS profile from which the L7-DoS policy is extracted.",
            &["DOSL7::profile"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
