//! `COMPRESS::method` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "COMPRESS::method",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Specifies the preferred compression algorithm.",
            &["COMPRESS::method (request | response)? prefer ('gzip' | 'deflate')"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
