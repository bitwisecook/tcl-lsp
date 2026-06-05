//! `LB::class` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "LB::class",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Provides the name of the traffic class that matched the connection",
            synopsis: &["LB::class"],
            snippet: "Provides the name of the traffic class that matched the connection",
            source: "https://clouddocs.f5.com/api/irules/LB__class.html",
            examples: "",
            return_value: "Return the name of the traffic class that matched the connection",
        }),
        ..CommandSpec::DEFAULT
    }
}
