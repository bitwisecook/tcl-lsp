//! `SIP::route` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "SIP::route",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Gets SIP route header information.",
            &["SIP::route (INDEX | 'top')"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
