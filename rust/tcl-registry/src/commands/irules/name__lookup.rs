//! `NAME::lookup` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "NAME::lookup",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Deprecated: Performs DNS query for A or PTR record corresponding to a hostname or IP address.",
            synopsis: &["NAME::lookup"],
            snippet: "Performs a DNS query, typically returning the A record for the indicated hostname, or the PTR record for the indicated IP address.",
            source: "https://clouddocs.f5.com/api/irules/NAME__lookup.html",
            examples: "",
            return_value: "",
        }),
        forms: &[
            FormSpec { kind: FormKind::Default, synopsis: "NAME::lookup" },
        ],
        side_effects: &[
            SideEffect {
                target: SideEffectTarget::DnsState,
                reads: true,
                writes: false,
                connection_side: ConnectionSide::Both,
            },
        ],
        ..CommandSpec::DEFAULT
    }
}
