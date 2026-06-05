//! `AUTH::authenticate` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "AUTH::authenticate",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Performs a new authentication operation.",
            synopsis: &["AUTH::authenticate AUTH_ID"],
            snippet: "Performs a new authentication operation. This command returns an error\nif attempted for a standby system or while an authentication operation\nis already in progress for this authentication session.\n\nAUTH::authenticate <authid>\n\n     * Performs a new authentication operation. This command returns an\n       error if attempted for a standby system or while an authentication\n       operation is already in progress for this authentication session.",
            source: "https://clouddocs.f5.com/api/irules/AUTH__authenticate.html",
            examples: "when HTTP_REQUEST {\n  AUTH::username_credential $auth_id [HTTP::username]\n  AUTH::password_credential $auth_id [HTTP::password]\n  AUTH::authenticate $auth_id\n  HTTP::collect\n}",
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
        forms: &[
            FormSpec { kind: FormKind::Default, synopsis: "AUTH::authenticate AUTH_ID" },
        ],
        ..CommandSpec::DEFAULT
    }
}
