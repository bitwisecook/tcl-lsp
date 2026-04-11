//! `DNS::query` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "DNS::query",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Returns or constructs and sends a query to the DNS-Express database for a name a",
            &["DNS::query ('dnsx' | 'dns-express') NAME DNS_TYPE (DNSSEC)?"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
