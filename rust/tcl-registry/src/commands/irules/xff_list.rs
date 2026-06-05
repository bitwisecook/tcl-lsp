//! `xff_list` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "xff_list",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::new(0, 1),
hover: Some(HoverSnippet {
            summary: "Return a sorted, deduplicated list of valid non-loopback IP addresses from the X-Forwarded-For header.",
            synopsis: &["call xff_list", "call xff_list \"X-Real-IP\""],
            snippet: "Also known as `xff_uniq_sorted_ip_list`.  Collects all addresses from the named header (default `X-Forwarded-For`), even when the header appears multiple times.\n\n  - Entries that are not IPv4 or IPv6 are removed\n  - Both IPv4 and IPv6 addresses are collected and returned\n  - The result is sorted; duplicate IPs are collapsed\n  - FQDNs are not valid IPs and are therefore removed\n  - Loopback / zero addresses (`127.0.0.0/8`, `0.0.0.0/32`, `::/127`) are filtered out\n\nUseful for geolocation or blacklist lookups against forwarded addresses.",
            source: "https://clouddocs.f5.com/api/irules/xff_list.html",
            examples: "when HTTP_REQUEST priority 350 {\n    foreach ip [call xff_list] {\n        if {[class match -- $ip eq \"blacklist-ips\"]} {\n            log local0. \"Blocking bad XFF IP: $ip\"\n            reject\n            return\n        }\n    }\n}",
            return_value: "A Tcl list of unique, sorted IP address strings.",
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
        ..CommandSpec::DEFAULT
    }
}
