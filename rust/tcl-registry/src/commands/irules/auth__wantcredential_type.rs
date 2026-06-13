//! `AUTH::wantcredential_type` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "AUTH::wantcredential_type",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Returns an authorization session authidXs credential type.",
            synopsis: &["AUTH::wantcredential_type AUTH_ID"],
            snippet: "Returns the authorization session authid’s credential type that the\nsystem last requested (when the system generated an AUTH_WANTCREDENTIAL\nevent). The value of the <authid> argument is either username,\npassword, x509, x509_issuer, or unknown, based upon the system’s\nassessment of the credential prompt string and style.\n\nAUTH::wantcredential_type <authid>\n\n     * Returns the authorization session authid’s credential type that the\n       system last requested (when the system generated an\n       AUTH_WANTCREDENTIAL event).",
            source: "https://clouddocs.f5.com/api/irules/AUTH__wantcredential_type.html",
            examples: "when AUTH_WANTCREDENTIAL {\n  HTTP::respond 401 \"WWW-Authenticate\" \"Basic realm=\\\"\\\"\"\n}",
            return_value: "",
        }),
        forms: &[
            FormSpec { kind: FormKind::Default, synopsis: "AUTH::wantcredential_type AUTH_ID" },
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
