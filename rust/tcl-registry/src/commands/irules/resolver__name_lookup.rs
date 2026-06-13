//! `RESOLVER::name_lookup` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "RESOLVER::name_lookup",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "This command performs a DNS lookup.",
            synopsis: &["RESOLVER::name_lookup NET_RESOLVER_NAME NAME TYPE"],
            snippet: "RESOLVER::name_lookup performs a DNS query for a name and type using the network resolver specified.\n\nThis command allows queries for all resource record types and provides access to the DNS services available on the BIGIP.",
            source: "https://clouddocs.f5.com/api/irules/RESOLV-lookup.html",
            examples: "when CLIENT_ACCEPTED {\n        set result [RESOLVER::name_lookup \"/Common/r1\" www.abc.com a]\n        set result [RESOLVER::name_lookup \"/Common/r1\" 206.6.177.2 ptr]\n}",
            return_value: "The response will be a dns_message structure usable by the RESOLVER::summarize, DNSMSG::header, and DNSMSG::section iRule commands.",
        }),
        forms: &[
            FormSpec { kind: FormKind::Default, synopsis: "RESOLVER::name_lookup NET_RESOLVER_NAME NAME TYPE" },
        ],
        side_effects: &[
            SideEffect {
                target: SideEffectTarget::DnsState,
                reads: true,
                writes: false,
                connection_side: ConnectionSide::Global,
            },
        ],
        ..CommandSpec::DEFAULT
    }
}
