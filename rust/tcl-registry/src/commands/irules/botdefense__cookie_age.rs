//! `BOTDEFENSE::cookie_age` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "BOTDEFENSE::cookie_age",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Returns the age of the Bot Defense cookie in seconds.",
            synopsis: &["BOTDEFENSE::cookie_age"],
            snippet: "Returns the age of the Bot Defense browser cookie in seconds. This is only relevant if the value of BOTDEFENSE::cookie_status is either \"valid\", \"expired\" or \"renewal\"; otherwise, -1 is returned.\n\nNote that In the previous version the returned status referred to both device_id and browser challenge, but now it only returns the age of the browser challenge.",
            source: "https://clouddocs.f5.com/api/irules/BOTDEFENSE__cookie_age.html",
            examples: "# EXAMPLE: In case of an expired cookie, log the age of the cookie\nwhen BOTDEFENSE_REQUEST {\n    if {[BOTDEFENSE::cookie_status] eq \"expired\"} {\n        set log \"expired botdefense cookie (from [BOTDEFENSE::cookie_age]\"\n        append log \" seconds ago) from IP [IP::client_addr]\"\n        HSL::send $hsl $log\n    }\n}",
            return_value: "Returns the age of the Bot Defense cookie in seconds, or -1 if not applicable.",
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
            synopsis: "BOTDEFENSE::cookie_age",
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
