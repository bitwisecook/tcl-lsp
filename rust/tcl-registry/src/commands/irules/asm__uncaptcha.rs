//! `ASM::uncaptcha` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "ASM::uncaptcha",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Overrides the CAPTCHA action.",
            synopsis: &["ASM::uncaptcha"],
            snippet: "Overrides the CAPTCHA action for a request mitigated during a Brute-Force attack. \n            Consequently, the request will be forwarded to the origin server. \n            If the present request was not supposed to be mitigated by CAPTCHA then the command has no effect.",
            source: "https://clouddocs.f5.com/api/irules/ASM__uncaptcha.html",
            examples: "when ASM_REQUEST_DONE {\n                set i 0\n                foreach {viol} [ASM::violation names] {\n                    if {$viol eq VIOLATION_ILLEGAL_PARAMETER} {\n                        set details [lindex [ASM::violation details] $i]\n                        set param_name [b64decode [llookup $details \"param_data.param_name\"]]\n                        #remove the bad parameter from the QS - does not work right in all cases, just for illustration!",
            return_value: "",
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
            FormSpec { kind: FormKind::Default, synopsis: "ASM::uncaptcha" },
        ],
        ..CommandSpec::DEFAULT
    }
}
