//! `POLICY::controls` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "POLICY::controls",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Returns details about the policy controls for the virtual server the iRule is en",
            &["POLICY::controls ('acceleration' |"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
