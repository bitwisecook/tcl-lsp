//! `ACCESS::oauth` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "ACCESS::oauth",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "OAuth related ACCESS iRule",
            synopsis: &["ACCESS::oauth sign ((-payload VALUE) (-key JWK_OBJECT)"],
            snippet: "OAuth related ACCESS iRule\n\nACCESS::oauth sign [ -header <raw-data> ] -payload <raw-data> -key <JWK object>\n                   [ -alg <signing algorithm> ] [ -ignore-cert-expiry ]\n\n     * Returns a JSON Web Signature token based on provided payload and signed\n       with provided JWK object. When the specified JWK object does not specify\n       a JWS signing algorithm, an additional signing algorithm is required\n       and must be provided with the -alg option.",
            source: "https://clouddocs.f5.com/api/irules/ACCESS__oauth.html",
            examples: "when ACCESS_SESSION_CLOSED {\n    call delete_jws_cache\n}",
            return_value: "JSON Web Signature string.",
        }),
        forms: &[
            FormSpec { kind: FormKind::Default, synopsis: "ACCESS::oauth sign ((-payload VALUE) (-key JWK_OBJECT)" },
        ],
        options: &[
            OptionSpec { name: "-header", takes_value: true, value_hint: "RAW_DATA", detail: "Raw data for JOSE header section.", dialects: None },
            OptionSpec { name: "-payload", takes_value: true, value_hint: "RAW_DATA", detail: "Raw data for JWS payload.", dialects: None },
            OptionSpec { name: "-key", takes_value: true, value_hint: "JWK_OBJECT", detail: "JWK object for signing.", dialects: None },
            OptionSpec { name: "-alg", takes_value: true, value_hint: "ALG", detail: "Signing algorithm.", dialects: None },
            OptionSpec { name: "-ignore-cert-expiry", takes_value: false, value_hint: "", detail: "Allow expired certificate.", dialects: None },
        ],
        ..CommandSpec::DEFAULT
    }
}
