//! `AUTH::status` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "AUTH::status",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Returns authentication status.",
            synopsis: &["AUTH::status (AUTH_ID)?"],
            snippet: "Returns authentication status. The returned status is a value of 0, 1,\n-1, or 2, corresponding to success, failure, error, or not-authed,\nbased on the result of the most recent authorization that the system\nperformed for the specified authorization session .\nIn the case of a not-authed result, the authentication process desires\na credential not yet provided. Specifics of the requested credential\ncan be determined using the AUTH::wantcredential_ commands. The\nauthentication process could be continued using\nAUTH::authenticate_continue*.",
            source: "https://clouddocs.f5.com/api/irules/AUTH__status.html",
            examples: "when HTTP_RESPONSE {\n  set authStatus [AUTH::status $authSessionId]\n}",
            return_value: "",
        }),
        forms: &[
            FormSpec { kind: FormKind::Default, synopsis: "AUTH::status (AUTH_ID)?" },
        ],
        ..CommandSpec::DEFAULT
    }
}
