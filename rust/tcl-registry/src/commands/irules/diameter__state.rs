//! `DIAMETER::state` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "DIAMETER::state",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Returns the current state of the Diameter peer's connection.",
            &["DIAMETER::state"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
