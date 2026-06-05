//! `PSC::tower_id` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "PSC::tower_id",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Get or set tower id.",
            synopsis: &["PSC::tower_id (TOWER_ID)?"],
            snippet: "The PSC::tower_id command gets the tower id or sets the tower id when the optional value is given.",
            source: "https://clouddocs.f5.com/api/irules/PSC-tower-id.html",
            examples: "",
            return_value: "Return the tower id when no argument is given.",
        }),
        ..CommandSpec::DEFAULT
    }
}
