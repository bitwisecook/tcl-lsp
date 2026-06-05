//! `LSN::inbound-entry` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "LSN::inbound-entry",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "This command creates and gets the inbound mapping for a translation address, translation port and protocol.",
            synopsis: &["LSN::inbound-entry (get | delete) IP_TUPLE IP_PROTOCOL", "LSN::inbound-entry create (-mirror)?"],
            snippet: "This command creates and gets the inbound mapping for a translation address, translation port and protocol.\n\nLSN::inbound-entry get <translation_address>:<translation_port> <protocol>\nLSN::inbound-entry create [-mirror] [-override] [-dslite <dslite local address> <dslite remote address>] [-prefix <IPv6 address>] <LSN pool name> <timeout> <client IP:client port> <translation address:translation port> <protocol>\n\nv11.5+\nLSN::inbound-entry delete <translation_address>:<translation_port> <protocol>",
            source: "https://clouddocs.f5.com/api/irules/LSN__inbound-entry.html",
            examples: "",
            return_value: "LSN::inbound-entry get <translation IP>:<translation port> <protocol> - Gets inbound entry for the specified translation IP, translation port and protocol. Protocol can be set TCP or UDP. This command returns the client IP address, port and route domain ID.",
        }),
        ..CommandSpec::DEFAULT
    }
}
