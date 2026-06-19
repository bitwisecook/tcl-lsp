//! `BOTDEFENSE::cs_possible` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "BOTDEFENSE::cs_possible",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Returns whether it is possible for Bot Defense to take a client-side action.",
            synopsis: &["BOTDEFENSE::cs_possible"],
            snippet: "Returns \"true\" or \"false\" based on whether it is possible to take one of the client-side actions that initiate a response (browser challenge, or CAPTCHA challenge, or device id collection) or send browser challenge in response. Certain characteristics of a request make it impossible to respond with a browser verification or CAPTCHA challenge or device id, in which case \"false\" is returned.\n\nSetting to a client-side action with BOTDEFENSE::action, while the value of BOTDEFENSE::cs_possible is \"false\", will fail.",
            source: "https://clouddocs.f5.com/api/irules/BOTDEFENSE__cs_possible.html",
            examples: "# EXAMPLE: Prevent blocking of requests that cannot be responded with a\n# client-side challenge.\nwhen BOTDEFENSE_ACTION {\n    if {    ([BOTDEFENSE::action] eq \"tcp_rst\") &&\n            (not [BOTDEFENSE::cs_possible])} {\n        BOTDEFENSE::action allow\n    }\n}",
            return_value: "Returns a boolean value (0 or 1), whether taking a client-side action is possible.",
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
            synopsis: "BOTDEFENSE::cs_possible",
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
