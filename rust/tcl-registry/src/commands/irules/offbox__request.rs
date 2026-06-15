//! `OFFBOX::request` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "OFFBOX::request",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Performs a request with a payload to certain service.",
            synopsis: &[
                "OFFBOX::request SERVICE PAYLOAD (((cache KEY) blocking) | (blocking | (cache KEY)) | (blocking TIMEOUT))?",
            ],
            snippet: "Performs a request with a payload to certain service.",
            source: "https://clouddocs.f5.com/api/irules/OFFBOX__request.html",
            examples: "when HTTP_REQUEST {\n                 if { [HTTP::uri] eq \"login.php\" } {\n                     OFFBOX::request \"/Common/offbox::ip_reputation\" [TCP::client_addr] cache [TCP::client_addr] blocking\n                 }\n             }",
            return_value: "",
        }),
        forms: &[FormSpec {
            kind: FormKind::Default,
            synopsis: "OFFBOX::request SERVICE PAYLOAD (((cache KEY) blocking) | (blocking | (cache KEY)) | (blocking TIMEOUT))?",
        }],
        side_effects: &[SideEffect {
            target: SideEffectTarget::NetworkIo,
            reads: false,
            writes: true,
            connection_side: ConnectionSide::Both,
        }],
        ..CommandSpec::DEFAULT
    }
}
