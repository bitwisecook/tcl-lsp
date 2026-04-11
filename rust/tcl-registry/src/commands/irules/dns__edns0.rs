//! `DNS::edns0` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "DNS::edns0",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Gets (v11.0+) and sets (v11.1+) the values of the edns0 pseudo-RR.",
            &["DNS::edns0 'remove' ('nsid' | 'subnet')?"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
