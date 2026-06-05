//! `BOTDEFENSE::cookie_status` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "BOTDEFENSE::cookie_status",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Returns the status of the Bot Defense cookie.",
            synopsis: &["BOTDEFENSE::cookie_status"],
            snippet: "Returns the status of the Bot Defense cookie that is received on the request. The returned value is one of the following strings:\n    * not_received - the cookie did not appear in the request\n    * valid - the cookie is valid and not expired\n    * invalid - the cookie cannot be parsed; this could mean that it was modified by an attacker, or that it is older than two days, or due to a configuration change\n    * expired - the cookie is valid, but is expired\n    * valid_redirect_challenge - the cookie of the redirect was validated\n    * renewal - browser challenge answer is about to expire",
            source: "https://clouddocs.f5.com/api/irules/BOTDEFENSE__cookie_status.html",
            examples: "# EXAMPLE: In case of an invalid cookie, send a message to High Speed Logging\nwhen BOTDEFENSE_REQUEST {\n    if {[BOTDEFENSE::cookie_status] eq \"invalid\"} {\n        HSL::send $hsl \"invalid botdefense cookie from IP [IP::client_addr]\"\n    }\n}",
            return_value: "A string signifying the status of the Bot Defense cookie.",
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
            FormSpec { kind: FormKind::Default, synopsis: "BOTDEFENSE::cookie_status" },
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
