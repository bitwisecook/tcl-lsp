//! `radius_authenticate` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "radius_authenticate",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "radius_authenticate command creates a RADIUS access request message, sends to the given RADIUS server and returns the request result.",
            synopsis: &["radius_authenticate"],
            snippet: "radius_authenticate command creates a RADIUS access request message, sends to the given RADIUS server and returns the request result.",
            source: "https://clouddocs.f5.com/api/irules/radius_authenticate.html",
            examples: "",
            return_value: "",
        }),
        ..CommandSpec::DEFAULT
    }
}
