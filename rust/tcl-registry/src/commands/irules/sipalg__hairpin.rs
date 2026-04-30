//! `SIPALG::hairpin` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "SIPALG::hairpin",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Gets or sets the value of hairpin flag for the current message.",
            &["SIPALG::hairpin"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
