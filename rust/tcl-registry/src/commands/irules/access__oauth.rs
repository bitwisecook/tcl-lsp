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
        ..CommandSpec::DEFAULT
    }
}
