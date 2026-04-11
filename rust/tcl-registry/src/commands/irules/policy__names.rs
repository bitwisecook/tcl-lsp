//! `POLICY::names` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "POLICY::names",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Returns details about the policy names for the virtual server the iRule is enabl",
            &["POLICY::names (active | matched | unmatched)"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
