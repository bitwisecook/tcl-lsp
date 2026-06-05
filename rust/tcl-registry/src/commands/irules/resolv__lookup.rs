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
        forms: &[
            FormSpec { kind: FormKind::Default, synopsis: "RESOLV::lookup ?@nameserver? ?-type? hostname" },
        ],
        options: &[
            OptionSpec { name: "-a", takes_value: false, value_hint: "", detail: "Query for type A (IPv4) records.", dialects: None },
            OptionSpec { name: "-aaaa", takes_value: false, value_hint: "", detail: "Query for type AAAA (IPv6) records.", dialects: None },
            OptionSpec { name: "-ptr", takes_value: false, value_hint: "", detail: "Query for PTR records.", dialects: None },
            OptionSpec { name: "-txt", takes_value: false, value_hint: "", detail: "Query for TXT records.", dialects: None },
            OptionSpec { name: "-mx", takes_value: false, value_hint: "", detail: "Query for MX records.", dialects: None },
        ],
        ..CommandSpec::DEFAULT
    }
}
