//! `DIAMETER::retry` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "DIAMETER::retry",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Tries to send the Diameter message contained in the binary array \"binary_message",
            &["DIAMETER::retry DIAMETER_MESSAGE (BOOL_ACROSS)?"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
