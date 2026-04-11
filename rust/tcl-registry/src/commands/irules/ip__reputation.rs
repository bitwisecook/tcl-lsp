//! `IP::reputation` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "IP::reputation",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Looks up the supplied IP address in the IP intelligence (reputation) database an",
            &["IP::reputation (IP_ADDR)+"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
