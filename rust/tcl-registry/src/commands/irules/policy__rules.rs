//! `POLICY::rules` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "POLICY::rules",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Returns the policy rules of the supplied policy that had actions executed.",
            &["POLICY::rules ('matched')? POLICY_NAME"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
