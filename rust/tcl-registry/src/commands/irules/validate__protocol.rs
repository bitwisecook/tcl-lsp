//! `VALIDATE::protocol` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "VALIDATE::protocol",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Performs validation of given application to match payload.",
            &["VALIDATE::protocol CLASSIFY_APP_NAME ANY_CHARS"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
