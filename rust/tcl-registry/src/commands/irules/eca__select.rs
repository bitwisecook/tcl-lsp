//! `ECA::select` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "ECA::select",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Selects one of NTLM Authentication configuration name and Kerberos Authenticatio",
            &["ECA::select"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
