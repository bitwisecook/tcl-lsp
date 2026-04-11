//! `snat` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "snat",
        traits: Traits::DIAGRAM_ACTION,
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Assigns the specified SNAT translation address to the current connection.",
            &["snat (automap | none | IP_TUPLE | (IP_ADDR (PORT)?))"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
