//! `RESOLVER::name_lookup` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "RESOLVER::name_lookup",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "This command performs a DNS lookup.",
            &["RESOLVER::name_lookup NET_RESOLVER_NAME NAME TYPE"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
