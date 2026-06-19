//! `PSC::policy` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "PSC::policy",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Get/set/remove policies.",
            synopsis: &[
                "PSC::policy (POLICY_NAME)*",
                "PSC::policy 'add' POLICY_NAME",
                "PSC::policy 'remove' (POLICY_NAME)?",
            ],
            snippet: "The PSC::policy commands get/set/remove the PSC policies.",
            source: "https://clouddocs.f5.com/api/irules/PSC__policy.html",
            examples: "",
            return_value: "Return the list of PSC policies when no argument is given.",
        }),
        forms: &[FormSpec {
            kind: FormKind::Default,
            synopsis: "PSC::policy (POLICY_NAME)*",
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
