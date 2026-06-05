//! `SSL::authenticate` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "SSL::authenticate",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Overrides the current setting for authentication frequency or for the maximum depth of certificate chain traversal.",
            synopsis: &["SSL::authenticate (once | always | (depth DEPTH))"],
            snippet: "Overrides the current setting for authentication frequency or for the maximum depth of certificate chain traversal.\n\nSSL::authenticate <\"once\" | \"always\">\n    Valid in a client-side context only, this command overrides the client-side SSL connection's current setting regarding authentication frequency.\n\nSSL::authenticate depth <number>\n    When the system evaluates the command in a client-side context, the command overrides the client-side SSL connection's current setting regarding maximum certificate chain traversal depth.",
            source: "https://clouddocs.f5.com/api/irules/SSL__authenticate.html",
            examples: "when CLIENT_ACCEPTED {\n    set session_flag 0\n}",
            return_value: "",
        }),
        forms: &[
            FormSpec { kind: FormKind::Default, synopsis: "SSL::authenticate <once | always | depth <number>>" },
        ],
        ..CommandSpec::DEFAULT
    }
}
