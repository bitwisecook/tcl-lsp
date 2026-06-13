//! `PSC::ip_address` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "PSC::ip_address",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Get/set/remove ip address(es).",
            synopsis: &["PSC::ip_address (IP_ADDR)*", "PSC::ip_address 'add' IP_ADDR", "PSC::ip_address 'remove' (IP_ADDR)?"],
            snippet: "The PSC::ip_address commands get/set/remove the IP addresses.\n\nNote:IP address used in the commands below could be in IPv4 or IPv6 format. The route domain can be specified using % as a separator, e.g. 14.15.16.17%10.",
            source: "https://clouddocs.f5.com/api/irules/PSC__ip_address.html",
            examples: "",
            return_value: "Return the list of PSC ip addresses when no argument is given.",
        }),
        forms: &[
            FormSpec { kind: FormKind::Default, synopsis: "PSC::ip_address (IP_ADDR)*" },
        ],
        side_effects: &[
            SideEffect {
                target: SideEffectTarget::NetworkIo,
                reads: true,
                writes: false,
                connection_side: ConnectionSide::Both,
            },
        ],
        ..CommandSpec::DEFAULT
    }
}
