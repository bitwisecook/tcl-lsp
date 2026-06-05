//! `AUTH::password_credential` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "AUTH::password_credential",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Sets the password credential to the specified string for a future AUTH::authenticate call.",
            synopsis: &["AUTH::password_credential AUTH_ID PASSWORD_CREDENTIAL"],
            snippet: "Sets the password credential to the specified string for a future\nAUTH::authenticate call. This command returns an error if\nattempted for a standby system.\n\nAUTH::password_credential authid <string>\n\n     * Sets the password credential to the specified string for a future\n       AUTH::authenticate call.",
            source: "https://clouddocs.f5.com/api/irules/AUTH__password_credential.html",
            examples: "when HTTP_REQUEST {\n  AUTH::password_credential $auth_id [HTTP::password]\n}",
            return_value: "",
        }),
        event_requires: Some(EventRequires {
            client_side: false,
            server_side: false,
            transport: None,
            profiles: &["HTTP"],
            also_in: &[],
            init_only: false,
            flow: false,
            capability: None,
        }),
        ..CommandSpec::DEFAULT
    }
}
