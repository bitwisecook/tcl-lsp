//! `ASM::raise` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "ASM::raise",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Issues a user-defined violation on the request.",
            &["ASM::raise VIOLATION_NAME (VIOLATION_DETAILS)?"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
