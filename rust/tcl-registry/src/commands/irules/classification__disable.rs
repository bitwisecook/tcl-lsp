//! `CLASSIFICATION::disable` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "CLASSIFICATION::disable",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Deprecated: Disables classification for the current flow.",
            synopsis: &["CLASSIFICATION::disable"],
            snippet: "This command disables classification for the current flow.\n\nCLASSIFICATION::disable",
            source: "https://clouddocs.f5.com/api/irules/CLASSIFICATION__disabled.html",
            examples: "",
            return_value: "",
        }),
        ..CommandSpec::DEFAULT
    }
}
