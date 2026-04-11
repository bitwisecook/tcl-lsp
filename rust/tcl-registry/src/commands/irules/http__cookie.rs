//! `HTTP::cookie` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "HTTP::cookie",
        traits: Traits::PURE | Traits::CSE_CANDIDATE | Traits::DIAGRAM_ACTION,
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Queries for or manipulates cookies in HTTP requests and responses.",
            &["HTTP::cookie <subcommand> ?arg ...?"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
