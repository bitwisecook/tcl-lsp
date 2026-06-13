//! `PSC::lease_time` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "PSC::lease_time",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Get the session lease time.",
            synopsis: &["PSC::lease_time IP_ADDR (LEASE_TIME)?"],
            snippet: "The PSC::lease_time command gets the lease time of the session.",
            source: "https://clouddocs.f5.com/api/irules/PSC__lease-time.html",
            examples: "",
            return_value: "Return the session lease time.",
        }),
        forms: &[FormSpec {
            kind: FormKind::Default,
            synopsis: "PSC::lease_time IP_ADDR (LEASE_TIME)?",
        }],
        side_effects: &[SideEffect {
            target: SideEffectTarget::NetworkIo,
            reads: true,
            writes: false,
            connection_side: ConnectionSide::Both,
        }],
        ..CommandSpec::DEFAULT
    }
}
