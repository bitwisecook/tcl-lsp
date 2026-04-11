//! `LB::snat` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "LB::snat",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Returns information on the SNAT configuration for the current connection.",
            &["LB::snat"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
