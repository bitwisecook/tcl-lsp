//! `STREAM::expression` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "STREAM::expression",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Replaces the expression in a Stream profile with another expression.",
            synopsis: &["STREAM::expression EXPRESSION"],
            snippet: "Replaces the stream expression in the Stream profile with the specified\nvalue. The syntax is identical to the profile syntax.\nNote that this change affects this connection only and is sticky\nfor the duration of the connection.",
            source: "https://clouddocs.f5.com/api/irules/STREAM__expression.html",
            examples: "when HTTP_REQUEST {\n   # Disable the stream filter for all requests\n   STREAM::disable\n}",
            return_value: "",
        }),
        forms: &[
            FormSpec { kind: FormKind::Default, synopsis: "STREAM::expression EXPRESSION" },
        ],
        ..CommandSpec::DEFAULT
    }
}
