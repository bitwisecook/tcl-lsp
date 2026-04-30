//! `DIAG::test` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "DIAG::test",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "F5 iRules command `DIAG::test`.",
            &["DIAG::test"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
