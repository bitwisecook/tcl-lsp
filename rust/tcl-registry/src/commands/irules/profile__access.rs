//! `PROFILE::access` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "PROFILE::access",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "F5 iRules command `PROFILE::access`.",
            synopsis: &["PROFILE::access ATTR"],
            snippet: "",
            source: "https://clouddocs.f5.com/api/irules/PROFILE__access.html",
            examples: "",
            return_value: "",
        }),
        ..CommandSpec::DEFAULT
    }
}
