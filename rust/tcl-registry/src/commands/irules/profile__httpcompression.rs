//! `PROFILE::httpcompression` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "PROFILE::httpcompression",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Returns the value of an HTTP compression profile setting.",
            synopsis: &["PROFILE::httpcompression ATTR"],
            snippet: "Returns the current value of the specified setting in an assigned HTTP compression profile.",
            source: "https://clouddocs.f5.com/api/irules/PROFILE__httpcompression.html",
            examples: "",
            return_value: "Returns the current value of the specified setting in an assigned HTTP compression profile.",
        }),
        ..CommandSpec::DEFAULT
    }
}
