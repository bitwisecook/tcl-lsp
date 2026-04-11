//! `DNS::rpz_policy` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "DNS::rpz_policy",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Returns the RPZ policy associated with the DNS cache.",
            &["DNS::rpz_policy"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
