//! `ECA::domainname` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "ECA::domainname",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Returns NTLM authenticating user's domain name.",
            &["ECA::domainname"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
