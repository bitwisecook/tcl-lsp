//! `ECA::disable` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "ECA::disable",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Disables the plugin in the flow.",
            synopsis: &["ECA::disable"],
            snippet: "The ECA::disable command disables the plugin in the flow.",
            source: "https://clouddocs.f5.com/api/irules/ECA__disable.html",
            examples: "",
            return_value: "",
        }),
        ..CommandSpec::DEFAULT
    }
}
