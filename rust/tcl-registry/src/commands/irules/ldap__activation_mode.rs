//! `LDAP::activation_mode` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "LDAP::activation_mode",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Set the activation mode.",
            synopsis: &["LDAP::activation_mode (none | allow | require)"],
            snippet: "Sets the activation mode to none (it will never activate), allow (if the SMTP client sends STARTTLS, we will activate TLS), or require (all commands will be rejected until STARTTLS is received).",
            source: "https://clouddocs.f5.com/api/irules/LDAP__activation_mode.html",
            examples: "when CLIENT_ACCEPTED {\n                if { !([IP::addr [IP::client_addr] ne 10.0.0.0/8) } {\n                    LDAP::activation_mode require\n                }\n            }",
            return_value: "",
        }),
        event_requires: Some(EventRequires {
            client_side: false,
            server_side: false,
            transport: None,
            profiles: &[],
            also_in: &["CLIENT_ACCEPTED", "SERVER_CONNECTED"],
            init_only: false,
            flow: false,
            capability: None,
        }),
        forms: &[
            FormSpec { kind: FormKind::Default, synopsis: "LDAP::activation_mode (none | allow | require)" },
        ],
        ..CommandSpec::DEFAULT
    }
}
