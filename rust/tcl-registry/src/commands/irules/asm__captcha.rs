//! `ASM::captcha` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "ASM::captcha",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Responds to the client with a CAPTCHA challenge.",
            synopsis: &["ASM::captcha"],
            snippet: "Responds to the client with a CAPTCHA challenge. \n            Note although ASM will send the CAPTCHA challenge screen back to the user, the enforcement is not always done automatically. \n            To enforce the correct CAPTCHA response, the ASM::captcha_status command should be used.",
            source: "https://clouddocs.f5.com/api/irules/ASM__captcha.html",
            examples: "le counts the number of violations, and if it exceeds 3,\n            # it issues a CAPTCHA action.\n            when ASM_REQUEST_DONE {\n                if {[ASM::violation count] > 3 and [ASM::severity] eq \"Error\"} {\n                    ASM::captcha\n                }\n            }",
            return_value: "Returns a string signifying if the challenge was sent successfully: \"ok\" - CAPTCHA challenge was sent successfully \"nok asm blocked request\" - CAPTCHA challenge was not sent, because a blocking page action was performed \"nok asm uncaptcha command was raised\" - CAPTCHA challenge was not sent, because…",
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
        forms: &[
            FormSpec { kind: FormKind::Default, synopsis: "ASM::captcha" },
        ],
        ..CommandSpec::DEFAULT
    }
}
