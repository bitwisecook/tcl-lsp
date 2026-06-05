//! `BOTDEFENSE::captcha_age` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "BOTDEFENSE::captcha_age",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Returns the age of the CAPTCHA challenge in seconds.",
            synopsis: &["BOTDEFENSE::captcha_age"],
            snippet: "Returns the age of the CAPTCHA challenge in seconds. This is only relevant if the value of BOTDEFENSE::captcha_status is \"correct\", \"renewal\" or \"expired\"; otherwise, -1 is returned.",
            source: "https://clouddocs.f5.com/api/irules/BOTDEFENSE__captcha_age.html",
            examples: "# EXAMPLE: Send CAPTCHA challenge and validate the response, but to avoid\n# blocking requests to which CAPTCHA challenge cannot be sent (non-HTML pages),\n# send the CAPTCHA challenge on HTML pages after 30 seconds of aging, which is\n# before the expiration of the answer.\nwhen BOTDEFENSE_ACTION {\n    if {[BOTDEFENSE::action] eq \"allow\"} {\n        if {[BOTDEFENSE::captcha_status] eq \"correct\"} {\n            if {    ([BOTDEFENSE::cs_allowed]) &&",
            return_value: "Returns the age of the CAPTCHA challenge in seconds, or -1 if not applicable.",
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
            FormSpec { kind: FormKind::Default, synopsis: "BOTDEFENSE::captcha_age" },
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
