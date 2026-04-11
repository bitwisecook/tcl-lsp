//! `PSC::imsi` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "PSC::imsi",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Get or set the imsi value.",
            &["PSC::imsi (IMSI)?"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
