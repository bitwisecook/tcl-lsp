//! `BOTDEFENSE::previous_request_age` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "BOTDEFENSE::previous_request_age",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Returns the number of seconds that passed since the previous request was received.",
            synopsis: &["BOTDEFENSE::previous_request_age"],
            snippet: "Returns the number of seconds that passed since the previous request was received; this is applicable if the current request is a follow-up to a challenge. Otherwise, \"0\" is returned",
            source: "https://clouddocs.f5.com/api/irules/BOTDEFENSE__previous_request_age.html",
            examples: "# EXAMPLE: Log the previous request age.\nwhen BOTDEFENSE_REQUEST {\n    if {[BOTDEFENSE::previous_request_age] != 0} {\n        set log \"botdefense previous request age is\"\n        append log \" [BOTDEFENSE::previous_request_age]\"\n        HSL::send $hsl $log\n    }\n}",
            return_value: "Returns the number of seconds that passed since the previous request was received or 0 if not applicable.",
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
        forms: &[FormSpec {
            kind: FormKind::Default,
            synopsis: "BOTDEFENSE::previous_request_age",
        }],
        side_effects: &[SideEffect {
            target: SideEffectTarget::AsmState,
            reads: true,
            writes: false,
            connection_side: ConnectionSide::Client,
        }],
        ..CommandSpec::DEFAULT
    }
}
