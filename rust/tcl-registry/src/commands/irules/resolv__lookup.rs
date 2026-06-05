//! `RESOLV::lookup` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "RESOLV::lookup",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Deprecated: The commands for making a DNS lookup.",
            synopsis: &["RESOLV::lookup"],
            snippet: "RESOLV::lookup performs a DNS query, returning one or more addresses (A records) for a hostname, a domain name (PTR record) for an IP address, or optionally one or more values for records of other types.",
            source: "https://clouddocs.f5.com/api/irules/RESOLV__lookup.html",
            examples: "",
            return_value: "",
        }),
        ..CommandSpec::DEFAULT
    }
}
