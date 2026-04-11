//! `ECA::client_machine_name` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "ECA::client_machine_name",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Returns NTLM authenticating user's machine name.",
            &["ECA::client_machine_name"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
