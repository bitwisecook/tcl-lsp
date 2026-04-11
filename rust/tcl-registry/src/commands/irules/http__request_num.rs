//! `HTTP::request_num` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "HTTP::request_num",
        traits: Traits::PURE | Traits::CSE_CANDIDATE,
        dialects: Some(DialectSet::IRULES),
        arity: Arity::exact(0),
        hover: Some(HoverSnippet::brief(
            "Returns the ordinal number of the current HTTP request on the connection.",
            &["HTTP::request_num"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
