//! `SSE::field` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "SSE::field",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Basic access and manipulation of SSE message fields",
            &["SSE::field ("],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
