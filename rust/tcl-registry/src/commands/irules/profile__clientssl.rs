//! `PROFILE::clientssl` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "PROFILE::clientssl",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Returns the value of a Client SSL profile setting.",
            synopsis: &["PROFILE::clientssl ATTR"],
            snippet: "Returns the current value of the specified setting in the assigned Client SSL profile.",
            source: "https://clouddocs.f5.com/api/irules/PROFILE__clientssl.html",
            examples: "",
            return_value: "Returns the current value of the specified setting in the assigned Client SSL profile.",
        }),
        ..CommandSpec::DEFAULT
    }
}
