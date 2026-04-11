//! `POLICY::targets` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "POLICY::targets",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Returns or sets properties of the policy rule targets for the policies associate",
            &["POLICY::targets ('ltm-policy' |"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
