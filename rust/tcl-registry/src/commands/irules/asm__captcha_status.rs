//! `ASM::captcha_status` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "ASM::captcha_status",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Returns the status of the user's answer to the CAPTCHA challenge.",
            synopsis: &["ASM::captcha_status"],
            snippet: "Returns the status of the user's answer to the CAPTCHA challenge. The returned value is one of the following strings:\n            not_received - the answer to the CAPTCHA challenge did not appear in the request; this is the normal result, before the CAPTCHA challenge is sent to the client\n            correct - the answer is correct\n            incorrect - the answer is incorrect\n            empty - an empty answer was given, or if the user clicked on the CAPTCHA Refresh button",
            source: "https://clouddocs.f5.com/api/irules/ASM__captcha_status.html",
            examples: "# EXAMPLE: Send a CAPTCHA challenge on the login page, and only allow the\n            # login if the user passed the CAPTCHA challenge\n            when ASM_REQUEST_DONE {\n                if {[ASM::captcha_status] ne \"correct\"} {\n                    if {[HTTP::uri] eq \"/t/login.php\"} {\n                        set res [ASM::captcha]\n                        if {$res ne \"ok\"} {\n                            log local0. \"cannot send captcha_challenge: \\\"$res\\\"\"\n                        }",
            return_value: "Returns a string signifying the status of the CAPTCHA challenge.",
        }),
        event_requires: Some(EventRequires {
            client_side: false,
            server_side: false,
            transport: None,
            profiles: &["ASM"],
            also_in: &[],
            init_only: false,
            flow: false,
            capability: None,
        }),
        forms: &[FormSpec {
            kind: FormKind::Default,
            synopsis: "ASM::captcha_status",
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
