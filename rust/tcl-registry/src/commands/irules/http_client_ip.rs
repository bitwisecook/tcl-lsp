//! `http_client_ip` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "http_client_ip",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::new(0, 1),
hover: Some(HoverSnippet {
            summary: "Return the first IP address from X-Forwarded-For (or a named header), otherwise the L3 client IP address.",
            synopsis: &["call http_client_ip", "call http_client_ip \"True-Client-IP\""],
            snippet: "Returns the first valid, non-loopback IP from the specified forwarding header (default `X-Forwarded-For`), falling back to `IP::client_addr` when no suitable address is found.\n\nHTTP headers are case-insensitive, so `x-forwarded-for` == `X-FORWARDED-FOR` == `X-Forwarded-For`.\n\nFor a client `9.9.9.9` with headers `X-Forwarded-For: 1.1.1.1,2.2.2.2` and `X-Forwarded-For: 3.3.3.3,4.4.4.4`, returns `1.1.1.1`.  With no forwarding header, returns `9.9.9.9`.\n\nUses `catch {clientside {HTTP::version}}` as a lightweight `HTTP::has_responded` equivalent compatible with TMOS < 14.1.\n\nLoopback / zero addresses filtered out:\n  - `127.0.0.0/8`\n  - `0.0.0.0/32`\n  - `::/127`",
            source: "https://clouddocs.f5.com/api/irules/http_client_ip.html",
            examples: "when HTTP_REQUEST priority 500 {\n    # Rate-limit by real client IP\n    table set pfx-[call http_client_ip] 1 180 180\n}\n\n# Use a custom header name\nwhen HTTP_REQUEST priority 500 {\n    table set pfx-[call http_client_ip True-Client-IP] 1 180 180\n}",
            return_value: "A single IP address string.",
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
            FormSpec { kind: FormKind::Default, synopsis: "http_client_ip ?xff_header_name?" },
        ],
        ..CommandSpec::DEFAULT
    }
}
