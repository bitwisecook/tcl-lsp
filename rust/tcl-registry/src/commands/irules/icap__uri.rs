//! `ICAP::uri` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "ICAP::uri",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Sets or returns the ICAP request URI.",
            &["ICAP::uri (URI_STRING)?"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
