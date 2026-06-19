//! `BOTDEFENSE::support_id` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "BOTDEFENSE::support_id",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Returns the support ID of the request.",
            synopsis: &["BOTDEFENSE::support_id"],
            snippet: "Returns a number, representing the support ID of the request.",
            source: "https://clouddocs.f5.com/api/irules/BOTDEFENSE__support_id.html",
            examples: "# EXAMPLE: Log the support ID of the request.\nwhen BOTDEFENSE_REQUEST {\n    set log \"support id is\"\n    append log \" [BOTDEFENSE::support_id]\"\n    HSL::send $hsl $log\n}",
            return_value: "Returns the support ID of the reuqest.",
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
            synopsis: "BOTDEFENSE::support_id",
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
