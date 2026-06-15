//! `IP::addr` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "IP::addr",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "IP address comparison.",
            synopsis: &[
                "IP::addr IP_ADDR_MASK 'equals' IP_ADDR_MASK",
                "IP::addr 'parse' ((('-swap')? BINARY_FIELD (OFFSET)?) |",
            ],
            snippet: "IP address comparison\n\nPerforms comparison of IP address/subnet/supernet to IP address/subnet/supernet.\n\nReturns 0 if no match, 1 for a match.\n\nUse of IP::addr is not necessary if the class (v10+) or matchclass (v9) command is used to perform the address-to-address comparison.\n\nDoes NOT perform a string comparison. To perform a literal string comparison, simply compare the 2 strings with the appropriate operator (equals, contains, starts_with, etc) rather than using the IP::addr comparison.\n\nFor versions 10.0 - 10.2.",
            source: "https://clouddocs.f5.com/api/irules/IP__addr.html",
            examples: "# To select a specific pool for a specific client IP address.\nwhen CLIENT_ACCEPTED {\n   if { [IP::addr [IP::client_addr] equals 10.10.10.10] } {\n      pool my_pool\n   }\n}",
            return_value: "Returns 0 IF NO MATCH, 1 for a match.",
        }),
        forms: &[FormSpec {
            kind: FormKind::Default,
            synopsis: "IP::addr IP_ADDR_MASK 'equals' IP_ADDR_MASK",
        }],
        options: &[
            OptionSpec {
                name: "-swap",
                takes_value: false,
                value_hint: "",
                detail: "Swap byte order.",
                dialects: None,
            },
            OptionSpec {
                name: "-ipv4",
                takes_value: false,
                value_hint: "",
                detail: "Parse as IPv4 address.",
                dialects: None,
            },
            OptionSpec {
                name: "-ipv6",
                takes_value: false,
                value_hint: "",
                detail: "Parse as IPv6 address.",
                dialects: None,
            },
        ],
        side_effects: &[SideEffect {
            target: SideEffectTarget::TcpState,
            reads: true,
            writes: false,
            connection_side: ConnectionSide::Both,
        }],
        ..CommandSpec::DEFAULT
    }
}
