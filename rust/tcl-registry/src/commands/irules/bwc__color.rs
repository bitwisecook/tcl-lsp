//! `BWC::color` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "BWC::color",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "This command is used to classify a traffic flow to a particular color (applicati",
            &["BWC::color ('set' | 'unset') POLICY_NAME APPLICATION_NAME"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
