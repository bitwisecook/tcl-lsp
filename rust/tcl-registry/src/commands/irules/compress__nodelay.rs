//! `COMPRESS::nodelay` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "COMPRESS::nodelay",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "F5 iRules command `COMPRESS::nodelay`.",
            &["COMPRESS::nodelay (request | response)?"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
