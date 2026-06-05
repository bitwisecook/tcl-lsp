//! `HTTP2::disable` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "HTTP2::disable",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Changes the HTTP2 filter from full parsing to passthrough mode.",
            synopsis: &["HTTP2::disable ('clientside')? ('serverside')? ('discard')?"],
            snippet: "Changes the HTTP2 filter from full parsing to passthrough mode. This\ncommand is useful when using an HTTP2 profile with an application that\nproxies data over HTTP.",
            source: "https://clouddocs.f5.com/api/irules/HTTP2__disable.html",
            examples: "when HTTP_REQUEST {\n    if { [HTTP::uri] contains \"http1_backend\"} {\n        HTTP2::disable serverside\n    }\n}",
            return_value: "",
        }),
        ..CommandSpec::DEFAULT
    }
}
