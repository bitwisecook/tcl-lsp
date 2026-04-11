//! `LB::mode` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "LB::mode",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Sets the load balancing mode, overriding the mode set in the pool definition.",
            &["LB::mode (default | rr | roundrobin)"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
