//! `SIP::record-route` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "SIP::record-route",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "This command allows you get get information in the SIP record-route header.",
            &["SIP::record-route (INDEX | 'top')"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
