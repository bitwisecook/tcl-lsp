//! `HTML::enable` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "HTML::enable",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Enable the processing of HTML for this transaction.",
            synopsis: &["HTML::enable"],
            snippet: "Enable the processing of HTML for this transaction.",
            source: "https://clouddocs.f5.com/api/irules/HTML__enable.html",
            examples: "when HTTP_RESPONSE {\n    if {$host == \"www.f5.com\"} {\n        HTML::enable\n    }\n    log local0. \"host: $host\"\n}",
            return_value: "empty return code.",
        }),
        ..CommandSpec::DEFAULT
    }
}
