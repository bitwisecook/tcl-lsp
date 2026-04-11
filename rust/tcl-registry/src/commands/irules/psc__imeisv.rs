//! `PSC::imeisv` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "PSC::imeisv",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Get or set imeisv value.",
            &["PSC::imeisv (IMEISV)?"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
