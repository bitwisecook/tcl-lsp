//! `BWC::mark` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "BWC::mark",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "This command allows you to set or unset marking for traffic flows in bwc when co",
            &["BWC::mark SESSION_ID ('qos'|'tos') (BWC_VALUE | 'passthrough')"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
