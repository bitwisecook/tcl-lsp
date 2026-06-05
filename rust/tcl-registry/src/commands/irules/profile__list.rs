//! `PROFILE::list` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "PROFILE::list",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Returns all the names of the profiles of the class asked for that are attached to this virtual server.",
            synopsis: &["PROFILE::list 'auth'"],
            snippet: "This command returns all the names of the profiles of the class asked for that are attached to this virtual server.",
            source: "https://clouddocs.f5.com/api/irules/PROFILE__list.html",
            examples: "",
            return_value: "Returns all the names of the profiles of the class asked for that are attached to this virtual server",
        }),
        ..CommandSpec::DEFAULT
    }
}
