//! `PROFILE::exchange` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "PROFILE::exchange",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "F5 iRules command `PROFILE::exchange`.",
            synopsis: &["PROFILE::exchange ATTR"],
            snippet: "",
            source: "https://clouddocs.f5.com/api/irules/PROFILE__exchange.html",
            examples: "",
            return_value: "",
        }),
        ..CommandSpec::DEFAULT
    }
}
