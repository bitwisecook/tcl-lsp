//! `HTTP::username` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "HTTP::username",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Returns the username part of HTTP basic authentication.",
            synopsis: &["HTTP::username"],
            snippet: "Returns the username part of HTTP basic authentication.\nAs described in RFC2617 the username and password in basic\nauthentication is sent by the client in the Authorization header. The\nclient base64 encodes the username and password in the format of:\nAuthorization: Basic base64encoding(username:password)\nThe HTTP::username command parses and base64 decodes the username.\nThe HTTP::password command parses and base64 decodes the password.",
            source: "https://clouddocs.f5.com/api/irules/HTTP__username.html",
            examples: "when CLIENT_ACCEPTED {\n  set auth_sid [AUTH::start pam default_radius]\n}",
            return_value: "Returns the username part of HTTP basic authentication",
        }),
        event_requires: Some(EventRequires {
            client_side: false,
            server_side: false,
            transport: Some("tcp"),
            profiles: &["FASTHTTP", "HTTP"],
            also_in: &[],
            init_only: false,
            flow: false,
            capability: None,
        }),
        ..CommandSpec::DEFAULT
    }
}
