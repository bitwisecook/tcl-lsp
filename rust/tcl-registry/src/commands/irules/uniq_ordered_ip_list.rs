//! `uniq_ordered_ip_list` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "uniq_ordered_ip_list",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Return a deduplicated list of valid IP addresses from the given arguments, preserving the original order.",
            synopsis: &["call uniq_ordered_ip_list $ip_string", "call uniq_ordered_ip_list 1.1.1.1 {2.2.2.2, 3.3.3.3} 5.5.5.5"],
            snippet: "Like `xff_uniq_ordered_ip_list` but takes a list of potential IPs as an argument rather than reading from an HTTP header.\n\nThe list may be nested and may contain commas or spaces as delimiters.  It comes with a memory and performance penalty over the sorted `uniq_sorted_ip_list` so should only be used when order truly matters.\n\n  - Entries that are not IPv4 or IPv6 are removed\n  - The order of the input IPs is preserved\n  - Both IPv4 and IPv6 addresses are collected and returned\n  - Duplicate IPs are collapsed\n  - FQDNs are not valid IPs and are therefore removed\n\nUnlike the `xff_*` variants, this proc does **not** filter out loopback or zero addresses.",
            source: "https://clouddocs.f5.com/api/irules/uniq_ordered_ip_list.html",
            examples: "when HTTP_REQUEST priority 350 {\n    foreach ip [call uniq_ordered_ip_list 1.1.1.1 {2.2.2.2, 3.3.3.3} 2a01:4b00:8480:ae00:acf0:fe84:3bf2:eeee badentry 5.5.5.5] {\n        if {[class match -- $ip eq \"blacklist-ips\"]} {\n            reject\n            return\n        }\n    }\n}",
            return_value: "A Tcl list of unique IP address strings in original input order.",
        }),
        forms: &[
            FormSpec { kind: FormKind::Default, synopsis: "uniq_ordered_ip_list ?arg ...?" },
        ],
        ..CommandSpec::DEFAULT
    }
}
