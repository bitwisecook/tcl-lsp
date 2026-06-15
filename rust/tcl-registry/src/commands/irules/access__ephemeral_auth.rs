//! `ACCESS::ephemeral-auth` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "ACCESS::ephemeral-auth",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Ephemeral auth related iRule",
            synopsis: &[
                "ACCESS::ephemeral-auth create ('-user' USER) ('-auth_cfg' AUTH_CONFIG)? ('-sid' SESSION_ID)?",
                "ACCESS::ephemeral-auth verify ('-user' USER) ('-password' PASSWORD) ('-protocol' EPHEMERAL_AUTH_PROTOCOL)",
            ],
            snippet: "Ephemeral auth related iRule\n\nThis command can be used either to create or verify a temporary password for ephemeral authentication.\n\nACCESS::ephemeral-auth create [] will create a temporary password and return its value. When auth_cfg is not given, it will use the one deduced from access-config that is associated with the virtual server.  When sid is not given, it will use the one retrieved from the current access environment.\n\nACCESS::ephemeral-auth verify [] will verify the user credentials and return the session id that was used to generate temporary password.",
            source: "https://clouddocs.f5.com/api/irules/ACCESS__ephemeral-auth.html",
            examples: "when HTTP_REQUEST {\n    if { [ HTTP::path ] starts_with \"/test1\" } {\n        call ephemeral_auth_test1\n        HTTP::respond 200 -content \"<html>test1</html>\\n\"\n    }\n}",
            return_value: "ACCESS::ephemeral-auth create [] will return the generated temporary password. ACCESS::ephemeral-auth verify [] will return the session id.",
        }),
        forms: &[FormSpec {
            kind: FormKind::Default,
            synopsis: "ACCESS::ephemeral-auth create ('-user' USER) ('-auth_cfg' AUTH_CONFIG)? ('-sid' SESSION_ID)?",
        }],
        options: &[
            OptionSpec {
                name: "-user",
                takes_value: true,
                value_hint: "",
                detail: "Option -user.",
                dialects: None,
            },
            OptionSpec {
                name: "-auth_cfg",
                takes_value: true,
                value_hint: "",
                detail: "Option -auth_cfg.",
                dialects: None,
            },
            OptionSpec {
                name: "-sid",
                takes_value: true,
                value_hint: "",
                detail: "Option -sid.",
                dialects: None,
            },
            OptionSpec {
                name: "-password",
                takes_value: true,
                value_hint: "",
                detail: "Option -password.",
                dialects: None,
            },
            OptionSpec {
                name: "-protocol",
                takes_value: true,
                value_hint: "",
                detail: "Option -protocol.",
                dialects: None,
            },
        ],
        side_effects: &[SideEffect {
            target: SideEffectTarget::ApmState,
            reads: false,
            writes: true,
            connection_side: ConnectionSide::Both,
        }],
        ..CommandSpec::DEFAULT
    }
}
