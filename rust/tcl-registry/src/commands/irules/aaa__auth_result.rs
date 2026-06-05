//! `AAA::auth_result` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "AAA::auth_result",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "This command is used to check the result of an authentication request.",
            synopsis: &["AAA::auth_result AAA_REQUEST_ID"],
            snippet: "This command is used to check the result of an authentication request. It can be used to determine whether the user was successfully authenticated, or if the authentication failed or if the system encountered an error.",
            source: "https://clouddocs.f5.com/api/irules/AAA__auth_result.html",
            examples: "when HTTP_REQUEST_DATA {\n    set aaa_result [AAA::auth_result $request_id]\n    if { $aaa_result == \"INPROGRESS\" } {\n        after 200\n        continue\n    }\n\n    if { $aaa_result == \"OK\" } {\n        # request was successfull\n    } else {\n        # handle errors\n    }\n}",
            return_value: "There are 4 possible return values for this command (All STRING type): \"OK\" - User was successfully authenticated \"FAIL\" - Authentication failed \"INPROGRESS\" - the request is still in progress (asyncronous). \"ERROR\" - there was an error during the request.",
        }),
        forms: &[
            FormSpec { kind: FormKind::Default, synopsis: "AAA::auth_result AAA_REQUEST_ID" },
        ],
        side_effects: &[
            SideEffect {
                target: SideEffectTarget::ApmState,
                reads: true,
                writes: false,
                connection_side: ConnectionSide::Both,
            },
        ],
        ..CommandSpec::DEFAULT
    }
}
