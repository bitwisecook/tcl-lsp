//! `BOTDEFENSE::reason` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "BOTDEFENSE::reason",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Returns the reason for the Bot Defense action.",
            synopsis: &["BOTDEFENSE::reason"],
            snippet: "Returns the reason that lead Bot Defense to decide on the action to be taken (the action that is specified in BOTDEFENSE::action).",
            source: "https://clouddocs.f5.com/api/irules/BOTDEFENSE__reason.html",
            examples: "# EXAMPLE: Send the Bot Defense action and reason to High Speed Logging\nwhen BOTDEFENSE_ACTION {\n    HSL::send $hsl \"action [BOTDEFENSE::action] reason \\\"[BOTDEFENSE::reason]\\\"\"\n}",
            return_value: "Returns a string signifying the reason for the Bot Defense action.",
        }),
        event_requires: Some(EventRequires {
            client_side: false,
            server_side: false,
            transport: None,
            profiles: &["BOTDEFENSE"],
            also_in: &[],
            init_only: false,
            flow: false,
            capability: None,
        }),
        forms: &[
            FormSpec { kind: FormKind::Default, synopsis: "BOTDEFENSE::reason" },
        ],
        side_effects: &[
            SideEffect {
                target: SideEffectTarget::AsmState,
                reads: true,
                writes: false,
                connection_side: ConnectionSide::Client,
            },
        ],
        ..CommandSpec::DEFAULT
    }
}
