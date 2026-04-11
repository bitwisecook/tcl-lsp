//! `COMPRESS::gzip` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "COMPRESS::gzip",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Sets HTTP data compression criteria.",
            &["COMPRESS::gzip (request | response)? ("],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
