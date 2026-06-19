//! `AAA::auth_send` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "AAA::auth_send",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "This command is used to send user authentication information to IVS(internal virtual server).",
            synopsis: &["AAA::auth_send VIRTUAL_SERVER USERNAME (PASSWORD)?"],
            snippet: "This command is used to send user authentication information to IVS(internal virtual server).",
            source: "https://clouddocs.f5.com/api/irules/AAA__auth_send.html",
            examples: "when HTTP_REQUEST_DATA {\n    set request_id [AAA::auth_send $internal_radius_aaa_vip $username $password]\n\n    set aaa_result [AAA::auth_result $request_id]\n    if { $aaa_result == \"OK\" } {\n        # request was successfull\n    } else {\n        # handle errors\n    }\n}",
            return_value: "request_id - the id of the current connection that can be used to check the status later with AAA::auth_result command",
        }),
        forms: &[FormSpec {
            kind: FormKind::Default,
            synopsis: "AAA::auth_send VIRTUAL_SERVER USERNAME (PASSWORD)?",
        }],
        side_effects: &[SideEffect {
            target: SideEffectTarget::ApmState,
            reads: false,
            writes: true,
            connection_side: ConnectionSide::Both,
        }],
        ..CommandSpec::DEFAULT
    }
}
