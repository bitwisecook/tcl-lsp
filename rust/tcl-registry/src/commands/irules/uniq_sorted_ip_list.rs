//! `uniq_sorted_ip_list` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "uniq_sorted_ip_list",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Return a sorted, deduplicated list of valid IP addresses extracted from the given arguments.",
            synopsis: &["call uniq_sorted_ip_list $ip_string", "call uniq_sorted_ip_list 1.1.1.1 {2.2.2.2, 3.3.3.3} 5.5.5.5"],
            snippet: "Like `xff_list` but takes a list of potential IPs as an argument rather than reading from an HTTP header.\n\nThe list may be nested and may contain commas or spaces as delimiters.\n\n  - Entries that are not IPv4 or IPv6 are removed\n  - The result is sorted; duplicate IPs are collapsed\n  - Both IPv4 and IPv6 addresses are collected and returned\n  - FQDNs are not valid IPs and are therefore removed\n\nUnlike the `xff_*` variants, this proc does **not** filter out loopback or zero addresses.",
            source: "https://clouddocs.f5.com/api/irules/uniq_sorted_ip_list.html",
            examples: "when HTTP_REQUEST priority 350 {\n    foreach ip [call uniq_sorted_ip_list 1.1.1.1 {2.2.2.2, 3.3.3.3} 2a01:4b00:8480:ae00:acf0:fe84:3bf2:eeee badentry 5.5.5.5] {\n        if {[class match -- $ip eq \"blacklist-ips\"]} {\n            reject\n            return\n        }\n    }\n}",
            return_value: "A Tcl list of unique, sorted IP address strings.",
        }),
        forms: &[
            FormSpec { kind: FormKind::Default, synopsis: "uniq_sorted_ip_list ?arg ...?" },
        ],
        side_effects: &[
            SideEffect {
                target: SideEffectTarget::Unknown,
                reads: true,
                writes: false,
                connection_side: ConnectionSide::Global,
            },
        ],
        ..CommandSpec::DEFAULT
    }
}
