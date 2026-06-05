//! `xff_uniq_ordered_ip_list` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "xff_uniq_ordered_ip_list",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::new(0, 1),
hover: Some(HoverSnippet {
            summary: "Return a deduplicated list of valid non-loopback IP addresses from the X-Forwarded-For header, preserving the original header order.",
            synopsis: &["call xff_uniq_ordered_ip_list", "call xff_uniq_ordered_ip_list \"X-Real-IP\""],
            snippet: "Like `xff_uniq_sorted_ip_list` but preserves the original header order instead of sorting.  Use this when the order of IPs in the headers matters (e.g. tracing the proxy chain).  It comes with a memory and performance penalty over the sorted `xff_list` so should only be used when truly necessary.\n\n  - Entries that are not IPv4 or IPv6 are removed\n  - Both IPv4 and IPv6 addresses are collected and returned\n  - The order of the request IPs is preserved\n  - Duplicate IPs are collapsed\n  - FQDNs are not valid IPs and are therefore removed\n  - Loopback / zero addresses (`127.0.0.0/8`, `0.0.0.0/32`, `::/127`) are filtered out",
            source: "https://clouddocs.f5.com/api/irules/xff_uniq_ordered_ip_list.html",
            examples: "when HTTP_REQUEST priority 350 {\n    foreach ip [call xff_uniq_ordered_ip_list] {\n        if {[class match -- $ip eq \"blacklist-ips\"]} {\n            log local0. \"Blocking bad XFF IP: $ip\"\n            reject\n            return\n        }\n    }\n}",
            return_value: "A Tcl list of unique IP address strings in original header order.",
        }),
        event_requires: Some(EventRequires {
            client_side: false,
            server_side: false,
            transport: Some("tcp"),
            profiles: &["HTTP"],
            also_in: &[],
            init_only: false,
            flow: false,
            capability: None,
        }),
        forms: &[
            FormSpec { kind: FormKind::Default, synopsis: "xff_uniq_ordered_ip_list ?xff_header_name?" },
        ],
        side_effects: &[
            SideEffect {
                target: SideEffectTarget::HttpHeader,
                reads: true,
                writes: false,
                connection_side: ConnectionSide::Client,
            },
        ],
        ..CommandSpec::DEFAULT
    }
}
