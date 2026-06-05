//! `PROFILE::diameter` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "PROFILE::diameter",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Returns the current value of the specified setting in an assigned DIAMETER profile.",
            synopsis: &["PROFILE::diameter ATTR"],
            snippet: "Returns the current value of the specified setting in an assigned DIAMETER profile.",
            source: "https://clouddocs.f5.com/api/irules/PROFILE__diameter.html",
            examples: "",
            return_value: "Returns the current value of the specified setting in an assigned DIAMETER profile.",
        }),
        ..CommandSpec::DEFAULT
    }
}
