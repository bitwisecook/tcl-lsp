//! `ICAP::header` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "ICAP::header",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Sets or returns ICAP attributes in the ICAP header.",
            &["ICAP::header 'names'"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
