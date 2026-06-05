//! `LB::select` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "LB::select",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Forces a load balancing selection and returns the result.",
            synopsis: &["LB::select"],
            snippet: "This command forces the system to make a load balancing selection based on current conditions, and returns a string in the form of a pool command that can be eval'd to activate that selection.",
            source: "https://clouddocs.f5.com/api/irules/LB__select.html",
            examples: "",
            return_value: "",
        }),
        ..CommandSpec::DEFAULT
    }
}
